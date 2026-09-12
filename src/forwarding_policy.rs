//! Transport-derived forwarding metadata for trusted edge-to-upstream requests.
//!
//! The ingress boundary never accepts request-controlled forwarding identity as authority. It
//! removes legacy proxy fields first and then rebuilds the subset required by characterized
//! consumer behavior from the accepted downstream connection and original request authority.

use std::net::IpAddr;

use pingora::prelude::{Error, ErrorType, RequestHeader};
use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;

/// Scheme observed on the accepted downstream connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownstreamScheme {
    /// Clear-text HTTP transport.
    Http,
    /// TLS-terminated HTTPS transport.
    Https,
}

impl DownstreamScheme {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }
}

/// Trusted transport metadata used to reconstruct legacy forwarding fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardingContext {
    client_ip: IpAddr,
    original_host: String,
    downstream_port: u16,
    scheme: DownstreamScheme,
}

impl ForwardingContext {
    /// Creates forwarding metadata only from values already observed by the edge runtime.
    pub fn new(
        client_ip: IpAddr,
        original_host: String,
        downstream_port: u16,
        scheme: DownstreamScheme,
    ) -> Self {
        Self {
            client_ip,
            original_host,
            downstream_port,
            scheme,
        }
    }

    /// Derives forwarding identity from the accepted client socket and original request authority.
    ///
    /// The listener socket is intentionally not an input. Container, Service, NAT, or port-publish
    /// layers may expose a different external authority than the process bind address. An explicit
    /// Host port is therefore authoritative for `X-Forwarded-Port`; otherwise the admitted
    /// downstream scheme supplies its well-known port. Malformed authority fails closed.
    pub fn from_downstream_transport(
        client_addr: Option<&PingoraSocketAddr>,
        upstream_request: &RequestHeader,
        downstream_request: &RequestHeader,
        scheme: DownstreamScheme,
    ) -> pingora::Result<Self> {
        let client_ip = client_addr
            .and_then(PingoraSocketAddr::as_inet)
            .map(|address| address.ip())
            .ok_or_else(|| {
                Error::explain(
                    ErrorType::HTTPStatus(500),
                    "pg-erd migration requires an IP downstream client address",
                )
            })?;
        let original_host = upstream_request
            .headers
            .get("host")
            .or_else(|| downstream_request.headers.get("host"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| {
                Error::explain(
                    ErrorType::HTTPStatus(400),
                    "pg-erd migration requires a valid downstream Host authority",
                )
            })?;
        let downstream_port = authority_port(original_host.as_str(), scheme)?;

        Ok(Self::new(client_ip, original_host, downstream_port, scheme))
    }

    /// Removes request-controlled proxy identity and emits transport-derived compatibility fields.
    ///
    /// `Forwarded` is deliberately removed rather than synthesized: the characterized Traefik
    /// consumer contract relies on the legacy `X-Forwarded-*` family. `X-Forwarded-Server` is also
    /// removed because it identifies the proxy host itself and is not consumer authority; adding a
    /// fabricated server identity would create behavior that the Pingora runtime cannot prove.
    pub fn apply(&self, upstream_request: &mut RequestHeader) -> pingora::Result<()> {
        for header in [
            "Forwarded",
            "X-Forwarded-For",
            "X-Forwarded-Host",
            "X-Forwarded-Port",
            "X-Forwarded-Proto",
            "X-Forwarded-Server",
            "X-Real-IP",
        ] {
            upstream_request.remove_header(header);
        }

        let client_ip = self.client_ip.to_string();
        let downstream_port = self.downstream_port.to_string();
        upstream_request.insert_header("X-Forwarded-For", client_ip.as_str())?;
        upstream_request.insert_header("X-Real-IP", client_ip.as_str())?;
        upstream_request.insert_header("X-Forwarded-Host", self.original_host.as_str())?;
        upstream_request.insert_header("X-Forwarded-Port", downstream_port.as_str())?;
        upstream_request.insert_header("X-Forwarded-Proto", self.scheme.as_str())?;
        Ok(())
    }
}

fn authority_port(authority: &str, scheme: DownstreamScheme) -> pingora::Result<u16> {
    if authority.is_empty() {
        return Err(invalid_authority());
    }

    if let Some(rest) = authority.strip_prefix('[') {
        let closing = rest.find(']').ok_or_else(invalid_authority)?;
        let suffix = &rest[closing + 1..];
        return match suffix {
            "" => Ok(scheme.default_port()),
            _ if suffix.starts_with(':') => parse_port(&suffix[1..]),
            _ => Err(invalid_authority()),
        };
    }

    if authority.contains('[') || authority.contains(']') {
        return Err(invalid_authority());
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() || host.contains(':') {
            return Err(invalid_authority());
        }
        return parse_port(port);
    }

    Ok(scheme.default_port())
}

fn parse_port(port: &str) -> pingora::Result<u16> {
    let parsed = port.parse::<u16>().map_err(|_| invalid_authority())?;
    if parsed == 0 {
        return Err(invalid_authority());
    }
    Ok(parsed)
}

fn invalid_authority() -> Box<Error> {
    Error::explain(
        ErrorType::HTTPStatus(400),
        "pg-erd migration requires a valid downstream Host authority",
    )
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use pingora::prelude::{ErrorType, RequestHeader};
    use pingora::protocols::l4::socket::SocketAddr as PingoraSocketAddr;

    use super::{authority_port, DownstreamScheme, ForwardingContext};

    fn request_with_host(host: &str) -> RequestHeader {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        request
            .insert_header("host", host)
            .expect("fixture Host must be valid HTTP field data");
        request
    }

    #[test]
    fn ipv6_forwarding_uses_ip_without_socket_port() {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        let context = ForwardingContext::new(
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            "[::1]:8443".to_string(),
            8443,
            DownstreamScheme::Https,
        );

        context
            .apply(&mut request)
            .expect("literal transport metadata must produce valid headers");

        assert_eq!(request.headers["x-forwarded-for"].to_str().unwrap(), "::1");
        assert_eq!(request.headers["x-real-ip"].to_str().unwrap(), "::1");
    }

    #[test]
    fn host_authority_port_is_external_authority_not_listener_bind_port() {
        let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
        let downstream = request_with_host("app.example");
        let upstream = downstream.clone();
        let context = ForwardingContext::from_downstream_transport(
            Some(&client),
            &upstream,
            &downstream,
            DownstreamScheme::Http,
        )
        .expect("valid clear-text authority must derive forwarding metadata");
        let mut emitted = upstream.clone();

        context
            .apply(&mut emitted)
            .expect("derived forwarding metadata must remain representable");

        assert_eq!(emitted.headers["x-forwarded-host"], "app.example");
        assert_eq!(emitted.headers["x-forwarded-port"], "80");
        assert_eq!(emitted.headers["x-forwarded-proto"], "http");
    }

    #[test]
    fn explicit_and_ipv6_host_ports_are_preserved() {
        assert_eq!(
            authority_port("app.example:8080", DownstreamScheme::Http).unwrap(),
            8080
        );
        assert_eq!(
            authority_port("[2001:db8::1]:8443", DownstreamScheme::Https).unwrap(),
            8443
        );
        assert_eq!(
            authority_port("[2001:db8::1]", DownstreamScheme::Https).unwrap(),
            443
        );
    }

    #[test]
    fn malformed_host_authority_fails_closed() {
        for authority in [
            "",
            "app.example:",
            "app.example:0",
            "2001:db8::1",
            "[::1",
            "[::1]junk",
        ] {
            let error = authority_port(authority, DownstreamScheme::Http)
                .expect_err("malformed authority must not produce forwarding metadata");
            assert_eq!(error.etype, ErrorType::HTTPStatus(400));
        }
    }

    #[test]
    fn missing_ip_client_fails_closed() {
        let downstream = request_with_host("app.example:8080");
        let error = ForwardingContext::from_downstream_transport(
            None,
            &downstream,
            &downstream,
            DownstreamScheme::Http,
        )
        .expect_err("missing accepted IP client address must fail closed");
        assert_eq!(error.etype, ErrorType::HTTPStatus(500));
    }
}
