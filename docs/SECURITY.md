# Security

Treat configuration, DNS results, HTTP requests and forwarding metadata as untrusted. Configuration is validated before bind and contains only explicit static origins. Non-public DNS results require an explicit route opt-in. HTTPS uses certificate and hostname verification. Incoming forwarding metadata is discarded until a trusted-proxy contract exists. Header count/bytes, body bytes and upstream timeouts are bounded. CONNECT and arbitrary protocol upgrade behavior remain at Pingora safe defaults; WebSocket is not an advertised contract in this increment.

The container runs as UID/GID 65532 and the application does not require root-filesystem writes. Logs must never include credentials, Authorization, Cookie/Set-Cookie, query strings or raw request bodies. Metrics use no request-controlled labels.

Dependency audit is a release gate. Pingora 0.8.1 fixed critical request-smuggling/cache issues affecting older releases, but it predates a 2026-08 `lru` advisory remediation that landed on Pingora main. The current exact upstream pin is documented in ADR 0001 and must be revalidated against fresh releases/advisories before every release.
