use std::env;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use cwl_pingora_gateway::edge_contract::{UpstreamConfig, UpstreamTimeouts};
use cwl_pingora_gateway::edge_routing::{RouteMatch, RouteRule};
use cwl_pingora_gateway::http_policy::ResponseHeaderRule;
use cwl_pingora_gateway::migration_delivery::MigrationDeliveryPlan;
use cwl_pingora_gateway::migration_plan::EdgeMigrationPlan;
use cwl_pingora_gateway::migration_proxy::MigrationGatewayProxy;
use cwl_pingora_gateway::runtime_isolation::RuntimeIsolationLimits;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use pingora::prelude::{http_proxy_service, Server};
use pingora::server::RunArgs;

const SERVER_TEST_NAME: &str = "migration_proxy_wire_server_fixture";

fn upstream(name: &str, port: u16) -> UpstreamConfig {
    UpstreamConfig {
        name: name.to_string(),
        address: SocketAddr::from(([127, 0, 0, 1], port)),
        tls: false,
        sni: None,
        trust_bundle_file: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 150,
            total_connection_ms: 300,
            read_ms: 500,
            write_ms: 500,
            idle_ms: 500,
        },
    }
}

fn response_rules() -> Vec<ResponseHeaderRule> {
    vec![
        ResponseHeaderRule {
            name: "X-Content-Type-Options".to_string(),
            value: "nosniff".to_string(),
        },
        ResponseHeaderRule {
            name: "X-Frame-Options".to_string(),
            value: "DENY".to_string(),
        },
        ResponseHeaderRule {
            name: "Referrer-Policy".to_string(),
            value: "no-referrer".to_string(),
        },
        ResponseHeaderRule {
            name: "Permissions-Policy".to_string(),
            value: "geolocation=(), microphone=(), camera=()".to_string(),
        },
    ]
}

fn fixture_proxy(origin_port: u16, dead_port: u16) -> MigrationGatewayProxy {
    let plan = EdgeMigrationPlan::try_new(
        vec!["backend".to_string(), "dead".to_string()],
        vec![
            RouteRule {
                name: "healthz".to_string(),
                priority: 100,
                matcher: RouteMatch::Exact("/healthz".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "api".to_string(),
                priority: 90,
                matcher: RouteMatch::PathPrefix("/api".to_string()),
                upstream: "backend".to_string(),
            },
            RouteRule {
                name: "dead".to_string(),
                priority: 80,
                matcher: RouteMatch::Exact("/dead".to_string()),
                upstream: "dead".to_string(),
            },
        ],
        response_rules(),
    )
    .expect("wire fixture route and HTTP policy must be valid");
    let delivery = MigrationDeliveryPlan::try_new(
        plan,
        vec![
            upstream("backend", origin_port),
            upstream("dead", dead_port),
        ],
    )
    .expect("wire fixture upstream bindings must be complete");
    let limits = RuntimeIsolationLimits::try_new(16, 8)
        .expect("wire fixture isolation limits must be valid");

    MigrationGatewayProxy::try_new(delivery, limits)
        .expect("wire fixture proxy must activate from validated contracts")
}

fn spawn_origin(port: u16) {
    thread::spawn(move || {
        let listener = TcpListener::bind(("127.0.0.1", port)).expect("origin fixture must bind");
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            thread::spawn(move || handle_origin(stream));
        }
    });
}

fn handle_origin(mut stream: TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("origin read timeout must be configurable");
    let mut request = Vec::new();
    let mut buffer = [0_u8; 2048];
    let mut expected_body = None;

    loop {
        let read = match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => panic!("origin fixture read failed: {error}"),
        };
        request.extend_from_slice(&buffer[..read]);

        if expected_body.is_none() {
            if let Some(header_end) = find_bytes(&request, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end + 4]);
                let content_length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                });
                expected_body = Some((header_end + 4, content_length.unwrap_or(0)));
            }
        }

        if let Some((body_start, body_len)) = expected_body {
            if request.len() >= body_start + body_len {
                break;
            }
        }
    }

    let request_text = String::from_utf8_lossy(&request);
    let mut echoed = String::new();
    for line in request_text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("x-forwarded-") || lower.starts_with("x-real-ip:") {
            echoed.push_str(line);
            echoed.push('\n');
        }
    }

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        echoed.len(),
        echoed
    );
    stream
        .write_all(response.as_bytes())
        .expect("origin fixture response must be writable");
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn port_base() -> u16 {
    env::var("CWL_PINGORA_WIRE_PORT_BASE")
        .expect("wire child must receive an explicit port base")
        .parse::<u16>()
        .expect("wire port base must be numeric")
}

