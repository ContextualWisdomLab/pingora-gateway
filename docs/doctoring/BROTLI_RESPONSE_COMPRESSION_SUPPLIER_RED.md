# Brotli response-compression supplier RED

## Decision boundary

Issue #114 owns a fail-closed supplier/content-coding guard, not a request to enable response compression in the shared gateway. Production composition on the inherited #70 line does not gain `ResponseCompressionBuilder`, an operator compression knob, product MIME/cache policy, or any consumer-specific `Accept-Encoding` behavior in this lane.

This characterization inherits the released-supplier candidate `df5d2f05fc5fbdd94bbfb487283bf2a6d73a55bf`, where `Cargo.toml` pins crates.io `pingora = "=0.9.0"` and `pingora-prometheus = "=0.9.0"`. The resolved graph is therefore the CWL consumer graph rather than the standalone dependency set from the upstream reproducer. The test does not vendor an upstream fix or alter `Cargo.lock`.

## Finding

Cloudflare issue #1013 reports that Pingora's Brotli encoder handles `end == true` with `CompressorWriter::flush()` and then removes the accumulated output buffer. A flush emits pending data but does not finish the Brotli stream. Protected supplier `main@4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19` still contains that path, and the latest public release remains Pingora 0.9.0 (published 2026-09-09) with no release-qualified repair.

RFC 7932 §9.2 defines `ISLAST` in the meta-block header and states that an `ISLAST=1, ISLASTEMPTY=1` meta-block ends the Brotli stream. A representation advertised with HTTP `Content-Encoding: br` must therefore be a complete Brotli-coded representation, not merely a decodable prefix. RFC 9110 §§8.4 and 12.5.3 define `Content-Encoding` and `Accept-Encoding` as representation/content-coding semantics; successful decoding by a lenient client is not sufficient evidence that the selected content coding is valid.

## Executable RED contract

`tests/brotli_response_compression_supplier_red.rs` exercises Pingora's public `ResponseCompressionCtx` directly and remains outside production composition. It:

- negotiates `Accept-Encoding: br` against a compressible `text/html` response;
- requires Pingora to emit `Content-Encoding: br` and `Vary: Accept-Encoding`;
- streams the representation through two body chunks so the final call owns end-of-stream;
- passes the exact emitted bytes to Node's strict `zlib.brotliDecompressSync` decoder;
- requires decoder success and byte-for-byte equality with the original representation.

On an affected supplier this desired-behavior test is expected to be RED with a strict-decoder truncation/end-of-stream error. The Node process is test tooling on the repository's pinned `ubuntu-24.04` CI runner, not a runtime dependency or production implementation. If the runner image ceases to provide the runtime, that is an evidence-infrastructure failure and must not be reclassified as supplier GREEN.

This first RED is intentionally narrow. It proves stream-finalization correctness only. Future enablement still requires the broader #114 contract: consumer-owned legacy behavior characterization, multi-chunk/zero/short/large bodies, disconnect/backpressure, negotiation/framing/double-encoding semantics, CPU/memory bounds, exact-head gates, maintainer-integrated release authority, immutable gateway release, and consumer parity/shadow/canary/rollback/cutover.

## Promotion order

`#114 requirement → unchanged released-supplier RED → upstream maintainer disposition → release-qualified Pingora identity → unchanged strict-decoder GREEN → explicit versioned gateway capability only if a consumer needs it → immutable gateway release → consumer parity/shadow/canary/rollback/cutover`.

Do not append a handcrafted terminal byte, copy a mutable upstream patch, weaken the strict decoder, enable Brotli opportunistically, or treat gzip/zstd as defective without separate evidence.

## References

Alakuijala, J., & Szabadka, Z. (2016). *Brotli compressed data format* (RFC 7932). Internet Engineering Task Force. https://www.rfc-editor.org/rfc/rfc7932

Cloudflare, Inc. (2026, September 17). *Brotli response compression never finishes the stream (flush instead of finish), strict decoders reject every `br` body* (Issue #1013). GitHub. https://github.com/cloudflare/pingora/issues/1013

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). Internet Engineering Task Force. https://www.rfc-editor.org/rfc/rfc9110
