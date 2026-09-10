//! Manual representative Linux/NUMA profile for parked HTTP/1 keep-alive shutdown.
//!
//! Normal CI compiles this target but does not execute it. The ignored test runs only when an
//! operator explicitly supplies the representative-profile environment and therefore cannot turn a
//! small hosted runner into commercial contention evidence by accident.

#![cfg(target_os = "linux")]

use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

const REGISTERED_SERVICE_COUNT: usize = 2;
const METRICS_SERVICE_THREADS: usize = 1;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const PROCESS_EXIT_BOUND: Duration = Duration::from_secs(30);

#[derive(Debug)]
struct GatewayProcess(Child);

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[derive(Debug)]
struct Topology {
    online_cpus: usize,
    numa_nodes: usize,
    sockets: usize,
    csv: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct SchedulerSample {
    task_count: u64,
    cpu_runtime_ns: u64,
    runqueue_wait_ns: u64,
    timeslices: u64,
    voluntary_ctxt_switches: u64,
    nonvoluntary_ctxt_switches: u64,
}

impl SchedulerSample {
    fn saturating_delta(self, earlier: Self) -> Self {
        Self {
            task_count: self.task_count,
            cpu_runtime_ns: self.cpu_runtime_ns.saturating_sub(earlier.cpu_runtime_ns),
            runqueue_wait_ns: self.runqueue_wait_ns.saturating_sub(earlier.runqueue_wait_ns),
            timeslices: self.timeslices.saturating_sub(earlier.timeslices),
            voluntary_ctxt_switches: self
                .voluntary_ctxt_switches
                .saturating_sub(earlier.voluntary_ctxt_switches),
            nonvoluntary_ctxt_switches: self
                .nonvoluntary_ctxt_switches
                .saturating_sub(earlier.nonvoluntary_ctxt_switches),
        }
    }
}

#[derive(Debug)]
struct RoundEvidence {
    close_latencies_ms: Vec<u128>,
    survivors_at_close_bound: usize,
    process_exit_ms: u128,
    scheduler_delta: Option<SchedulerSample>,
}

fn required_env_usize(name: &str) -> usize {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must be set for representative profiling"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("{name} must be a positive integer"))
}

fn command_stdout(program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("unable to execute {program}: {error}"));
    assert!(
        output.status.success(),
        "{program} {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("profile command output must be UTF-8")
}

fn detect_topology() -> Topology {
    let online_cpus = command_stdout("nproc", &[])
        .trim()
        .parse::<usize>()
        .expect("nproc must return an integer");
    let csv = command_stdout("lscpu", &["-p=CPU,NODE,SOCKET"]);
    let mut nodes = BTreeSet::new();
    let mut sockets = BTreeSet::new();
    for line in csv.lines().filter(|line| !line.starts_with('#') && !line.is_empty()) {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 3, "lscpu topology row must have CPU,NODE,SOCKET");
        assert!(fields[0].parse::<usize>().is_ok(), "CPU id must be numeric");
        if fields[1] != "-" {
            nodes.insert(fields[1].to_owned());
        }
        if fields[2] != "-" {
            sockets.insert(fields[2].to_owned());
        }
    }
    Topology {
        online_cpus,
        numa_nodes: nodes.len(),
        sockets: sockets.len(),
        csv,
    }
}

fn reserve_distinct_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let traffic = TcpListener::bind("127.0.0.1:0").expect("traffic port should be reservable");
    let metrics = TcpListener::bind("127.0.0.1:0").expect("metrics port should be reservable");
    let addresses = (
        traffic.local_addr().expect("traffic address should exist"),
        metrics.local_addr().expect("metrics address should exist"),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn write_config(listener: SocketAddr, metrics_listener: SocketAddr, service_threads: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("temporary profile config should be writable");
    writeln!(
        file,
        "version: 1\nlistener: {listener}\nmetrics_listener: {metrics_listener}\nmax_request_body_bytes: 1024\nmax_in_flight_requests: 128\nservice_threads: {service_threads}\nupstream_keepalive_pool_size: 32\nupstreams:\n  - name: unused-profile-origin\n    address: 127.0.0.1:9\n    tls: false\n    timeouts:\n      connection_ms: 250\n      total_connection_ms: 500\n      read_ms: 1000\n      write_ms: 1000\n      idle_ms: 5000"
    )
    .expect("profile config should be written");
    file
}

fn wait_until_listening(address: SocketAddr, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            panic!("gateway exited before profile traffic: {status}");
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "gateway did not start within 15 seconds");
        thread::sleep(Duration::from_millis(25));
    }
}

