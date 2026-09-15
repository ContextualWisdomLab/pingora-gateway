//! Transport-derived forwarding metadata for trusted edge-to-upstream requests.
//!
//! The ingress boundary never accepts request-controlled forwarding identity as authority. It
//! removes the complete legacy forwarding namespace first and then rebuilds the subset required by
//! characterized consumer behavior from the accepted downstream connection and original request
//! authority.

use std::net::{IpAddr, Ipv6Addr};

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
    /// Returns the forwarding-protocol token derived from the accepted downstream transport.
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    /// Returns the external default port used only when Host omits an explicit port.
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

    /// Removes request-controlled proxy authority and emits transport-derived compatibility fields.
    ///
    /// The whole case-insensitive `X-Forwarded-*` namespace is removed before the characterized
    /// subset is rebuilt. This prevents uncharacterized compatibility or identity fields such as
    /// `X-Forwarded-Prefix` and `X-Forwarded-Client-Cert` from crossing the edge trust boundary.
    /// `Forwarded` and `X-Real-IP` are handled separately because they are outside that namespace.
    /// Headers such as `X-Application-Context` remain ordinary application metadata.
    pub fn apply(&self, upstream_request: &mut RequestHeader) -> pingora::Result<()> {
        let x_forwarded_headers = upstream_request
            .headers
            .keys()
            .filter(|name| {
                name.as_str()
                    .as_bytes()
                    .get(..b"x-forwarded-".len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"x-forwarded-"))
            })
            .cloned()
            .collect::<Vec<_>>();

        upstream_request.remove_header("Forwarded");
        upstream_request.remove_header("X-Real-IP");
        for header in &x_forwarded_headers {
            upstream_request.remove_header(header);
        }

        let client_ip = self.client_ip.to_string();
        let downstream_port = self.downstream_port.to_string();
        let insertion_result = [
            ("X-Forwarded-For", client_ip.as_str()),
            ("X-Real-IP", client_ip.as_str()),
            ("X-Forwarded-Host", self.original_host.as_str()),
            ("X-Forwarded-Port", downstream_port.as_str()),
            ("X-Forwarded-Proto", self.scheme.as_str()),
        ]
        .into_iter()
        .try_for_each(|(name, value)| upstream_request.insert_header(name, value));
        insertion_result
    }
}

/// Resolves the external forwarding port from an RFC 9110 Host authority and downstream scheme.
///
/// `Host` adopts URI host syntax, so generic HTTP field validity is insufficient: userinfo,
/// path/query delimiters, malformed percent escapes, and non-IP bracket literals must fail before
/// the value can be promoted into trusted `X-Forwarded-Host` compatibility metadata.
fn authority_port(authority: &str, scheme: DownstreamScheme) -> pingora::Result<u16> {
    if authority.is_empty() {
        return Err(invalid_authority());
    }

    if let Some(rest) = authority.strip_prefix('[') {
        let closing = rest.find(']').ok_or_else(invalid_authority)?;
        let literal = &rest[..closing];
        if !is_valid_ip_literal(literal) {
            return Err(invalid_authority());
        }
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
        if host.contains(':') || !is_valid_reg_name(host) {
            return Err(invalid_authority());
        }
        return parse_port(port);
    }

    if !is_valid_reg_name(authority) {
        return Err(invalid_authority());
    }
    Ok(scheme.default_port())
}

/// Accepts bracket contents allowed by RFC 3986 `IP-literal` without treating arbitrary text as IP.
fn is_valid_ip_literal(literal: &str) -> bool {
    literal.parse::<Ipv6Addr>().is_ok() || is_valid_ipv_future(literal)
}

/// Validates the forward-compatible bracket-literal form from RFC 3986 without interpreting it.
fn is_valid_ipv_future(literal: &str) -> bool {
    let bytes = literal.as_bytes();
    if bytes
        .first()
        .is_none_or(|byte| !matches!(*byte, b'v' | b'V'))
    {
        return false;
    }

    let Some(dot) = bytes[1..]
        .iter()
        .position(u8::is_ascii_hexdigit)
        .and_then(|_| bytes[1..].iter().position(|byte| *byte == b'.'))
        .map(|position| position + 1)
    else {
        return false;
    };
    if dot == 1 || !bytes[1..dot].iter().all(u8::is_ascii_hexdigit) {
        return false;
    }
    let suffix = &bytes[dot + 1..];
    !suffix.is_empty()
        && suffix
            .iter()
            .all(|byte| is_unreserved(*byte) || is_sub_delim(*byte) || *byte == b':')
}

