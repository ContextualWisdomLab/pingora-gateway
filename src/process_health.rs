//! Process-local liveness/readiness policy shared by Pingora delivery adapters.
//!
//! These endpoints report only that the gateway process is alive and able to serve through its
//! listener. They deliberately do not probe or claim readiness for consumer product dependencies.
//! Only payload-free GET/HEAD requests receive the privileged admission bypass; application-shaped
//! traffic that happens to reuse a health path remains subject to runtime-isolation capacity.

use pingora::prelude::{Error, ErrorType, RequestHeader, ResponseHeader, Session};

/// Stable process-local liveness endpoint.
pub const LIVENESS_PATH: &str = "/livez";
/// Stable process-local readiness endpoint reached through the Pingora serving path.
pub const READINESS_PATH: &str = "/readyz";

/// Classification of one request against the process-health privilege boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessHealthAction {
    /// The request is not addressed to a process-health path.
    NotHealth,
    /// A payload-free GET/HEAD may bypass application admission and receive the local response.
    Probe,
    /// A health path used with another method is not a process probe.
    RejectMethod,
    /// A health-path request declares framing that could carry a request body.
    RejectPayload,
}

fn has_request_body_framing(request: &RequestHeader) -> bool {
    if request.headers.contains_key("transfer-encoding") {
        return true;
    }

    let mut content_lengths = request.headers.get_all("content-length").iter();
    let Some(content_length) = content_lengths.next() else {
        return false;
    };
    if content_lengths.next().is_some() {
        return true;
    }
    !matches!(
        content_length
            .to_str()
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok()),
        Some(0)
    )
}

/// Classifies whether a request is entitled to the process-health admission bypass.
///
/// A privileged probe is deliberately narrow: exact `/livez` or `/readyz`, GET or HEAD, and no
/// request-body framing other than one valid `Content-Length: 0`. RFC 9110 defines HEAD as GET
/// without response content and requires general-purpose HTTP servers to support both methods.
/// `Transfer-Encoding`, duplicate or malformed content lengths, and positive lengths are
/// application-shaped traffic and therefore do not receive the bypass. Body framing is classified
/// before the method so a body-bearing request is rejected through the ordinary HTTP error path
/// rather than receiving a successful process-health response with unread bytes.
pub(crate) fn classify_process_health_request(request: &RequestHeader) -> ProcessHealthAction {
    if !matches!(request.uri.path(), LIVENESS_PATH | READINESS_PATH) {
        return ProcessHealthAction::NotHealth;
    }
    if has_request_body_framing(request) {
        return ProcessHealthAction::RejectPayload;
    }
    if !matches!(request.method.as_str(), "GET" | "HEAD") {
        return ProcessHealthAction::RejectMethod;
    }
    ProcessHealthAction::Probe
}

async fn respond_empty(
    session: &mut Session,
    status: u16,
    advertise_retrieval_methods: bool,
) -> pingora::Result<()> {
    let mut response = ResponseHeader::build(status, None)
        .expect("literal process-health response status must be valid");
    response
        .insert_header("Content-Length", "0")
        .expect("literal Content-Length response header must be valid");
    response
        .insert_header("Cache-Control", "no-store")
        .expect("literal Cache-Control response header must be valid");
    if advertise_retrieval_methods {
        response
            .insert_header("Allow", "GET, HEAD")
            .expect("literal Allow response header must be valid");
    }
    session.write_response_header(Box::new(response), true).await
}

/// Writes the payload-free local health response without contacting a consumer upstream.
pub(crate) async fn respond_healthy(session: &mut Session) -> pingora::Result<()> {
    respond_empty(session, 200, false).await
}

/// Rejects another method while advertising the supported process-health retrieval methods.
pub(crate) async fn respond_method_not_allowed(session: &mut Session) -> pingora::Result<()> {
    respond_empty(session, 405, true).await
}

