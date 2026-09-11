//! Production composition root for the shared Pingora gateway.
//!
//! The binary obtains network authority only after the explicit edge contract has been parsed and
//! validated. Product policy stays outside this process boundary; this file only wires the
//! transport-neutral contract to the Pingora delivery adapter.

use std::env;
use std::fmt::Display;
use std::process::ExitCode;

use cwl_pingora_gateway::logging_policy::init_runtime_logging;
use cwl_pingora_gateway::runtime_composition::compose_gateway_runtime;
use cwl_pingora_gateway::startup::GatewayCommand;
use cwl_pingora_gateway::tls_delivery::build_downstream_tls_settings;
use pingora::prelude::{http_proxy_service, Server};
use pingora::server::RunArgs;

/// Initializes payload-safe logging, validates Admin Config, then grants listener authority.
fn main() -> ExitCode {
    init_runtime_logging();

    let args: Vec<_> = env::args_os().collect();
    let command = match GatewayCommand::parse(&args) {
        Ok(command) => command,
        Err(error) => return exit_with_error(error),
    };
    let config = match command.load_config() {
        Ok(config) => config,
        Err(error) => return exit_with_error(error),
    };
    let (proxy, server_conf) = match compose_gateway_runtime(&config) {
        Ok(runtime) => runtime,
        Err(error) => return exit_with_error(error),
    };
    let downstream_tls = match config.downstream_tls() {
        Some(tls) => match build_downstream_tls_settings(tls) {
            Ok(settings) => Some(settings),
            Err(error) => return exit_with_error(error),
        },
        None => None,
    };
    let listener = config.listener.to_string();
    let metrics_listener = config.metrics_listener.to_string();

    let mut server = Server::new_with_opt_and_conf(None, server_conf);
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    if let Some(settings) = downstream_tls {
        proxy_service.add_tls_with_settings(&listener, None, settings);
    } else {
        proxy_service.add_tcp(&listener);
    }
    server.add_service(proxy_service);

    let mut metrics_service = pingora_prometheus::prometheus_http_service();
    // Pingora's global `threads` value also sizes HttpProxy shutdown sharding, so the proxy must
    // keep that exact value. The low-volume metrics listener is isolated at one worker instead of
    // multiplying operator-facing telemetry threads with proxy capacity.
    metrics_service.threads = Some(1);
    metrics_service.add_tcp(&metrics_listener);
    server.add_service(metrics_service);

    // `run_forever()` calls `process::exit(0)` after the same drain path. Returning an ExitCode
    // from `main` after `run()` preserves Pingora's graceful shutdown while allowing process
    // destructors, profile data and diagnostics to flush on both success and startup failure.
    server.run(RunArgs::default());
    ExitCode::SUCCESS
}

/// Emits a bounded startup/configuration error and returns the stable failure exit code.
fn exit_with_error(error: impl Display) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(2)
}
