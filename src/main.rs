#![no_main]

use std::{
    io::{Read, Result, Write},
    net::{SocketAddr, TcpListener},
    thread::{available_parallelism, spawn},
};

const PORT: u16 = 8080;
const BUFFER_SIZE: usize = 8096;

const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\n\r\n";
const OK: &[u8] = b"HTTP/1.1 200 OK\r\n";

#[no_mangle]
pub fn main(_argc: i32, _argv: *const *const u8) {
    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let listener = TcpListener::bind(addr).unwrap();

    let thread_count = available_parallelism().unwrap();
    let mut threads = Vec::with_capacity(thread_count.get());

    for _ in 0..thread_count.get() {
        let listener = listener.try_clone().unwrap();
        threads.push(spawn(move || listen_task(listener)))
    }

    for thread in threads {
        thread.join().unwrap().unwrap();
    }
}

fn listen_task(listener: TcpListener) -> Result<()> {
    let mut buf = vec![0; BUFFER_SIZE];
    let mut bytes_read = 0;

    for mut stream in listener.incoming().flatten() {
        loop {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut req = httparse::Request::new(&mut headers);

            bytes_read += stream.read(&mut buf[bytes_read..])?;

            if let Ok(res) = req.parse(&buf[..bytes_read]) {
                if res.is_complete() {
                    stream.write_all(OK)?;
                    break;
                }
            } else {
                stream.write_all(BAD_REQUEST)?;
                break;
            };
        }

        // Since we reuse buf, we do not care about the contents
        // left over from previous reads, all we want to do is
        // start writing again!
        bytes_read = 0;
    }

    Ok(())
}
