use cwl_pingora_gateway::{
    edge_contract::{GatewayConfigError, UpstreamConfig, UpstreamTimeouts},
    pingora_delivery::{build_peer, PeerBuildError},
};
use pingora::tls::x509::X509;
use pingora::upstreams::peer::ALPN;
use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::NamedTempFile;

fn upstream(tls: bool, sni: Option<&str>) -> UpstreamConfig {
    UpstreamConfig {
        name: "api".to_string(),
        address: "127.0.0.1:8443".parse::<SocketAddr>().unwrap(),
        tls,
        sni: sni.map(str::to_string),
        trust_bundle_file: None,
        timeouts: UpstreamTimeouts {
            connection_ms: 1_250,
            total_connection_ms: 2_500,
            read_ms: 7_500,
            write_ms: 6_500,
            idle_ms: 15_000,
        },
    }
}

#[test]
fn tls_peer_verifies_identity_and_uses_explicit_io_budgets() {
    let peer = build_peer(&upstream(true, Some("api.internal.example")))
        .expect("validated TLS upstream must build a peer");

    assert!(peer.is_tls());
    assert_eq!(peer.sni, "api.internal.example");
    assert!(peer.options.verify_cert);
    assert!(peer.options.verify_hostname);
    assert_eq!(peer.options.alpn, ALPN::H1);
    assert!(peer.options.ca.is_none());
    assert_eq!(
        peer.options.connection_timeout,
        Some(Duration::from_millis(1_250))
    );
    assert_eq!(
        peer.options.total_connection_timeout,
        Some(Duration::from_millis(2_500))
    );
    assert_eq!(
        peer.options.read_timeout,
        Some(Duration::from_millis(7_500))
    );
    assert_eq!(
        peer.options.write_timeout,
        Some(Duration::from_millis(6_500))
    );
    assert_eq!(
        peer.options.idle_timeout,
        Some(Duration::from_millis(15_000))
    );
    assert!(peer.options.http_upstream_request_policy.strip_hop_by_hop);
    assert!(
        peer.options
            .http_upstream_request_policy
            .strip_connection_nominated
    );
    assert!(
        peer.options
            .http_upstream_request_policy
            .reject_malformed_connection_nominations
    );
}

#[test]
fn valid_explicit_trust_bundle_is_loaded_into_peer_authority() {
    let mut bundle = NamedTempFile::new().expect("temporary trust bundle");
    let certificate = X509::builder()
        .expect("OpenSSL should allocate an X509 builder")
        .build();
    bundle
        .write_all(
            &certificate
                .to_pem()
                .expect("test certificate should serialize as PEM"),
        )
        .expect("valid trust bundle should be writable");

    let mut tls = upstream(true, Some("api.internal.example"));
    tls.trust_bundle_file = Some(bundle.path().to_path_buf());

    let peer = build_peer(&tls).expect("valid explicit trust bundle must build a peer");
    let configured_roots = peer
        .options
        .ca
        .as_ref()
        .expect("explicit trust bundle must replace the default CA source");

    assert_eq!(configured_roots.len(), 1);
}

#[test]
fn cleartext_peer_does_not_invent_a_tls_identity() {
    let peer =
        build_peer(&upstream(false, None)).expect("validated HTTP upstream must build a peer");

    assert!(!peer.is_tls());
    assert!(peer.sni.is_empty());
    assert_eq!(peer.options.alpn, ALPN::H1);
    assert!(peer.options.ca.is_none());
}

#[test]
fn direct_peer_construction_still_fails_closed_for_invalid_tls_identity() {
    let invalid = upstream(true, None);

    assert_eq!(
        build_peer(&invalid).unwrap_err(),
        PeerBuildError::InvalidConfiguration(GatewayConfigError::MissingTlsServerName {
            upstream_name: "api".to_string(),
        })
    );
}

#[test]
fn missing_explicit_trust_bundle_fails_before_connection_authority() {
    let mut tls = upstream(true, Some("api.internal.example"));
    tls.trust_bundle_file = Some(PathBuf::from("/definitely/missing/cwl-local-ca.pem"));

    assert!(matches!(
        build_peer(&tls).unwrap_err(),
        PeerBuildError::ReadTrustBundle { .. }
    ));
}

#[test]
fn malformed_explicit_trust_bundle_fails_closed() {
    let mut bundle = NamedTempFile::new().expect("temporary trust bundle");
    writeln!(
        bundle,
        "-----BEGIN CERTIFICATE-----\nnot-base64\n-----END CERTIFICATE-----"
    )
    .expect("malformed trust bundle should be writable");
    let mut tls = upstream(true, Some("api.internal.example"));
    tls.trust_bundle_file = Some(bundle.path().to_path_buf());

    assert!(matches!(
        build_peer(&tls).unwrap_err(),
        PeerBuildError::InvalidTrustBundle { .. }
    ));
}

#[test]
fn empty_explicit_trust_bundle_fails_closed() {
    let bundle = NamedTempFile::new().expect("temporary trust bundle");
    let mut tls = upstream(true, Some("api.internal.example"));
    tls.trust_bundle_file = Some(bundle.path().to_path_buf());

    assert!(matches!(
        build_peer(&tls).unwrap_err(),
        PeerBuildError::InvalidTrustBundle { .. }
    ));
}
