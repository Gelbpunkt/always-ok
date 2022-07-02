#![no_main]

use std::{
    io::{BufReader, Read, Result, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::available_parallelism,
};

mod threadpool;

const PORT: u16 = 8080;
const BUFFER_SIZE: usize = 8096;

const BAD_REQUEST: &[u8] = b"HTTP/1.1 400 Bad Request\r\n\r\n";
const OK: &[u8] = b"HTTP/1.1 200 OK\r\n";

#[no_mangle]
pub fn main(_argc: i32, _argv: *const *const u8) {
    let pool = threadpool::ThreadPool::new(available_parallelism().unwrap());

    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let listener = TcpListener::bind(addr).unwrap();

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            pool.spawn(move || handle(stream));
        }
    }
}

fn handle(stream: TcpStream) -> Result<()> {
    let mut buffer = BufReader::new(stream);

    let mut buf = vec![0; BUFFER_SIZE];
    let mut bytes_read = 0;

    loop {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);

        let read = buffer.read(&mut buf[bytes_read..])?;
        bytes_read += read;

        if let Ok(res) = req.parse(&buf) {
            if res.is_complete() {
                buffer.into_inner().write_all(OK)?;
                break;
            } else {
                continue;
            }
        } else {
            buffer.into_inner().write_all(BAD_REQUEST)?;
            break;
        };
    }

    Ok(())
}
