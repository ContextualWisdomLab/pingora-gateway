use std::net::{IpAddr, Ipv6Addr};

use pingora::prelude::RequestHeader;

use crate::forwarding_policy::{DownstreamScheme, ForwardingContext};

#[test]
fn unit_compilation_sanitizes_client_controlled_forwarding_namespace() {
    let mut request =
        RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
    request
        .insert_header("X-Forwarded-For", "203.0.113.10")
        .expect("fixture forwarding header must be valid");
    request
        .insert_header("X-Forwarded-Prefix", "/spoofed")
        .expect("fixture compatibility header must be valid");
    let context = ForwardingContext::new(
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        "[::1]:8443".to_string(),
        8443,
        DownstreamScheme::Https,
    );

    context
        .apply(&mut request)
        .expect("trusted forwarding metadata must remain representable");

    assert_eq!(request.headers["x-forwarded-for"].to_str().unwrap(), "::1");
    assert!(request.headers.get("x-forwarded-prefix").is_none());
}
