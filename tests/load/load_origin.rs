use std::env;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const DEFAULT_PORT: u16 = 18_081;
const DEFAULT_PAYLOAD: &str = "upstream-ok";
const DEFAULT_WORKERS: usize = 32;
const MAX_WORKERS: usize = 256;
const DEFAULT_RESPONSE_DELAY_MS: u64 = 0;
const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionMode {
    KeepAlive,
    Close,
}

impl ConnectionMode {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        match env::var("UPSTREAM_CONNECTION_MODE") {
            Ok(value) if value == "keep-alive" => Ok(Self::KeepAlive),
            Ok(value) if value == "close" => Ok(Self::Close),
            Ok(value) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "UPSTREAM_CONNECTION_MODE must be 'keep-alive' or 'close', received {value:?}"
                ),
            )
            .into()),
            Err(env::VarError::NotPresent) => Ok(Self::KeepAlive),
            Err(error) => Err(error.into()),
        }
    }

    fn response_header_value(self) -> &'static str {
        match self {
            Self::KeepAlive => "keep-alive",
            Self::Close => "close",
        }
    }
}

#[derive(Debug)]
struct OriginConfig {
    port: u16,
    payload: String,
    workers: usize,
    response_delay: Duration,
    connection_mode: ConnectionMode,
}

impl OriginConfig {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let workers = parse_workers()?;
        Ok(Self {
            port: parse_port()?,
            payload: match env::var("UPSTREAM_PAYLOAD") {
                Ok(value) => value,
                Err(env::VarError::NotPresent) => DEFAULT_PAYLOAD.to_string(),
                Err(error) => return Err(error.into()),
            },
            workers,
            response_delay: Duration::from_millis(parse_response_delay_ms()?),
            connection_mode: ConnectionMode::from_env()?,
        })
    }

    fn queue_capacity(&self) -> usize {
        self.workers.saturating_mul(2).max(1)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OriginConfig::from_env()?;
    let queue_capacity = config.queue_capacity();
    let response = Arc::new(build_response(
        config.payload.as_bytes(),
        config.connection_mode,
    ));
    let listener = TcpListener::bind(("127.0.0.1", config.port))?;
    let (sender, receiver) = mpsc::sync_channel::<TcpStream>(queue_capacity);
    let receiver = Arc::new(Mutex::new(receiver));

    for worker_id in 0..config.workers {
        let receiver = Arc::clone(&receiver);
        let response = Arc::clone(&response);
        let response_delay = config.response_delay;
        let connection_mode = config.connection_mode;
        let worker = thread::Builder::new()
            .name(format!("load-origin-{worker_id}"))
            .spawn(move || worker_loop(receiver, response, response_delay, connection_mode))?;
        drop(worker);
    }

    eprintln!(
        "load origin ready: port={} workers={} queue_capacity={} connection_mode={:?} response_delay_ms={}",
        config.port,
        config.workers,
        queue_capacity,
        config.connection_mode,
        config.response_delay.as_millis()
    );

    for stream in listener.incoming() {
        let stream = stream?;
        sender.send(stream).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "load-origin worker queue disconnected",
            )
        })?;
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

fn parse_workers() -> Result<usize, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_WORKERS") {
        Ok(value) => {
            let workers = value.parse::<usize>()?;
            if workers == 0 || workers > MAX_WORKERS {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("UPSTREAM_WORKERS must be in 1..={MAX_WORKERS}"),
                )
                .into());
            }
            Ok(workers)
        }
        Err(env::VarError::NotPresent) => Ok(DEFAULT_WORKERS),
        Err(error) => Err(error.into()),
    }
}

fn parse_response_delay_ms() -> Result<u64, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_RESPONSE_DELAY_MS") {
        Ok(value) => Ok(value.parse::<u64>()?),
        Err(env::VarError::NotPresent) => Ok(DEFAULT_RESPONSE_DELAY_MS),
        Err(error) => Err(error.into()),
    }
}

fn worker_loop(
    receiver: Arc<Mutex<mpsc::Receiver<TcpStream>>>,
    response: Arc<Vec<u8>>,
    response_delay: Duration,
    connection_mode: ConnectionMode,
) {
    loop {
        let stream = {
            let receiver = match receiver.lock() {
                Ok(receiver) => receiver,
                Err(_) => return,
            };
            match receiver.recv() {
                Ok(stream) => stream,
                Err(_) => return,
            }
        };

        if let Err(error) = serve_connection(
            stream,
            response.as_slice(),
            response_delay,
            connection_mode,
        ) {
            eprintln!("load origin connection failed: {error}");
        }
    }
}

fn build_response(payload: &[u8], connection_mode: ConnectionMode) -> Vec<u8> {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n",
        payload.len(),
        connection_mode.response_header_value()
    );
    let mut response = Vec::with_capacity(header.len() + payload.len());
    response.extend_from_slice(header.as_bytes());
    response.extend_from_slice(payload);
    response
}

fn serve_connection(
    mut stream: TcpStream,
    response: &[u8],
    response_delay: Duration,
    connection_mode: ConnectionMode,
) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut buffered = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];

    loop {
        while let Some(header_end) = find_header_end(&buffered) {
            drop(buffered.drain(..header_end));
            if !response_delay.is_zero() {
                thread::sleep(response_delay);
            }
            stream.write_all(response)?;
            if connection_mode == ConnectionMode::Close {
                return Ok(());
            }
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

#[cfg(test)]
mod tests {
    use super::{build_response, find_header_end, ConnectionMode};

    #[test]
    fn response_framing_matches_connection_mode() {
        let keep_alive = build_response(b"ok", ConnectionMode::KeepAlive);
        let close = build_response(b"ok", ConnectionMode::Close);

        assert!(keep_alive
            .windows(b"Connection: keep-alive".len())
            .any(|window| window == b"Connection: keep-alive"));
        assert!(close
            .windows(b"Connection: close".len())
            .any(|window| window == b"Connection: close"));
    }

    #[test]
    fn header_boundary_includes_the_terminal_separator() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: test\r\n\r\nbody"), Some(30));
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: test\r\n"), None);
    }
}