#[test]
#[ignore = "subprocess fixture; exercised by migration_proxy_callbacks_run_on_real_http_traffic"]
fn migration_proxy_wire_server_fixture() {
    let base = port_base();
    let listener_port = base;
    let origin_port = base + 1;
    let dead_port = base + 2;
    spawn_origin(origin_port);

    let mut server = Server::new_with_opt_and_conf(None, build_server_conf(8));
    server.bootstrap();
    let mut proxy_service = http_proxy_service(
        &server.configuration,
        fixture_proxy(origin_port, dead_port),
    );
    proxy_service.add_tcp(&format!("127.0.0.1:{listener_port}"));
    server.add_service(proxy_service);
    server.run(RunArgs::default());
}

struct FixtureProcess {
    child: Child,
}

impl FixtureProcess {
    fn spawn(base: u16) -> Self {
        let child = Command::new(env::current_exe().expect("test executable path must exist"))
            .arg("--ignored")
            .arg("--exact")
            .arg(SERVER_TEST_NAME)
            .arg("--nocapture")
            .arg("--test-threads=1")
            .env("CWL_PINGORA_WIRE_PORT_BASE", base.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("wire fixture subprocess must start");
        Self { child }
    }

    fn shutdown(mut self) {
        let pid = self.child.id().to_string();
        let status = Command::new("kill")
            .args(["-TERM", pid.as_str()])
            .status()
            .expect("SIGTERM command must run");
        assert!(status.success(), "wire fixture SIGTERM must be delivered");

        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .expect("wire fixture exit status must be readable")
            {
                assert!(status.success(), "wire fixture must stop cleanly: {status}");
                return;
            }
            if Instant::now() >= deadline {
                self.child.kill().ok();
                let _ = self.child.wait();
                panic!("wire fixture did not stop within the graceful shutdown budget");
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for FixtureProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn wait_until_listening(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => {
                drop(stream);
                return;
            }
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Err(error) => panic!("wire fixture never became reachable: {error}"),
        }
    }
}

fn raw_request(port: u16, request: &[u8]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("gateway must accept request");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("client read timeout must be configurable");
    stream
        .write_all(request)
        .expect("wire request must be writable");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("wire request write side must close cleanly");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("gateway response must be readable");
    String::from_utf8(response).expect("gateway fixture responses must be UTF-8")
}

fn assert_status_and_policy(response: &str, status: u16) {
    assert!(
        response.starts_with(&format!("HTTP/1.1 {status} ")),
        "unexpected response status: {response}"
    );
    let lower = response.to_ascii_lowercase();
    assert!(lower.contains("x-content-type-options: nosniff\r\n"));
    assert!(lower.contains("x-frame-options: deny\r\n"));
    assert!(lower.contains("referrer-policy: no-referrer\r\n"));
    assert!(lower.contains(
        "permissions-policy: geolocation=(), microphone=(), camera=()\r\n"
    ));
}

#[test]
fn migration_proxy_callbacks_run_on_real_http_traffic() {
    let process_id = std::process::id() as u16;
    let base = 30_000 + (process_id % 4_000) * 4;
    let fixture = FixtureProcess::spawn(base);
    wait_until_listening(base);

    let ok = raw_request(
        base,
        b"GET /healthz HTTP/1.1\r\nHost: app.example:8080\r\nConnection: close\r\n\r\n",
    );
    assert_status_and_policy(&ok, 200);
    assert!(ok.contains("X-Forwarded-Host: app.example:8080\n"));
    assert!(ok.contains("X-Forwarded-Port: 8080\n"));
    assert!(ok.contains("X-Forwarded-Proto: http\n"));
    assert!(ok.contains("X-Real-IP: 127.0.0.1\n"));

    let small_body = raw_request(
        base,
        b"POST /api HTTP/1.1\r\nHost: app.example\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest",
    );
    assert_status_and_policy(&small_body, 200);
    assert!(small_body.contains("X-Forwarded-Port: 80\n"));

    let declared_oversize = raw_request(
        base,
        b"POST /api HTTP/1.1\r\nHost: app.example\r\nContent-Length: 17\r\nConnection: close\r\n\r\n",
    );
    assert_status_and_policy(&declared_oversize, 413);

    let streamed_oversize = raw_request(
        base,
        b"POST /api HTTP/1.1\r\nHost: app.example\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n11\r\n0123456789abcdefg\r\n0\r\n\r\n",
    );
    assert_status_and_policy(&streamed_oversize, 413);

    let invalid_authority = raw_request(
        base,
        b"GET /healthz HTTP/1.1\r\nHost: app.example:0\r\nConnection: close\r\n\r\n",
    );
    assert_status_and_policy(&invalid_authority, 400);

    let unmatched = raw_request(
        base,
        b"GET /missing HTTP/1.1\r\nHost: app.example\r\nConnection: close\r\n\r\n",
    );
    assert_status_and_policy(&unmatched, 404);

    let dead_upstream = raw_request(
        base,
        b"GET /dead HTTP/1.1\r\nHost: app.example\r\nConnection: close\r\n\r\n",
    );
    assert_status_and_policy(&dead_upstream, 502);

    fixture.shutdown();
}
