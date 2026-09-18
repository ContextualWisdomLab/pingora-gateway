use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use cwl_pingora_gateway::forwarding_policy::{DownstreamScheme, ForwardingContext};
use pingora::prelude::{ErrorType, RequestHeader};
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;

fn request_with_host(host: &str) -> RequestHeader {
    let mut request =
        RequestHeader::build("GET", b"/api", None).expect("fixture request must be valid");
    request
        .insert_header("Host", host)
        .expect("Host grammar may be invalid while HTTP field data remains valid");
    request
}

#[test]
fn public_boundary_covers_default_bracket_and_percent_encoded_authority() {
    let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));

    for (host, scheme, port) in [
        ("[::1]", DownstreamScheme::Https, 443),
        ("exa%6Dple.example", DownstreamScheme::Http, 80),
    ] {
        let request = request_with_host(host);
        let context =
            ForwardingContext::from_downstream_transport(Some(&client), &request, &request, scheme)
                .expect("admitted Host syntax must derive forwarding authority");

        assert_eq!(
            context,
            ForwardingContext::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                host.to_string(),
                port,
                scheme,
            ),
            "{host}"
        );
    }
}

#[test]
fn public_boundary_covers_remaining_malformed_authority_shapes() {
    let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));

    for host in [
        "",
        "[::1",
        "[::1]junk",
        "app[example",
        "app]example",
        ":8080",
    ] {
        let request = request_with_host(host);
        let error = ForwardingContext::from_downstream_transport(
            Some(&client),
            &request,
            &request,
            DownstreamScheme::Http,
        )
        .expect_err("malformed Host authority must fail closed at the public boundary");

        assert_eq!(error.etype, ErrorType::HTTPStatus(400), "{host}");
    }
}
