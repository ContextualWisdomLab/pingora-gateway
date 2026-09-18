//! Supplier RED for Pingora 0.9.0 Brotli response-stream finalization.
//!
//! The production gateway does not enable response compression. This fixture exercises Pingora's
//! public response-compression context directly so a supplier defect can be characterized without
//! adding a gateway compression capability or copying a supplier patch. A strict Node Brotli
//! decoder is the acceptance oracle because lenient clients can return the decoded payload even
//! when the Brotli stream is missing its terminal meta-block.

use std::io::Write;
use std::process::{Command, Stdio};

use bytes::Bytes;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::protocols::http::compression::ResponseCompressionCtx;

const STRICT_DECODER_SCRIPT: &str = r#"
const fs = require('fs');
const zlib = require('zlib');
const input = fs.readFileSync(0);
process.stdout.write(zlib.brotliDecompressSync(input));
"#;

fn pingora_brotli_response(body: &[u8]) -> Vec<u8> {
    let mut compression = ResponseCompressionCtx::new(6, false, false);
    let mut request = RequestHeader::build("GET", b"/brotli-characterization", None)
        .expect("literal request must be valid");
    request
        .insert_header("Accept-Encoding", "br")
        .expect("literal Accept-Encoding must be valid");
    compression.request_filter(&request);

    let mut response = ResponseHeader::build(200, None).expect("literal response must be valid");
    response
        .insert_header("Content-Type", "text/html; charset=utf-8")
        .expect("literal Content-Type must be valid");
    response
        .insert_header("Content-Length", body.len().to_string())
        .expect("body length must be a valid header value");
    compression.response_header_filter(&mut response, false);

    assert_eq!(
        response
            .headers
            .get("content-encoding")
            .expect("Pingora must select Brotli for this characterization")
            .as_bytes(),
        b"br"
    );
    assert!(
        response
            .headers
            .get_all("vary")
            .iter()
            .any(|value| value
                .as_bytes()
                .split(|byte| *byte == b',')
                .any(|token| token.trim_ascii().eq_ignore_ascii_case(b"accept-encoding"))),
        "compressed responses must vary on Accept-Encoding"
    );

    let split = body.len() / 2;
    let first = Bytes::copy_from_slice(&body[..split]);
    let second = Bytes::copy_from_slice(&body[split..]);
    let mut encoded = Vec::new();

    if let Some(chunk) = compression.response_body_filter(Some(&first), false) {
        encoded.extend_from_slice(&chunk);
    }
    if let Some(chunk) = compression.response_body_filter(Some(&second), true) {
        encoded.extend_from_slice(&chunk);
    }

    assert!(!encoded.is_empty(), "Brotli encoder must emit a body");
    encoded
}

fn strict_brotli_decode(encoded: &[u8]) -> std::process::Output {
    let mut child = Command::new("node")
        .args(["-e", STRICT_DECODER_SCRIPT])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Ubuntu CI contract requires the Node runtime used by GitHub Actions");

    {
        let mut stdin = child
            .stdin
            .take()
            .expect("strict decoder stdin must be piped");
        stdin
            .write_all(encoded)
            .expect("encoded Brotli bytes must be writable to the decoder");
    }

    child
        .wait_with_output()
        .expect("strict Brotli decoder process must be waitable")
}

#[test]
fn pingora_brotli_response_is_a_complete_stream_for_strict_decoders() {
    let body = b"<html><body>cwl pingora brotli finalization</body></html>".repeat(32);
    let encoded = pingora_brotli_response(&body);
    let decoded = strict_brotli_decode(&encoded);

    assert!(
        decoded.status.success(),
        "Pingora emitted Content-Encoding: br but the strict decoder rejected the complete response: {}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    assert_eq!(
        decoded.stdout, body,
        "a successful Brotli decode must reproduce the complete representation"
    );
}
