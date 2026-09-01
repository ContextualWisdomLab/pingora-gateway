//! Fail-closed process startup boundary for the shared gateway.
//!
//! Network authority is never activated from an implicit configuration path. Operators must pass
//! exactly one `--config <path>` argument, and the referenced file must both exist and satisfy the
//! versioned edge contract before the delivery layer may start listeners.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::edge_contract::{GatewayConfig, GatewayConfigError};

/// Parsed command-line authority required before gateway startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayCommand {
    config_path: PathBuf,
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
    /// The startup surface intentionally accepts no undocumented arguments.
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
    /// Parses process arguments and requires exactly one explicit configuration path.
    pub fn parse<I>(args: I) -> Result<Self, GatewayCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut arguments = args.into_iter();
        let _program_name = arguments.next();
        let mut config_path = None;

        while let Some(argument) = arguments.next() {
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
            config_path = Some(PathBuf::from(value));
        }

        config_path
            .map(|config_path| Self { config_path })
            .ok_or(GatewayCommandError::MissingConfigOption)
    }

    /// Returns the operator-selected configuration path without normalizing or replacing it.
    pub fn config_path(&self) -> &Path {
        &self.config_path
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
