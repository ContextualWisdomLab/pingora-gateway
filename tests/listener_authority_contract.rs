//! Generic v1 upstream-to-own-listener admission regression, from Issue #26.

use cwl_pingora_gateway::edge_contract::GatewayConfig;

/// Configures both real startup modes with occupied, fixture-owned sockets and proves rejection.
#[test]
fn real_startup_parser_rejects_self_routing_in_both_modes() {
    use cwl_pingora_gateway::startup::{GatewayCommand, GatewayCommandError};
    use std::ffi::OsString;
    use std::net::TcpListener;

    let traffic = TcpListener::bind("127.0.0.1:0").unwrap();
    let metrics = TcpListener::bind("127.0.0.1:0").unwrap();
    let directory = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let path = directory.path().join("gateway.yaml");
    for upstream in [traffic.local_addr().unwrap(), metrics.local_addr().unwrap()] {
        std::fs::write(
            &path,
            source(
                &traffic.local_addr().unwrap().to_string(),
                &metrics.local_addr().unwrap().to_string(),
                &upstream.to_string(),
            ),
        )
        .unwrap();
        for check in [false, true] {
            let mut args = vec![
                OsString::from("gateway"),
                OsString::from("--config"),
                path.clone().into_os_string(),
            ];
            if check {
                args.push(OsString::from("--check"));
            }
            let command = GatewayCommand::parse(&args).unwrap();
            assert_eq!(command.check_only(), check);
            assert_eq!(command.config_path(), path);
            let error = command
                .load_config()
                .expect_err("startup must reject self-routing before delivery composition");
            assert_eq!(error.path(), Some(path.as_path()));
            assert!(matches!(error, GatewayCommandError::InvalidConfig { .. }));
        }
    }
    // Existing sockets remain owned; this seam never starts Pingora or sends recursive traffic.
    assert_eq!(traffic.local_addr().unwrap().ip().to_string(), "127.0.0.1");
    assert_eq!(metrics.local_addr().unwrap().ip().to_string(), "127.0.0.1");
}

/// Constructs an otherwise-valid single-upstream configuration without acquiring sockets.
fn source(listener: &str, metrics: &str, upstream: &str) -> String {
    format!(
        "version: 1\nlistener: \"{listener}\"\nmetrics_listener: \"{metrics}\"\nmax_request_body_bytes: 8\nmax_in_flight_requests: 8\nupstream_keepalive_pool_size: 4\nupstreams:\n  - name: application\n    address: \"{upstream}\"\n    tls: false\n    timeouts:\n      connection_ms: 100\n      total_connection_ms: 200\n      read_ms: 300\n      write_ms: 400\n      idle_ms: 500\n"
    )
}

#[test]
fn rejects_traffic_listener_self_routing_before_network_activation() {
    let config = source("127.0.0.1:6188", "127.0.0.1:6192", "127.0.0.1:6188");
    let error = GatewayConfig::from_yaml(&config).err();
    assert_eq!(
        error.map(|error| error.to_string()),
        Some("upstream application must not overlap a gateway listener authority".to_string()),
        "upstream equal to the traffic listener must be rejected by the real config contract"
    );
}

#[test]
fn preserves_distinct_upstream_authority() {
    for (listener, metrics, upstream) in [
        ("127.0.0.1:6188", "127.0.0.1:6192", "127.0.0.1:8080"),
        ("127.0.0.1:6188", "127.0.0.1:6192", "127.0.0.2:6188"),
        ("[::1]:6188", "127.0.0.1:6192", "[::2]:6188"),
        ("[::1]:6188", "127.0.0.1:6192", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "127.0.0.1:6192", "[::1]:6188"),
        ("0.0.0.0:6188", "[::]:6192", "127.0.0.1:8080"),
    ] {
        let config = source(listener, metrics, upstream);
        assert!(GatewayConfig::from_yaml(&config).is_ok(), "{config}");
    }
}

#[test]
fn rejects_metrics_and_aliased_self_routing_before_network_activation() {
    // Cases are retained from PR #12's existing generic/pg-erd alias matrix; no traffic is sent.
    for (listener, metrics, upstream) in [
        ("127.0.0.1:6188", "127.0.0.1:6192", "127.0.0.1:6188"),
        ("0.0.0.0:6188", "127.0.0.1:6192", "127.0.0.1:6188"),
        ("[::]:6188", "127.0.0.1:6192", "127.0.0.1:6188"),
        (
            "127.0.0.1:6188",
            "127.0.0.1:6192",
            "[::ffff:127.0.0.1]:6188",
        ),
        (
            "[::ffff:127.0.0.1]:6188",
            "127.0.0.1:6192",
            "127.0.0.1:6188",
        ),
        ("127.0.0.1:6188", "127.0.0.1:6192", "127.0.0.1:6192"),
        ("127.0.0.1:6188", "0.0.0.0:6192", "127.0.0.1:6192"),
        ("127.0.0.1:6188", "[::]:6192", "127.0.0.1:6192"),
        (
            "127.0.0.1:6188",
            "127.0.0.1:6192",
            "[::ffff:127.0.0.1]:6192",
        ),
        (
            "127.0.0.1:6188",
            "[::ffff:127.0.0.1]:6192",
            "127.0.0.1:6192",
        ),
        ("[::]:6188", "127.0.0.1:6192", "[::1]:6188"),
        ("[::1]:6188", "127.0.0.1:6192", "[::1]:6188"),
        ("[::1]:6188", "127.0.0.1:6192", "[::]:6188"),
        ("127.0.0.1:6188", "127.0.0.1:6192", "0.0.0.0:6188"),
        ("127.0.0.1:6188", "127.0.0.1:6192", "[::]:6188"),
        ("[::ffff:0.0.0.0]:6188", "127.0.0.1:6192", "127.0.0.1:6188"),
        ("127.0.0.1:6188", "127.0.0.1:6192", "[::ffff:0.0.0.0]:6188"),
        ("127.0.0.1:6188", "[::]:6192", "[::1]:6192"),
    ] {
        let config = source(listener, metrics, upstream);
        assert_eq!(
            GatewayConfig::from_yaml(&config)
                .err()
                .map(|error| error.to_string()),
            Some("upstream application must not overlap a gateway listener authority".to_string()),
            "{config}"
        );
    }
}