fn read_zero_length_health_response(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("health read timeout should be configurable");
    let mut response = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        let read = stream
            .read(&mut buffer)
            .expect("health response should arrive before timeout");
        assert!(read > 0, "gateway closed before health response completed");
        response.extend_from_slice(&buffer[..read]);
        assert!(response.len() <= MAX_HEADER_BYTES, "health header exceeded 16 KiB");
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    assert!(
        response.starts_with(b"HTTP/1.1 200 "),
        "health response must be HTTP/1.1 200: {:?}",
        String::from_utf8_lossy(&response)
    );
    let lower = String::from_utf8_lossy(&response).to_ascii_lowercase();
    assert!(
        lower.contains("\r\ncontent-length: 0\r\n"),
        "health response must remain zero-length before the socket is parked"
    );
}

fn park_health_connections(address: SocketAddr, count: usize) -> Vec<TcpStream> {
    let mut streams = Vec::with_capacity(count);
    for index in 0..count {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
            .unwrap_or_else(|error| panic!("parked connection {index} failed: {error}"));
        stream
            .set_nodelay(true)
            .expect("profile socket TCP_NODELAY should be configurable");
        stream
            .write_all(b"GET /livez HTTP/1.1\r\nHost: gateway.test\r\nConnection: keep-alive\r\n\r\n")
            .unwrap_or_else(|error| panic!("parked connection {index} write failed: {error}"));
        read_zero_length_health_response(&mut stream);
        stream
            .set_nonblocking(true)
            .expect("profile socket must become nonblocking");
        streams.push(stream);
    }

    thread::sleep(Duration::from_millis(250));
    let mut probe = [0_u8; 1];
    for (index, stream) in streams.iter_mut().enumerate() {
        match stream.read(&mut probe) {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Ok(0) => panic!("parked connection {index} closed before SIGTERM"),
            Ok(read) => panic!("parked connection {index} received {read} byte(s) before SIGTERM"),
            Err(error) => panic!("parked connection {index} pre-SIGTERM read failed: {error}"),
        }
    }
    streams
}

fn parse_status_counter(status: &str, key: &str) -> u64 {
    status
        .lines()
        .find_map(|line| {
            let (name, raw) = line.split_once(':')?;
            (name == key).then(|| raw.trim().parse::<u64>().expect("status counter must be numeric"))
        })
        .unwrap_or(0)
}

fn read_scheduler_sample(pid: u32) -> Option<SchedulerSample> {
    let task_dir = format!("/proc/{pid}/task");
    let entries = fs::read_dir(task_dir).ok()?;
    let mut sample = SchedulerSample::default();
    for entry in entries.flatten() {
        let tid = entry.file_name();
        let tid = tid.to_string_lossy();
        let schedstat = fs::read_to_string(format!("/proc/{pid}/task/{tid}/schedstat")).ok()?;
        let values: Vec<_> = schedstat.split_whitespace().collect();
        if values.len() < 3 {
            return None;
        }
        sample.task_count += 1;
        sample.cpu_runtime_ns = sample
            .cpu_runtime_ns
            .saturating_add(values[0].parse::<u64>().ok()?);
        sample.runqueue_wait_ns = sample
            .runqueue_wait_ns
            .saturating_add(values[1].parse::<u64>().ok()?);
        sample.timeslices = sample.timeslices.saturating_add(values[2].parse::<u64>().ok()?);

        let status = fs::read_to_string(format!("/proc/{pid}/task/{tid}/status")).ok()?;
        sample.voluntary_ctxt_switches = sample
            .voluntary_ctxt_switches
            .saturating_add(parse_status_counter(&status, "voluntary_ctxt_switches"));
        sample.nonvoluntary_ctxt_switches = sample
            .nonvoluntary_ctxt_switches
            .saturating_add(parse_status_counter(&status, "nonvoluntary_ctxt_switches"));
    }
    Some(sample)
}

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    assert!(!sorted.is_empty());
    let rank = ((sorted.len() * percentile) + 99) / 100;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn wait_for_process_exit(process: &mut Child, started: Instant) -> u128 {
    let deadline = Instant::now() + PROCESS_EXIT_BOUND;
    loop {
        if let Some(status) = process
            .try_wait()
            .expect("gateway process state should be readable")
        {
            assert!(status.success(), "SIGTERM shutdown must exit successfully: {status}");
            return started.elapsed().as_millis();
        }
        assert!(Instant::now() < deadline, "gateway exceeded the 30-second process exit bound");
        thread::sleep(Duration::from_millis(2));
    }
}

