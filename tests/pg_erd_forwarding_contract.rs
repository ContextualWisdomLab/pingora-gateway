use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use cwl_pingora_gateway::forwarding_policy::{DownstreamScheme, ForwardingContext};
use pingora::prelude::{ErrorType, RequestHeader};
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;

#[test]
fn pg_erd_forwarding_rebuilds_transport_identity_instead_of_trusting_request_headers() {
    let mut request =
        RequestHeader::build("GET", b"/api", None).expect("fixture request must be valid");
    request
        .insert_header("Host", "app.example:8443")
        .expect("fixture host must be valid");
    for (name, value) in [
        ("Forwarded", "for=203.0.113.7;proto=http"),
        ("X-Forwarded-For", "203.0.113.7"),
        ("X-Forwarded-Host", "attacker.example"),
        ("X-Forwarded-Port", "80"),
        ("X-Forwarded-Proto", "http"),
        ("X-Forwarded-Server", "attacker-proxy"),
        ("X-Real-IP", "203.0.113.7"),
    ] {
        request
            .insert_header(name, value)
            .expect("fixture forwarding field must be valid");
    }

    ForwardingContext::new(
        IpAddr::V4(Ipv4Addr::new(198, 51, 100, 19)),
        "app.example:8443".to_string(),
        8443,
        DownstreamScheme::Https,
    )
    .apply(&mut request)
    .expect("transport-derived forwarding fields must be valid");

    assert!(request.headers.get("forwarded").is_none());
    assert_eq!(
        request.headers["x-forwarded-for"].to_str().unwrap(),
        "198.51.100.19"
    );
    assert_eq!(
        request.headers["x-real-ip"].to_str().unwrap(),
        "198.51.100.19"
    );
    assert_eq!(
        request.headers["x-forwarded-host"].to_str().unwrap(),
        "app.example:8443"
    );
    assert_eq!(
        request.headers["x-forwarded-port"].to_str().unwrap(),
        "8443"
    );
    assert_eq!(
        request.headers["x-forwarded-proto"].to_str().unwrap(),
        "https"
    );
    assert!(request.headers.get("x-forwarded-server").is_none());
}

#[test]
fn pg_erd_forwarding_derives_authority_from_admitted_transport_and_host_fallback() {
    let client = PingoraSocketAddr::Inet(SocketAddr::from(([198, 51, 100, 19], 51515)));
    let listener = PingoraSocketAddr::Inet(SocketAddr::from(([127, 0, 0, 1], 8443)));
    let upstream =
        RequestHeader::build("GET", b"/api", None).expect("fixture upstream request must be valid");
    let mut downstream = RequestHeader::build("GET", b"/api", None)
        .expect("fixture downstream request must be valid");
    downstream
        .insert_header("Host", "app.example:8443")
        .expect("fixture host must be valid");

    let context = ForwardingContext::from_downstream_transport(
        Some(&client),
        Some(&listener),
        &upstream,
        &downstream,
        DownstreamScheme::Https,
    )
    .expect("IP transport and fallback Host authority must be admitted");
    let mut emitted = upstream.clone();
    context
        .apply(&mut emitted)
        .expect("validated transport context must emit compatibility forwarding fields");

    assert_eq!(emitted.headers["x-forwarded-for"], "198.51.100.19");
    assert_eq!(emitted.headers["x-forwarded-host"], "app.example:8443");
    assert_eq!(emitted.headers["x-forwarded-port"], "8443");
    assert_eq!(emitted.headers["x-forwarded-proto"], "https");
}

#[test]
fn pg_erd_forwarding_transport_authority_fails_closed_when_required_evidence_is_missing() {
    let client = PingoraSocketAddr::Inet(SocketAddr::from(([198, 51, 100, 19], 51515)));
    let listener = PingoraSocketAddr::Inet(SocketAddr::from(([127, 0, 0, 1], 8080)));
    let upstream =
        RequestHeader::build("GET", b"/api", None).expect("fixture upstream request must be valid");
    let mut downstream = RequestHeader::build("GET", b"/api", None)
        .expect("fixture downstream request must be valid");
    downstream
        .insert_header("Host", "app.example:8080")
        .expect("fixture host must be valid");

    let missing_client = ForwardingContext::from_downstream_transport(
        None,
        Some(&listener),
        &upstream,
        &downstream,
        DownstreamScheme::Http,
    )
    .expect_err("missing downstream peer authority must fail closed");
    assert_eq!(missing_client.etype, ErrorType::HTTPStatus(500));

    let missing_listener = ForwardingContext::from_downstream_transport(
        Some(&client),
        None,
        &upstream,
        &downstream,
        DownstreamScheme::Http,
    )
    .expect_err("missing accepted-listener authority must fail closed");
    assert_eq!(missing_listener.etype, ErrorType::HTTPStatus(500));

    let no_host = RequestHeader::build("GET", b"/api", None)
        .expect("fixture request without Host must still be constructible");
    let missing_host = ForwardingContext::from_downstream_transport(
        Some(&client),
        Some(&listener),
        &no_host,
        &no_host,
        DownstreamScheme::Http,
    )
    .expect_err("missing downstream Host authority must fail closed");
    assert_eq!(missing_host.etype, ErrorType::HTTPStatus(400));
}

#[test]
fn forwarding_context_rejects_a_manually_supplied_invalid_host_value() {
    let mut request =
        RequestHeader::build("GET", b"/api", None).expect("fixture request must be valid");
    let context = ForwardingContext::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        "bad\nhost".to_string(),
        8080,
        DownstreamScheme::Http,
    );

    assert!(
        context.apply(&mut request).is_err(),
        "public forwarding context must not emit an invalid HTTP Host value"
    );
}
