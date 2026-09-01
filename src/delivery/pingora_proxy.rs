//! Pingora HTTP delivery adapter for the Edge Routing domain.

use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use http::{header, uri::Authority, Method};
use pingora_core::{
    upstreams::peer::HttpPeer,
    Error, ErrorType, Result,
};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use tracing::{info, warn};

use crate::{
    edge_routing::{Route, RouteTable, UpstreamScheme},
    runtime_configuration::Limits,
    telemetry::Metrics,
};

/// Pingora adapter. Routing decisions are delegated to the domain aggregate.
pub struct GatewayProxy {
    routes: Arc<RouteTable>,
    limits: Limits,
    metrics: Arc<Metrics>,
}

/// Request-scoped state containing no credentials or request bodies.
#[derive(Default)]
pub struct RequestContext {
    route: Option<Route>,
    body_bytes: u64,
    local_response: bool,
}

impl GatewayProxy {
    /// Construct the delivery adapter from validated domain/configuration state.
    pub fn new(routes: Arc<RouteTable>, limits: Limits, metrics: Arc<Metrics>) -> Self {
        Self { routes, limits, metrics }
    }

    async fn local_text(
        session: &mut Session,
        status: u16,
        content_type: &'static str,
        body: String,
    ) -> Result<()> {
        let bytes = Bytes::from(body);
        let is_head = session.req_header().method == Method::HEAD;
        let mut response = ResponseHeader::build(status, Some(3))?;
        response.insert_header(header::CONTENT_TYPE, content_type)?;
        response.insert_header(header::CACHE_CONTROL, "no-store")?;
        response.insert_header(header::CONTENT_LENGTH, bytes.len().to_string())?;
        session.write_response_header(Box::new(response), is_head || bytes.is_empty()).await?;
        if !is_head && !bytes.is_empty() {
            session.write_response_body(Some(bytes), true).await?;
        }
        Ok(())
    }

    fn reject(&self, status: u16, reason: &'static str) -> Box<Error> {
        self.metrics.rejected();
        Error::explain(ErrorType::HTTPStatus(status), reason)
    }
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        self.metrics.request();
        let req = session.req_header();
        let path = req.uri.path();

        if matches!(path, "/livez" | "/readyz" | "/metrics") {
            if !matches!(req.method, Method::GET | Method::HEAD) {
                return Err(self.reject(405, "health and metrics endpoints accept GET or HEAD only"));
            }
            let (content_type, body) = if path == "/metrics" {
                ("text/plain; version=0.0.4; charset=utf-8", self.metrics.render())
            } else {
                ("text/plain; charset=utf-8", "ok\n".to_owned())
            };
            Self::local_text(session, 200, content_type, body).await?;
            ctx.local_response = true;
            return Ok(true);
        }

        if req.headers.len() > self.limits.max_header_count {
            return Err(self.reject(431, "request header count exceeds configured limit"));
        }
        let header_bytes = req
            .headers
            .iter()
            .map(|(name, value)| name.as_str().len().saturating_add(value.as_bytes().len()))
            .sum::<usize>();
        if header_bytes > self.limits.max_header_bytes {
            return Err(self.reject(431, "request headers exceed configured byte limit"));
        }
        if let Some(value) = req.headers.get(header::CONTENT_LENGTH) {
            let length = value
                .to_str()
                .ok()
                .and_then(|text| text.parse::<u64>().ok())
                .ok_or_else(|| self.reject(400, "invalid Content-Length"))?;
            if length > self.limits.max_body_bytes {
                return Err(self.reject(413, "request body exceeds configured limit"));
            }
        }

        let host = req
            .headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| Authority::from_str(value).ok())
            .map(|authority| authority.host().trim_end_matches('.').to_ascii_lowercase());
        let route = self
            .routes
            .find(host.as_deref(), path)
            .cloned()
            .ok_or_else(|| self.reject(404, "no configured route matched the request"))?;
        ctx.route = Some(route);
        Ok(false)
    }

    async fn request_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(chunk) = body {
            ctx.body_bytes = ctx.body_bytes.saturating_add(chunk.len() as u64);
            if ctx.body_bytes > self.limits.max_body_bytes {
                return Err(self.reject(413, "streamed request body exceeds configured limit"));
            }
        }
        Ok(())
    }

    async fn upstream_peer(&self, _session: &mut Session, ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        let route = ctx
            .route
            .as_ref()
            .ok_or_else(|| Error::explain(ErrorType::HTTPStatus(500), "route context missing"))?;
        let tls = matches!(route.upstream.scheme, UpstreamScheme::Https);
        let mut peer = HttpPeer::new(route.upstream.socket, tls, route.upstream.host.clone());
        peer.options.connection_timeout = Some(self.limits.connect_timeout);
        peer.options.total_connection_timeout = Some(self.limits.connect_timeout);
        peer.options.read_timeout = Some(self.limits.read_timeout);
        peer.options.write_timeout = Some(self.limits.write_timeout);
        peer.options.verify_cert = true;
        peer.options.verify_hostname = true;
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        let route = ctx
            .route
            .as_ref()
            .ok_or_else(|| Error::explain(ErrorType::HTTPStatus(500), "route context missing"))?;
        for name in [
            "forwarded",
            "x-forwarded-for",
            "x-forwarded-host",
            "x-forwarded-proto",
            "x-forwarded-port",
            "x-real-ip",
        ] {
            upstream_request.remove_header(name);
        }
        upstream_request.insert_header(header::HOST, route.upstream.authority.as_str())?;
        Ok(())
    }

    async fn logging(&self, _session: &mut Session, error: Option<&Error>, ctx: &mut Self::CTX) {
        if ctx.local_response {
            return;
        }
        let route_id = ctx.route.as_ref().map_or("unmatched", |route| route.id.as_str());
        if let Some(error) = error {
            self.metrics.proxy_error();
            warn!(route_id, error_type = %error.etype(), "gateway request failed");
        } else {
            info!(route_id, "gateway request completed");
        }
    }
}
