//! pingora-gateway process entry point.

use std::{env, process, sync::Arc};

use pingora_core::{server::configuration::Opt, server::Server};
use pingora_gateway::{
    delivery::pingora_proxy::GatewayProxy,
    runtime_configuration::ValidatedConfig,
    telemetry::Metrics,
};
use tracing::error;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_current_span(false)
        .with_span_list(false)
        .init();

    let path = env::var("PINGORA_GATEWAY_CONFIG")
        .unwrap_or_else(|_| "/etc/pingora-gateway/gateway.yaml".to_owned());
    let config = match ValidatedConfig::load(&path) {
        Ok(config) => config,
        Err(error) => {
            error!(config_path = %path, error = %error, "configuration rejected before listener startup");
            process::exit(78);
        }
    };

    let listener = config.listener.to_string();
    let route_count = config.routes.len();
    let metrics = Arc::new(Metrics::default());
    let proxy = GatewayProxy::new(Arc::new(config.routes), config.limits, metrics);

    let mut server = Server::new(Some(Opt::default())).unwrap_or_else(|error| {
        error!(error = %error, "Pingora server initialization failed");
        process::exit(70);
    });
    server.bootstrap();
    let mut service = pingora_proxy::http_proxy_service(&server.configuration, proxy);
    service.add_tcp(&listener);
    server.add_service(service);
    tracing::info!(%listener, route_count, "pingora gateway starting");
    server.run_forever();
}
