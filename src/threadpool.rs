use std::{
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{channel, sync_channel, Receiver, Sender, SyncSender},
        Arc,
    },
    thread::{spawn, JoinHandle as ThreadJoinHandle},
};

enum Message<T> {
    Work {
        task: Box<dyn FnOnce() -> T + Send + 'static>,
        return_value: SyncSender<T>,
    },
    Terminate,
}

pub struct ThreadPool<T> {
    threads: Vec<Thread<T>>,
    next_round_robin: AtomicUsize,
}

struct Thread<T> {
    queue: Sender<Message<T>>,
    is_busy: Arc<AtomicBool>,
    handle: ThreadJoinHandle<()>,
}

fn worker<T>(rx: Receiver<Message<T>>, is_busy: Arc<AtomicBool>) {
    while let Ok(job) = rx.recv() {
        match job {
            Message::Work { task, return_value } => {
                is_busy.store(true, Ordering::Relaxed);
                let r: T = task();
                let _ = return_value.send(r);
                is_busy.store(false, Ordering::Relaxed);
            }
            Message::Terminate => break,
        }
    }
}

pub struct JoinHandle<T>(Receiver<T>);

impl<T> JoinHandle<T> {
    pub fn join(self) -> Option<T> {
        self.0.recv().ok()
    }
}

impl<T> ThreadPool<T>
where
    T: Send + 'static,
{
    pub fn new(thread_count: NonZeroUsize) -> Self {
        let mut threads = Vec::with_capacity(thread_count.get());
        let next_round_robin = AtomicUsize::new(0);

        for _ in 0..thread_count.get() {
            let (tx, rx) = channel();
            let is_busy = Arc::new(AtomicBool::new(false));
            let is_busy_clone = is_busy.clone();
            let handle = spawn(move || worker(rx, is_busy_clone));

            let thread = Thread {
                queue: tx,
                is_busy,
                handle,
            };

            threads.push(thread);
        }

        Self {
            threads,
            next_round_robin,
        }
    }

    pub fn spawn<F>(&self, f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = sync_channel(1);
        let msg = Message::Work {
            task: Box::new(f),
            return_value: tx,
        };
        let handle = JoinHandle(rx);

        // Determine a thread to run this on.

        // If any is available immediately, prefer that.
        for thread in self.threads.iter() {
            if !thread.is_busy.load(Ordering::Relaxed) {
                let _ = thread.queue.send(msg);
                return handle;
            }
        }

        // Otherwise, send it to the thread that is next in the round robin.
        let thread_count = self.threads.len();

        let thread_idx = self
            .next_round_robin
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |last_thread| {
                if last_thread == thread_count - 1 {
                    Some(0)
                } else {
                    Some(last_thread + 1)
                }
            })
            .unwrap();

        let _ = self.threads[thread_idx].queue.send(msg);

        handle
    }
}

impl<T> Drop for ThreadPool<T> {
    fn drop(&mut self) {
        for thread in self.threads.iter() {
            let _ = thread.queue.send(Message::Terminate);
        }

        while let Some(thread) = self.threads.pop() {
            thread.handle.join().unwrap();
        }
    }
}
