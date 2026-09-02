use std::env;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

const DEFAULT_PORT: u16 = 18_081;
const DEFAULT_PAYLOAD: &str = "upstream-ok";
const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = parse_port()?;
    let payload = env::var("UPSTREAM_PAYLOAD").unwrap_or_else(|_| DEFAULT_PAYLOAD.to_string());
    let response = Arc::new(build_response(payload.as_bytes()));
    let listener = TcpListener::bind(("127.0.0.1", port))?;

    for stream in listener.incoming() {
        let stream = stream?;
        let response = Arc::clone(&response);
        drop(thread::spawn(move || {
            if let Err(error) = serve_connection(stream, &response) {
                eprintln!("load origin connection failed: {error}");
            }
        }));
    }

    Ok(())
}

fn parse_port() -> Result<u16, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_PORT") {
        Ok(value) => {
            let port = value.parse::<u16>()?;
            if port == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "UPSTREAM_PORT must be non-zero",
                )
                .into());
            }
            Ok(port)
        }
        Err(env::VarError::NotPresent) => Ok(DEFAULT_PORT),
        Err(error) => Err(error.into()),
    }
}

fn build_response(payload: &[u8]) -> Vec<u8> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        payload.len()
    );
    let mut response = Vec::with_capacity(header.len() + payload.len());
    response.extend_from_slice(header.as_bytes());
    response.extend_from_slice(payload);
    response
}

fn serve_connection(mut stream: TcpStream, response: &[u8]) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut buffered = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];

    loop {
        while let Some(header_end) = find_header_end(&buffered) {
            drop(buffered.drain(..header_end));
            stream.write_all(response)?;
        }

        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(());
        }
        buffered.extend_from_slice(&chunk[..read]);
        if buffered.len() > MAX_REQUEST_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "load-origin request header exceeded fixture bound",
            ));
        }
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}
