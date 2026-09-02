//! Process-local liveness/readiness response shared by Pingora delivery adapters.
//!
//! These endpoints report only that the gateway process is alive and able to serve through its
//! listener. They deliberately do not probe or claim readiness for consumer product dependencies.

use pingora::prelude::{ResponseHeader, Session};

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable process-local readiness endpoint reached through the Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

pub(crate) async fn respond_healthy(session: &mut Session) -> pingora::Result<()> {
    let mut response =
        ResponseHeader::build(200, None).expect("literal HTTP 200 response header must be valid");
    response
        .insert_header("Content-Length", "0")
        .expect("literal Content-Length response header must be valid");
    response
        .insert_header("Cache-Control", "no-store")
        .expect("literal Cache-Control response header must be valid");
    session.write_response_header(Box::new(response), true).await
}
