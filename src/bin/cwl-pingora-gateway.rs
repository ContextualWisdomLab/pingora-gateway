//! Production composition root for the shared Pingora gateway.
//!
//! The binary obtains network authority only after the explicit edge contract has been parsed and
//! validated. Product policy stays outside this process boundary; this file only wires the
//! transport-neutral contract to the Pingora delivery adapter.

use std::env;
use std::fmt::Display;
use std::process::ExitCode;

use cwl_pingora_gateway::gateway_proxy::GatewayProxy;
use cwl_pingora_gateway::logging_policy::init_runtime_logging;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use cwl_pingora_gateway::startup::GatewayCommand;
use pingora::prelude::{http_proxy_service, Server};
use pingora::server::RunArgs;

fn main() -> ExitCode {
    if let Err(error) = init_runtime_logging() {
        return exit_with_error(format!(
            "unable to initialize payload-safe runtime logging: {error}"
        ));
    }

    let args: Vec<_> = env::args_os().collect();
    let command = match GatewayCommand::parse(&args) {
        Ok(command) => command,
        Err(error) => return exit_with_error(error),
    };
    let config = match command.load_config() {
        Ok(config) => config,
        Err(error) => return exit_with_error(error),
    };
    let proxy = match GatewayProxy::try_from_config(&config) {
        Ok(proxy) => proxy,
        Err(error) => return exit_with_error(error),
    };
    let listener = config.listener.to_string();
    let metrics_listener = config.metrics_listener.to_string();

    let mut server =
        Server::new_with_opt_and_conf(None, build_server_conf(config.upstream_keepalive_pool_size));
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp(&listener);
    server.add_service(proxy_service);

    let mut metrics_service = pingora_prometheus::prometheus_http_service();
    metrics_service.add_tcp(&metrics_listener);
    server.add_service(metrics_service);

    // `run_forever()` calls `process::exit(0)` after the same drain path. Returning an ExitCode
    // from `main` after `run()` preserves Pingora's graceful shutdown while allowing process
    // destructors, profile data and diagnostics to flush on both success and startup failure.
    server.run(RunArgs::default());
    ExitCode::SUCCESS
}

fn exit_with_error(error: impl Display) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(2)
}
