//! Production composition root for the characterized `pg-erd-cloud` edge migration.
//!
//! This binary is deliberately separate from the generic version-1 single-upstream runtime. It
//! can activate only the compiled pg-erd route/header contract and explicit `backend`/`frontend`
//! transport bindings admitted by `PgErdMigrationConfig`.

use std::env;
use std::fs;
use std::process::ExitCode;

use cwl_pingora_gateway::migration_admin::PgErdMigrationConfig;
use cwl_pingora_gateway::runtime_policy::build_server_conf;
use cwl_pingora_gateway::startup::GatewayCommand;
use pingora::prelude::{http_proxy_service, Server};
use pingora::server::RunArgs;

fn main() -> ExitCode {
    env_logger::init();

    let args: Vec<_> = env::args_os().collect();
    let command = match GatewayCommand::parse(&args) {
        Ok(command) => command,
        Err(error) => return exit_with_error(&error.to_string()),
    };
    let config_source = match fs::read_to_string(command.config_path()) {
        Ok(source) => source,
        Err(error) => {
            return exit_with_error(&format!(
                "unable to read pg-erd migration configuration {:?}: {:?}",
                command.config_path(),
                error.kind()
            ));
        }
    };
    let config = match PgErdMigrationConfig::from_yaml(&config_source) {
        Ok(config) => config,
        Err(error) => return exit_with_error(&error.to_string()),
    };
    let proxy = match config.build_proxy() {
        Ok(proxy) => proxy,
        Err(error) => return exit_with_error(&error.to_string()),
    };

    let listener = config.listener().to_string();
    let metrics_listener = config.metrics_listener().to_string();
    let mut server = Server::new_with_opt_and_conf(
        None,
        build_server_conf(config.upstream_keepalive_pool_size()),
    );
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp(&listener);
    server.add_service(proxy_service);

    let mut metrics_service = pingora_prometheus::prometheus_http_service();
    metrics_service.add_tcp(&metrics_listener);
    server.add_service(metrics_service);

    server.run(RunArgs::default());
    ExitCode::SUCCESS
}

fn exit_with_error(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(2)
}
