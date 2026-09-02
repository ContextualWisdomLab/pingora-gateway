//! Process logging policy that preserves the gateway's payload-minimization boundary.
//!
//! Pingora is a transport dependency, not an authority for CWL observability semantics. Some
//! supplier diagnostics include request-derived URI or header material. Operator-selected
//! verbosity therefore remains available, but Pingora-family record messages are replaced with a
//! static diagnostic marker before the process logger formats them. CWL-owned log targets retain
//! their normal bounded message content.

use log::{Log, Metadata, Record, SetLoggerError};

const REDACTED_PINGORA_DIAGNOSTIC: &str =
    "Pingora diagnostic message redacted by gateway payload-minimization policy";

struct PayloadSafeLogger {
    inner: env_logger::Logger,
}

impl PayloadSafeLogger {
    fn from_default_env() -> Self {
        Self {
            inner: env_logger::Builder::from_env(env_logger::Env::default()).build(),
        }
    }

    fn max_level(&self) -> log::LevelFilter {
        self.inner.filter()
    }
}

impl Log for PayloadSafeLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if is_pingora_dependency_target(record.target()) {
            self.inner.log(
                &Record::builder()
                    .args(format_args!("{REDACTED_PINGORA_DIAGNOSTIC}"))
                    .level(record.level())
                    .target(record.target())
                    .build(),
            );
            return;
        }

        self.inner.log(record);
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

fn is_pingora_dependency_target(target: &str) -> bool {
    target == "pingora" || target.starts_with("pingora_") || target.starts_with("pingora::")
}

/// Installs the process logger while enforcing payload-safe Pingora dependency diagnostics.
///
/// The operator's normal `RUST_LOG` directives still determine which records are enabled. The
/// gateway adds one non-bypassable security rule: message bodies emitted by Pingora-family targets
/// are replaced with a static marker before formatting, so broad dependency diagnostics cannot
/// disclose request-derived URI, header, cookie, credential, or payload material.
pub fn init_runtime_logging() -> Result<(), SetLoggerError> {
    let logger = PayloadSafeLogger::from_default_env();
    let max_level = logger.max_level();
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(max_level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_pingora_dependency_target;

    #[test]
    fn pingora_family_targets_are_classified_without_absorbing_application_targets() {
        for target in [
            "pingora",
            "pingora_proxy",
            "pingora_proxy::proxy_h1",
            "pingora_core::protocols::http",
            "pingora_http",
            "pingora_prometheus",
        ] {
            assert!(
                is_pingora_dependency_target(target),
                "Pingora dependency target must be payload-redacted: {target}"
            );
        }

        for target in [
            "cwl_pingora_gateway::observability",
            "pg_erd_cloud",
            "application::pingora_adapter",
            "not_pingora",
        ] {
            assert!(
                !is_pingora_dependency_target(target),
                "non-Pingora target must retain its own logging contract: {target}"
            );
        }
    }
}