/// Builds the stable fail-closed error for body-bearing traffic on a process-health path.
pub(crate) fn payload_too_large_error() -> Box<Error> {
    Error::explain(
        ErrorType::HTTPStatus(413),
        "process-health probes must not carry a request body",
    )
}

#[cfg(test)]
mod tests {
    use pingora::http::HeaderValue;
    use pingora::prelude::RequestHeader;

    use super::{classify_process_health_request, ProcessHealthAction};

    fn request(method: &str, path: &[u8]) -> RequestHeader {
        RequestHeader::build(method, path, None).expect("fixture request should be valid")
    }

    #[test]
    fn only_health_paths_are_classified_as_process_health() {
        assert_eq!(
            classify_process_health_request(&request("GET", b"/application-health")),
            ProcessHealthAction::NotHealth
        );
        for method in ["GET", "HEAD"] {
            assert_eq!(
                classify_process_health_request(&request(method, b"/livez")),
                ProcessHealthAction::Probe
            );
            assert_eq!(
                classify_process_health_request(&request(method, b"/readyz")),
                ProcessHealthAction::Probe
            );
        }
    }

    #[test]
    fn unsupported_health_methods_are_not_privileged_probes() {
        assert_eq!(
            classify_process_health_request(&request("POST", b"/livez")),
            ProcessHealthAction::RejectMethod
        );
        assert_eq!(
            classify_process_health_request(&request("DELETE", b"/readyz")),
            ProcessHealthAction::RejectMethod
        );
    }

    #[test]
    fn zero_content_length_remains_payload_free() {
        for method in ["GET", "HEAD"] {
            let mut probe = request(method, b"/readyz");
            probe
                .insert_header("Content-Length", "0")
                .expect("fixture content length should be valid");
            assert_eq!(
                classify_process_health_request(&probe),
                ProcessHealthAction::Probe
            );
        }
    }

    #[test]
    fn body_framing_is_not_entitled_to_health_bypass() {
        let mut positive_length = request("GET", b"/readyz");
        positive_length
            .insert_header("Content-Length", "1")
            .expect("fixture content length should be valid");
        assert_eq!(
            classify_process_health_request(&positive_length),
            ProcessHealthAction::RejectPayload
        );

        let mut transfer_encoding = request("HEAD", b"/livez");
        transfer_encoding
            .insert_header("Transfer-Encoding", "chunked")
            .expect("fixture transfer encoding should be valid");
        assert_eq!(
            classify_process_health_request(&transfer_encoding),
            ProcessHealthAction::RejectPayload
        );
    }

    #[test]
    fn body_framing_takes_precedence_over_method_rejection() {
        let mut post = request("POST", b"/readyz");
        post.insert_header("Content-Length", "1")
            .expect("fixture content length should be valid");
        assert_eq!(
            classify_process_health_request(&post),
            ProcessHealthAction::RejectPayload
        );
    }

    #[test]
    fn malformed_duplicate_or_non_text_content_length_is_not_a_probe() {
        let mut malformed = request("GET", b"/readyz");
        malformed
            .insert_header("Content-Length", "not-a-number")
            .expect("fixture header bytes should be valid");
        assert_eq!(
            classify_process_health_request(&malformed),
            ProcessHealthAction::RejectPayload
        );

        let mut non_text = request("HEAD", b"/readyz");
        non_text.headers.insert(
            "content-length",
            HeaderValue::from_bytes(b"\xff").expect("non-text header value should be representable"),
        );
        assert_eq!(
            classify_process_health_request(&non_text),
            ProcessHealthAction::RejectPayload
        );

        let mut duplicate = request("GET", b"/readyz");
        duplicate
            .append_header("Content-Length", "0")
            .expect("first fixture content length should be valid");
        duplicate
            .append_header("Content-Length", "0")
            .expect("second fixture content length should be valid");
        assert_eq!(
            classify_process_health_request(&duplicate),
            ProcessHealthAction::RejectPayload
        );
    }
}
