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
fn malformed_host_authority_fails_closed_through_transport_derivation() {
    let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
    let mut request =
        RequestHeader::build("GET", b"/api", None).expect("fixture request must be valid");
    request
        .insert_header("Host", "app.example:0")
        .expect("malformed authority is still valid HTTP field data");

    let error = ForwardingContext::from_downstream_transport(
        Some(&client),
        &request,
        &request,
        DownstreamScheme::Http,
    )
    .expect_err("invalid Host port must fail closed at the transport-derived boundary");

    assert_eq!(error.etype, ErrorType::HTTPStatus(400));
}
