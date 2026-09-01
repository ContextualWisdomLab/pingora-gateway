//! Production composition root for the shared Pingora gateway.
//!
//! The binary obtains network authority only after the explicit edge contract has been parsed and
//! validated. Product policy stays outside this process boundary; this file only wires the
//! transport-neutral contract to the Pingora delivery adapter.

use std::env;
use std::fmt::Display;
use std::process;

use cwl_pingora_gateway::gateway_proxy::GatewayProxy;
use cwl_pingora_gateway::startup::GatewayCommand;
use pingora::prelude::{http_proxy_service, Server};

fn main() {
    env_logger::init();

    let command =
        GatewayCommand::parse(env::args_os()).unwrap_or_else(|error| exit_with_error(error));
    let config = command
        .load_config()
        .unwrap_or_else(|error| exit_with_error(error));
    let proxy =
        GatewayProxy::try_from_config(&config).unwrap_or_else(|error| exit_with_error(error));
    let listener = config.listener.to_string();
    let metrics_listener = config.metrics_listener.to_string();

    let mut server = Server::new(None).unwrap_or_else(|error| exit_with_error(error));
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp(&listener);
    server.add_service(proxy_service);

    let mut metrics_service = pingora_prometheus::prometheus_http_service();
    metrics_service.add_tcp(&metrics_listener);
    server.add_service(metrics_service);

    server.run_forever();
}

fn exit_with_error(error: impl Display) -> ! {
    eprintln!("{error}");
    process::exit(2);
}
