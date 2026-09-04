//! Transport-neutral HTTP response policy.
//!
//! This bounded context owns only edge-level HTTP response mutations that are explicitly captured
//! from a consumer's existing proxy contract. It does not own product authorization, application
//! response semantics, certificate authority, or security verdicts from Wardnet/EgressWeave.

use std::collections::HashSet;

use thiserror::Error;

/// One explicit response-header mutation owned by the edge HTTP-policy boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHeaderRule {
    /// HTTP field name as declared by the migration contract.
    pub name: String,
    /// Exact HTTP field value to emit.
    pub value: String,
}

/// Fail-closed response-header policy validation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HeaderPolicyError {
    /// At least one explicit response header is required to activate a policy.
    #[error("HTTP response policy requires at least one header")]
    NoHeaders,
    /// Header field names must fit the conservative alphanumeric-plus-hyphen migration profile.
    #[error("invalid HTTP response header name: {header_name}")]
    InvalidHeaderName {
        /// Invalid field name supplied by configuration.
        header_name: String,
    },
    /// One canonical field name may have only one edge-owned value in this contract.
    #[error("duplicate HTTP response header name: {header_name}")]
    DuplicateHeaderName {
        /// Lower-cased duplicate field name.
        header_name: String,
    },
    /// Active response policy does not admit an empty field value.
    #[error("HTTP response header {header_name} must have a non-empty value")]
    EmptyHeaderValue {
        /// Field whose configured value is empty after trimming optional whitespace.
        header_name: String,
    },
    /// CR/LF is rejected to prevent response-splitting/header-injection semantics.
    #[error("invalid HTTP response header value for {header_name}")]
    InvalidHeaderValue {
        /// Field whose configured value contains prohibited line breaks.
        header_name: String,
    },
}

/// Validated response-header policy independent from Pingora transport types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHeaderPolicy {
    headers: Vec<ResponseHeaderRule>,
}

impl ResponseHeaderPolicy {
    /// Validates an explicit response-header set before it can be attached to an edge route.
    ///
    /// Field-name uniqueness is ASCII case-insensitive as required by HTTP semantics. The current
    /// migration profile deliberately accepts only alphanumerics and `-`, which covers the captured
    /// consumer contracts while remaining a strict subset of legal HTTP field-name syntax. Values
    /// reject CR/LF so a migration contract cannot accidentally introduce response splitting.
    pub fn try_new(headers: Vec<ResponseHeaderRule>) -> Result<Self, HeaderPolicyError> {
        if headers.is_empty() {
            return Err(HeaderPolicyError::NoHeaders);
        }

        let mut names = HashSet::with_capacity(headers.len());
        for header in &headers {
            if !is_supported_header_name(&header.name) {
                return Err(HeaderPolicyError::InvalidHeaderName {
                    header_name: header.name.clone(),
                });
            }

            let normalized_name = header.name.to_ascii_lowercase();
            if !names.insert(normalized_name.clone()) {
                return Err(HeaderPolicyError::DuplicateHeaderName {
                    header_name: normalized_name,
                });
            }

            if header.value.trim().is_empty() {
                return Err(HeaderPolicyError::EmptyHeaderValue {
                    header_name: header.name.clone(),
                });
            }
            if header.value.contains('\r') || header.value.contains('\n') {
                return Err(HeaderPolicyError::InvalidHeaderValue {
                    header_name: header.name.clone(),
                });
            }
        }

        Ok(Self { headers })
    }

    /// Returns the configured value for a field name using ASCII case-insensitive HTTP matching.
    pub fn value_for(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }

    /// Returns the number of explicit response-header mutations in this policy.
    pub fn len(&self) -> usize {
        self.headers.len()
    }

    /// Returns whether this validated policy contains no response-header mutations.
    ///
    /// A successfully constructed policy is never empty; this method exists to make collection-like
    /// semantics explicit to callers without exposing mutable internal state.
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

fn is_supported_header_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