fn run_round(service_threads: usize, parked_connections: usize, close_bound_ms: usize) -> RoundEvidence {
    let (gateway_address, metrics_address) = reserve_distinct_loopback_addresses();
    let config = write_config(gateway_address, metrics_address, service_threads);
    let child = Command::new(env!("CARGO_BIN_EXE_cwl-pingora-gateway"))
        .args(["--config", config.path().to_str().expect("UTF-8 config path")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("compiled gateway binary should start");
    let mut process = GatewayProcess(child);
    wait_until_listening(gateway_address, &mut process.0);

    let mut streams = park_health_connections(gateway_address, parked_connections);
    let scheduler_before = read_scheduler_sample(process.0.id());
    let signal_sent_at = Instant::now();
    let signal_status = Command::new("kill")
        .args(["-TERM", &process.0.id().to_string()])
        .status()
        .expect("kill must be available on Linux profile runners");
    assert!(signal_status.success(), "SIGTERM delivery should succeed");

    let close_bound = Duration::from_millis(close_bound_ms as u64);
    let mut close_latencies_ms = vec![None; streams.len()];
    let mut probe = [0_u8; 1];
    while signal_sent_at.elapsed() < close_bound {
        let mut open = 0_usize;
        for (index, stream) in streams.iter_mut().enumerate() {
            if close_latencies_ms[index].is_some() {
                continue;
            }
            match stream.read(&mut probe) {
                Ok(0) => close_latencies_ms[index] = Some(signal_sent_at.elapsed().as_millis()),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => open += 1,
                Ok(read) => panic!("parked connection {index} received {read} shutdown byte(s)"),
                Err(error) => panic!("parked connection {index} did not close cleanly: {error}"),
            }
        }
        if open == 0 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    let survivors_at_close_bound = close_latencies_ms.iter().filter(|value| value.is_none()).count();
    let scheduler_after = read_scheduler_sample(process.0.id());
    let process_exit_ms = wait_for_process_exit(&mut process.0, signal_sent_at);
    let scheduler_delta = scheduler_before.zip(scheduler_after).map(|(before, after)| after.saturating_delta(before));
    let observed = close_latencies_ms.into_iter().flatten().collect::<Vec<_>>();

    RoundEvidence {
        close_latencies_ms: observed,
        survivors_at_close_bound,
        process_exit_ms,
        scheduler_delta,
    }
}

fn current_git_sha() -> String {
    command_stdout("git", &["rev-parse", "HEAD"]).trim().to_owned()
}

fn append_scheduler_evidence(output: &mut String, round: usize, sample: Option<SchedulerSample>) {
    match sample {
        Some(sample) => {
            output.push_str(&format!("round_{round}_scheduler_available=true\n"));
            output.push_str(&format!("round_{round}_scheduler_task_count={}\n", sample.task_count));
            output.push_str(&format!("round_{round}_cpu_runtime_ns={}\n", sample.cpu_runtime_ns));
            output.push_str(&format!("round_{round}_runqueue_wait_ns={}\n", sample.runqueue_wait_ns));
            output.push_str(&format!("round_{round}_timeslices={}\n", sample.timeslices));
            output.push_str(&format!(
                "round_{round}_voluntary_ctxt_switches={}\n",
                sample.voluntary_ctxt_switches
            ));
            output.push_str(&format!(
                "round_{round}_nonvoluntary_ctxt_switches={}\n",
                sample.nonvoluntary_ctxt_switches
            ));
        }
        None => output.push_str(&format!("round_{round}_scheduler_available=false\n")),
    }
}

#[test]
#[ignore = "requires an explicitly dispatched representative Linux/NUMA runner"]
fn representative_numa_shutdown_profile() {
    assert_eq!(
        std::env::var("CWL_NUMA_PROFILE").as_deref(),
        Ok("1"),
        "ignored profile must not run without explicit CWL_NUMA_PROFILE=1"
    );

    let service_threads = required_env_usize("CWL_PROFILE_SERVICE_THREADS");
    let parked_connections = required_env_usize("CWL_PROFILE_PARKED_CONNECTIONS");
    let rounds = required_env_usize("CWL_PROFILE_ROUNDS");
    let close_bound_ms = required_env_usize("CWL_PROFILE_CLOSE_BOUND_MS");
    let min_online_cpus = required_env_usize("CWL_PROFILE_MIN_ONLINE_CPUS");
    let min_numa_nodes = required_env_usize("CWL_PROFILE_MIN_NUMA_NODES");
    let evidence_path = std::env::var("CWL_PROFILE_EVIDENCE_PATH")
        .expect("CWL_PROFILE_EVIDENCE_PATH must identify the immutable run receipt path");
    let expected_sha = std::env::var("CWL_PROFILE_EXPECTED_SHA")
        .expect("CWL_PROFILE_EXPECTED_SHA must bind evidence to an exact candidate");

    assert!((1..=256).contains(&service_threads));
    assert!(parked_connections >= 4096, "representative profile must not reduce connection pressure");
    assert!(rounds >= 25, "representative profile must preserve repeated shutdown jitter");
    assert_eq!(close_bound_ms, 1000, "correctness evidence keeps the one-second close bound");

    let actual_sha = current_git_sha();
    assert_eq!(actual_sha, expected_sha, "profile checkout must match exact candidate SHA");
    let topology = detect_topology();
    assert!(
        topology.online_cpus >= min_online_cpus,
        "runner has {} online CPUs but profile requires at least {min_online_cpus}",
        topology.online_cpus
    );
    assert!(
        topology.numa_nodes >= min_numa_nodes,
        "runner has {} NUMA nodes but profile requires at least {min_numa_nodes}",
        topology.numa_nodes
    );
    assert!(
        service_threads <= topology.online_cpus,
        "configured proxy workers must not exceed visible online CPUs for this profile"
    );

    let mut all_close_latencies = Vec::with_capacity(parked_connections * rounds);
    let mut all_survivors = 0_usize;
    let mut max_process_exit_ms = 0_u128;
    let mut scheduler_rounds = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let evidence = run_round(service_threads, parked_connections, close_bound_ms);
        all_survivors += evidence.survivors_at_close_bound;
        max_process_exit_ms = max_process_exit_ms.max(evidence.process_exit_ms);
        all_close_latencies.extend(evidence.close_latencies_ms);
        scheduler_rounds.push(evidence.scheduler_delta);
    }

    all_close_latencies.sort_unstable();
    assert!(!all_close_latencies.is_empty(), "profile must observe socket cleanup latencies");
    let p50 = percentile(&all_close_latencies, 50);
    let p95 = percentile(&all_close_latencies, 95);
    let p99 = percentile(&all_close_latencies, 99);
    let max = *all_close_latencies.last().expect("non-empty latency set");

    let mut output = String::new();
    output.push_str(&format!("expected_sha={expected_sha}\nactual_sha={actual_sha}\n"));
    output.push_str("supplier_packages=pingora=0.9.0,pingora-prometheus=0.9.0\n");
    output.push_str(&format!("configured_proxy_service_threads={service_threads}\n"));
    output.push_str(&format!("configured_metrics_service_threads={METRICS_SERVICE_THREADS}\n"));
    output.push_str(&format!("registered_service_count={REGISTERED_SERVICE_COUNT}\n"));
    output.push_str(&format!(
        "configured_service_worker_slots={}\n",
        service_threads + METRICS_SERVICE_THREADS
    ));
    output.push_str(&format!("online_cpus={}\nnuma_nodes={}\nsockets={}\n", topology.online_cpus, topology.numa_nodes, topology.sockets));
    output.push_str(&format!("parked_connections={parked_connections}\nprofile_rounds={rounds}\n"));
    output.push_str(&format!("shutdown_close_bound_ms={close_bound_ms}\n"));
    output.push_str(&format!("shutdown_close_p50_ms={p50}\nshutdown_close_p95_ms={p95}\nshutdown_close_p99_ms={p99}\nshutdown_close_max_ms={max}\n"));
    output.push_str(&format!("survivors_at_close_bound={all_survivors}\nmax_process_exit_ms={max_process_exit_ms}\n"));
    for (index, sample) in scheduler_rounds.into_iter().enumerate() {
        append_scheduler_evidence(&mut output, index + 1, sample);
    }
    output.push_str("topology_cpu_node_socket_begin\n");
    output.push_str(&topology.csv);
    if !topology.csv.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("topology_cpu_node_socket_end\n");
    fs::write(&evidence_path, output).expect("profile evidence receipt must be writable");

    assert_eq!(all_survivors, 0, "no parked keep-alive connection may survive the one-second bound");
    assert!(max <= close_bound_ms as u128, "all observed cleanup latencies must stay within the one-second bound");
}
