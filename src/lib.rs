//! Shared ContextualWisdomLab edge-runtime library.
//!
//! The library keeps versioned edge contracts independent from Pingora delivery types so
//! product repositories can integrate through stable configuration and deployment contracts.

#![deny(missing_docs)]
#![cfg_attr(not(test), deny(clippy::missing_docs_in_private_items))]

pub mod edge_contract;
pub mod gateway_proxy;
pub mod logging_policy;
pub mod pingora_delivery;
pub mod runtime_policy;
pub mod startup;
