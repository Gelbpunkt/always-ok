#![no_main]

use std::{
    io::{Read, Result, Write},
    net::{SocketAddr, TcpListener, TcpStream},
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
        thread.join().unwrap();
    }
}

fn listen_task(listener: TcpListener) {
    for stream in listener.incoming().flatten() {
        let _ = handle(stream);
    }
}

fn handle(mut stream: TcpStream) -> Result<()> {
    let mut buf = vec![0; BUFFER_SIZE];
    let mut bytes_read = 0;

    loop {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);

        let read = stream.read(&mut buf[bytes_read..])?;
        bytes_read += read;

        if let Ok(res) = req.parse(&buf) {
            if res.is_complete() {
                stream.write_all(OK)?;
                break;
            } else {
                continue;
            }
        } else {
            stream.write_all(BAD_REQUEST)?;
            break;
        };
    }

    Ok(())
}
