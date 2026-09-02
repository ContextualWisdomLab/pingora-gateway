//! Shared ContextualWisdomLab edge-runtime library.
//!
//! The library keeps versioned edge contracts independent from Pingora delivery types so
//! product repositories can integrate through stable configuration and deployment contracts.

#![deny(missing_docs)]

pub mod edge_contract;
pub mod edge_routing;
pub mod forwarding_policy;
pub mod gateway_proxy;
pub mod http_policy;
pub mod migration_admin;
pub mod migration_delivery;
pub mod migration_plan;
pub mod migration_proxy;
pub mod observability;
pub mod pingora_delivery;
pub mod runtime_isolation;
pub mod runtime_policy;
pub mod startup;
