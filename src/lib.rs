//! Reusable CWL Pingora edge runtime.
//!
//! Domain routing types live in [`edge_routing`] and deliberately do not depend on
//! Pingora. Transport adaptation is isolated in [`delivery`].

pub mod delivery;
pub mod edge_routing;
pub mod runtime_configuration;
pub mod telemetry;
