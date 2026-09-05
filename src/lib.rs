//! Shared ContextualWisdomLab edge-runtime library.
//!
//! The library keeps versioned edge contracts independent from Pingora delivery types so
//! product repositories can integrate through stable configuration and deployment contracts.

#![deny(missing_docs)]

pub mod edge_contract;
pub mod gateway_proxy;
pub mod pingora_delivery;
pub mod runtime_policy;
pub mod startup;
