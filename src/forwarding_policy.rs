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

    /// Derives forwarding authority from an accepted Pingora transport and request Host authority.
    ///
    /// The client and listener must both be IP sockets. Host is taken from the request that Pingora
    /// will send upstream when present, otherwise from the accepted downstream request. Missing,
    /// non-IP, or non-text authority fails closed before any compatibility forwarding field is
    /// emitted. This keeps Unix/custom transports and malformed Host bytes from being mistaken for
    /// pg-erd's characterized TCP edge authority.
    pub fn from_downstream_transport(
        client_addr: Option<&PingoraSocketAddr>,
        listener_addr: Option<&PingoraSocketAddr>,
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
        let downstream_port = listener_addr
            .and_then(PingoraSocketAddr::as_inet)
            .map(|address| address.port())
            .ok_or_else(|| {
                Error::explain(
                    ErrorType::HTTPStatus(500),
                    "pg-erd migration requires an IP downstream listener address",
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
        upstream_request
            .insert_header("X-Forwarded-For", client_ip.as_str())
            .expect("IP text and the static forwarding field name are valid HTTP header data");
        upstream_request
            .insert_header("X-Real-IP", client_ip.as_str())
            .expect("IP text and the static real-IP field name are valid HTTP header data");
        upstream_request.insert_header("X-Forwarded-Host", self.original_host.as_str())?;
        upstream_request
            .insert_header("X-Forwarded-Port", downstream_port.as_str())
            .expect("decimal TCP port text is valid HTTP header data");
        upstream_request
            .insert_header("X-Forwarded-Proto", self.scheme.as_str())
            .expect("the bounded downstream scheme is valid HTTP header data");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};

    use pingora::prelude::RequestHeader;

    use super::{DownstreamScheme, ForwardingContext};

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
}