/// Validates RFC 3986 `reg-name`, including only well-formed percent-encoded octets.
fn is_valid_reg_name(host: &str) -> bool {
    let bytes = host.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if is_unreserved(byte) || is_sub_delim(byte) {
            index += 1;
            continue;
        }
        if byte == b'%'
            && bytes
                .get(index + 1..=index + 2)
                .is_some_and(|escape| escape.iter().all(u8::is_ascii_hexdigit))
        {
            index += 3;
            continue;
        }
        return false;
    }
    true
}

/// Returns whether an octet belongs to RFC 3986 `unreserved`.
fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// Returns whether an octet belongs to RFC 3986 `sub-delims`.
fn is_sub_delim(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// Parses an explicit RFC 3986 Host port and rejects zero as unsupported external authority.
fn parse_port(port: &str) -> pingora::Result<u16> {
    if port.is_empty() || !port.as_bytes().iter().all(u8::is_ascii_digit) {
        return Err(invalid_authority());
    }
    let parsed = port.parse::<u16>().map_err(|_| invalid_authority())?;
    if parsed == 0 {
        return Err(invalid_authority());
    }
    Ok(parsed)
}

/// Builds the stable fail-closed error used for malformed downstream Host authority.
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
    fn downstream_host_is_fallback_authority_when_upstream_host_is_absent() {
        let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
        let upstream = RequestHeader::build("GET", b"/", None).expect("fixture request is valid");
        let downstream = request_with_host("fallback.example:8080");

        let context = ForwardingContext::from_downstream_transport(
            Some(&client),
            &upstream,
            &downstream,
            DownstreamScheme::Http,
        )
        .expect("downstream Host must supply fallback authority");

        assert_eq!(context.original_host, "fallback.example:8080");
        assert_eq!(context.downstream_port, 8080);
    }

    #[test]
    fn missing_host_authority_fails_closed() {
        let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
        let request = RequestHeader::build("GET", b"/", None).expect("fixture request is valid");

        let error = ForwardingContext::from_downstream_transport(
            Some(&client),
            &request,
            &request,
            DownstreamScheme::Http,
        )
        .expect_err("missing Host authority must fail closed");

        assert_eq!(error.etype, ErrorType::HTTPStatus(400));
    }

    #[test]
    fn malformed_host_authority_fails_closed_through_transport_derivation() {
        let client = PingoraSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 49152)));
        let request = request_with_host("app.example:0");

        let error = ForwardingContext::from_downstream_transport(
            Some(&client),
            &request,
            &request,
            DownstreamScheme::Http,
        )
        .expect_err("invalid Host port must fail closed at the transport-derived boundary");

        assert_eq!(error.etype, ErrorType::HTTPStatus(400));
    }

    #[test]
    fn invalid_forwarding_field_value_fails_closed() {
        let mut request =
            RequestHeader::build("GET", b"/", None).expect("fixture request must be valid");
        let context = ForwardingContext::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "bad\nvalue".to_string(),
            80,
            DownstreamScheme::Http,
        );

        assert!(context.apply(&mut request).is_err());
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
        assert_eq!(
            authority_port("[v1.edge]:9443", DownstreamScheme::Https).unwrap(),
            9443
        );
        assert_eq!(
            authority_port("exa%6Dple.example", DownstreamScheme::Http).unwrap(),
            80
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
            "app[example",
            "app]example",
            "user@app.example",
            "app.example/path",
            "app.example?query",
            "app%2.example",
            "%zz.example",
            "[not-an-ip]",
            "[v.example]",
        ] {
            let error = authority_port(authority, DownstreamScheme::Http)
                .expect_err("malformed authority must not produce forwarding metadata");
            assert_eq!(error.etype, ErrorType::HTTPStatus(400), "{authority}");
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
