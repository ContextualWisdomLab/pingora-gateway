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

/// Controls whether the synthetic origin exposes connection reuse or forces
/// each request to pay connection churn in the measured gateway round trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionMode {
    KeepAlive,
    Close,
}

impl ConnectionMode {
    /// Parses an optional startup value without touching process-global state so
    /// invalid-mode acceptance can be tested deterministically.
    fn parse(value: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        match value {
            Some("keep-alive") => Ok(Self::KeepAlive),
            Some("close") => Ok(Self::Close),
            Some(value) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "UPSTREAM_CONNECTION_MODE must be 'keep-alive' or 'close', received {value:?}"
                ),
            )
            .into()),
            None => Ok(Self::KeepAlive),
        }
    }

    /// Reads the fixture mode once at startup and delegates validation to the
    /// pure parser used by the direct rustc test contract.
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        match env::var("UPSTREAM_CONNECTION_MODE") {
            Ok(value) => Self::parse(Some(value.as_str())),
            Err(env::VarError::NotPresent) => Self::parse(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Returns the wire value paired with the selected connection behavior.
    fn response_header_value(self) -> &'static str {
        match self {
            Self::KeepAlive => "keep-alive",
            Self::Close => "close",
        }
    }
}

/// Startup-only controls for the bounded loopback origin used by load evidence.
#[derive(Debug)]
struct OriginConfig {
    port: u16,
    payload: String,
    workers: usize,
    response_delay: Duration,
    connection_mode: ConnectionMode,
}

impl OriginConfig {
    /// Loads and validates all capacity controls before binding the listener so
    /// invalid evidence configuration cannot partially activate the fixture.
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

    /// Keeps accepted-but-unassigned sockets bounded relative to worker
    /// capacity instead of recreating an effectively unbounded origin.
    fn queue_capacity(&self) -> usize {
        self.workers.saturating_mul(2).max(1)
    }
}

/// Runs the synthetic origin with a fixed worker budget and bounded accept
/// queue; it is test tooling and never part of the gateway production binary.
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

/// Parses a non-zero loopback port from an optional startup value. Keeping the
/// validation pure lets tests prove rejection without racing on environment vars.
fn parse_port_value(value: Option<&str>) -> Result<u16, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Ok(DEFAULT_PORT);
    };
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

/// Reads the loopback port once at startup and applies the same pure validation
/// used by the direct fixture tests.
fn parse_port() -> Result<u16, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_PORT") {
        Ok(value) => parse_port_value(Some(value.as_str())),
        Err(env::VarError::NotPresent) => parse_port_value(None),
        Err(error) => Err(error.into()),
    }
}

/// Parses the finite worker budget from an optional value and caps it before any
/// listener is bound or worker thread is created.
fn parse_workers_value(value: Option<&str>) -> Result<usize, Box<dyn std::error::Error>> {
    let Some(value) = value else {
        return Ok(DEFAULT_WORKERS);
    };
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

/// Reads the worker budget once at startup and delegates to the deterministic
/// parser used by the fixture's direct unit contract.
fn parse_workers() -> Result<usize, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_WORKERS") {
        Ok(value) => parse_workers_value(Some(value.as_str())),
        Err(env::VarError::NotPresent) => parse_workers_value(None),
        Err(error) => Err(error.into()),
    }
}

/// Parses the startup-only service delay without global environment mutation so
/// malformed values can be rejected by deterministic tests.
fn parse_response_delay_ms_value(
    value: Option<&str>,
) -> Result<u64, Box<dyn std::error::Error>> {
    match value {
        Some(value) => Ok(value.parse::<u64>()?),
        None => Ok(DEFAULT_RESPONSE_DELAY_MS),
    }
}

/// Reads the startup-only service delay used to make finite origin capacity
/// observable without injecting delay into the gateway itself.
fn parse_response_delay_ms() -> Result<u64, Box<dyn std::error::Error>> {
    match env::var("UPSTREAM_RESPONSE_DELAY_MS") {
        Ok(value) => parse_response_delay_ms_value(Some(value.as_str())),
        Err(env::VarError::NotPresent) => parse_response_delay_ms_value(None),
        Err(error) => Err(error.into()),
    }
}

/// Receives one admitted socket at a time for a worker, then releases the queue
/// lock before serving so the configured worker count can run concurrently.
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

/// Prebuilds an exact Content-Length-framed response so per-request fixture work
/// does not dominate the gateway latency signal.
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

/// Serves complete HTTP/1 request headers only, bounding buffered header bytes
/// and honoring the selected reuse mode required by the load scenario.
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

/// Finds the first complete HTTP/1 header block and returns the drain boundary,
/// leaving any pipelined bytes available for the next request.
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

#[cfg(test)]
mod tests {
    use super::{
        build_response, find_header_end, parse_port_value, parse_response_delay_ms_value,
        parse_workers_value, ConnectionMode, DEFAULT_PORT, DEFAULT_RESPONSE_DELAY_MS,
        DEFAULT_WORKERS, MAX_WORKERS,
    };

    /// Proves malformed startup controls are rejected by the parsers that run
    /// before `main` binds the listener, without mutating process-global env vars.
    #[test]
    fn invalid_startup_controls_fail_closed_before_binding() {
        assert!(parse_port_value(Some("0")).is_err());
        assert!(parse_port_value(Some("65536")).is_err());
        assert!(parse_port_value(Some("not-a-port")).is_err());
        assert_eq!(parse_port_value(None).unwrap(), DEFAULT_PORT);

        assert!(parse_workers_value(Some("0")).is_err());
        assert!(parse_workers_value(Some(&(MAX_WORKERS + 1).to_string())).is_err());
        let worker_overflow = format!("{}0", usize::MAX);
        assert!(parse_workers_value(Some(worker_overflow.as_str())).is_err());
        assert!(parse_workers_value(Some("not-a-worker-count")).is_err());
        assert_eq!(parse_workers_value(Some("4")).unwrap(), 4);
        assert_eq!(parse_workers_value(None).unwrap(), DEFAULT_WORKERS);

        assert!(parse_response_delay_ms_value(Some("18446744073709551616")).is_err());
        assert!(parse_response_delay_ms_value(Some("not-a-delay")).is_err());
        assert_eq!(parse_response_delay_ms_value(Some("150")).unwrap(), 150);
        assert_eq!(
            parse_response_delay_ms_value(None).unwrap(),
            DEFAULT_RESPONSE_DELAY_MS
        );

        assert!(ConnectionMode::parse(Some("upgrade")).is_err());
        assert_eq!(
            ConnectionMode::parse(Some("close")).unwrap(),
            ConnectionMode::Close
        );
        assert_eq!(ConnectionMode::parse(None).unwrap(), ConnectionMode::KeepAlive);
    }

    /// Prevents connection-mode changes from drifting away from emitted framing.
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

    /// Locks the parser boundary to the terminal CRLF sequence used by the
    /// direct socket self-check and keep-alive fixture path.
    #[test]
    fn header_boundary_includes_the_terminal_separator() {
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: test\r\n\r\nbody"), Some(30));
        assert_eq!(find_header_end(b"GET / HTTP/1.1\r\nHost: test\r\n"), None);
    }
}
