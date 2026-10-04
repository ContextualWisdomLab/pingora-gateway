//! Fail-closed process startup boundary for the shared gateway.
//!
//! Network authority is never activated from an implicit configuration path. Operators must pass
//! exactly one `--config <path>` argument, and the referenced file must both exist and satisfy the
//! versioned edge contract before the delivery layer may start listeners. Optional `--check` requests
//! local-only preflight; the binary must also load upstream trust and return before server creation.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};

/// Parsed command-line authority required before gateway startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayCommand {
    /// Explicit configuration source selected by the operator.
    config_path: PathBuf,
    /// Whether to validate local inputs and exit before granting listener authority.
    check_only: bool,
}

/// Failures that prevent the process from obtaining network authority.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatewayCommandError {
    /// The caller omitted the mandatory explicit configuration path.
    #[error("missing required --config <path> option")]
    MissingConfigOption,
    /// The caller provided `--config` without a path value.
    #[error("--config requires a path value")]
    MissingConfigValue,
    /// More than one configuration source is ambiguous and therefore rejected.
    #[error("--config may be specified exactly once")]
    DuplicateConfigOption,
    /// The startup surface rejects unknown arguments and repeated `--check` without new error variants.
    #[error("unexpected startup argument: {0}")]
    UnexpectedArgument(String),
    /// The explicit configuration file could not be read.
    #[error("unable to read gateway configuration {path:?}: {kind:?}")]
    ReadConfig {
        /// Explicit path requested by the operator.
        path: PathBuf,
        /// Stable operating-system failure classification.
        kind: ErrorKind,
    },
    /// The explicit configuration file violated the versioned edge contract.
    #[error("gateway configuration {path:?} is invalid: {source}")]
    InvalidConfig {
        /// Explicit path requested by the operator.
        path: PathBuf,
        /// Contract validation failure.
        #[source]
        source: GatewayConfigError,
    },
}

impl GatewayCommand {
    /// Parses exactly one explicit configuration path and an optional single `--check` in either order.
    ///
    /// Empty values and option-shaped path values are rejected; prefix such a filename with `./`.
    /// Normal serving is selected when `--check` is absent. Repeated `--check` uses the existing
    /// unexpected-argument error to preserve the public error enum.
    ///
    /// A slice keeps this parser monomorphic: coverage and failure evidence then describe the same
    /// executable parser used by the process instead of compiler-generated generic instantiations.
    pub fn parse(args: &[OsString]) -> Result<Self, GatewayCommandError> {
        let mut arguments = args.iter();
        let _program_name = arguments.next();
        let mut config_path = None;
        let mut check_only = false;

        while let Some(argument) = arguments.next() {
            if argument == "--check" {
                if check_only {
                    return Err(GatewayCommandError::UnexpectedArgument(
                        "--check".to_string(),
                    ));
                }
                check_only = true;
                continue;
            }
            if argument != "--config" {
                return Err(GatewayCommandError::UnexpectedArgument(
                    argument.to_string_lossy().into_owned(),
                ));
            }
            if config_path.is_some() {
                return Err(GatewayCommandError::DuplicateConfigOption);
            }
            let value = arguments
                .next()
                .ok_or(GatewayCommandError::MissingConfigValue)?;
            if value.is_empty() || value.to_string_lossy().starts_with("--") {
                return Err(GatewayCommandError::MissingConfigValue);
            }
            config_path = Some(PathBuf::from(value.as_os_str()));
        }

        config_path
            .map(|config_path| Self {
                config_path,
                check_only,
            })
            .ok_or(GatewayCommandError::MissingConfigOption)
    }

    /// Returns the operator-selected configuration path without normalizing or replacing it.
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Reports local-only preflight intent; callers must still load configuration and trust inputs.
    ///
    /// A true result requires the composition root to return before constructing a server. It does
    /// not grant network authority or replace validation during a later normal startup.
    pub fn check_only(&self) -> bool {
        self.check_only
    }

    /// Reads and validates the explicit configuration file before listeners are created.
    pub fn load_config(&self) -> Result<GatewayConfig, GatewayCommandError> {
        let source = fs::read_to_string(&self.config_path).map_err(|error| {
            GatewayCommandError::ReadConfig {
                path: self.config_path.clone(),
                kind: error.kind(),
            }
        })?;

        GatewayConfig::from_yaml(&source).map_err(|source| GatewayCommandError::InvalidConfig {
            path: self.config_path.clone(),
            source,
        })
    }
}

impl GatewayCommandError {
    /// Returns the explicit configuration path for file- or contract-related startup failures.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::ReadConfig { path, .. } | Self::InvalidConfig { path, .. } => Some(path),
            Self::MissingConfigOption
            | Self::MissingConfigValue
            | Self::DuplicateConfigOption
            | Self::UnexpectedArgument(_) => None,
        }
    }
}
