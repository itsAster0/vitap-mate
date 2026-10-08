//! MCP server for VTOP: Claude Code calls tools here; sessions come from
//! vtop-bridge, data from vtop-core. Never sees a password.

pub mod bridge;
pub mod config;
pub mod tools;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use bridge::SessionSource;
use config::Config;
use tools::{Caller, VtopTools};

/// Key checks answered by the bridge, kept for a while so each tool call
/// does not ask again. Lost on sleep, which costs one call.
struct KeyCache {
    source: Arc<dyn SessionSource>,
    ttl: Duration,
    known: Mutex<HashMap<String, Instant>>,
}

impl KeyCache {
    async fn check(&self, key: &str) -> Result<bool, String> {
        let hash = hex_sha256(key);
        let now = Instant::now();
        {
            let mut known = self
                .known
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            known.retain(|_, checked| now.duration_since(*checked) < self.ttl);
            if known.contains_key(&hash) {
                return Ok(true);
            }
        }
        let valid = self.source.whoami(key).await?.is_some();
        if valid {
            self.known
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(hash, now);
        }
        Ok(valid)
    }
}

fn hex_sha256(value: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn require_bearer(
    State(cache): State<Arc<KeyCache>>,
    mut request: Request,
    next: Next,
) -> Response {
    // The key comes from `Authorization: Bearer`, or from the path
    // (`/mcp/<key>`) for agents that only take a URL.
    let from_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|key| !key.is_empty());
    let from_path = request
        .uri()
        .path()
        .strip_prefix("/mcp/")
        .map(|rest| rest.split('/').next().unwrap_or_default().trim())
        .filter(|key| !key.is_empty());
    let presented = from_header.or(from_path).unwrap_or_default().to_string();
    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(serde_json::json!({ "error": "missing, unknown or revoked access key" })),
        )
            .into_response()
    };
    if presented.is_empty() {
        return unauthorized();
    }
    match cache.check(&presented).await {
        Ok(true) => {}
        Ok(false) => return unauthorized(),
        Err(error) => {
            tracing::warn!("key check failed: {error}");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "bridge unavailable" })),
            )
                .into_response();
        }
    }
    request.extensions_mut().insert(Caller(presented));
    next.run(request).await
}

/// Key checks are cached for five minutes.
pub fn router(config: &Config, source: Arc<dyn SessionSource>) -> Router {
    router_with_cache(config, source, Duration::from_secs(300))
}

pub fn router_with_cache(
    config: &Config,
    source: Arc<dyn SessionSource>,
    key_ttl: Duration,
) -> Router {
    let _ = config;
    let cache = Arc::new(KeyCache {
        source: source.clone(),
        ttl: key_ttl,
        known: Mutex::new(HashMap::new()),
    });
    // Stateless: nothing lives in memory between requests, so Railway's
    // app sleeping loses nothing. The bearer key guards the endpoint, so
    // the Host allow-list (meant for localhost servers) is off.
    let mcp_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .disable_allowed_hosts();
    let mcp = StreamableHttpService::new(
        move || Ok(VtopTools::new(source.clone())),
        Arc::new(NeverSessionManager::default()),
        mcp_config,
    );
    let protected = Router::new()
        .nest_service("/mcp", mcp)
        .layer(middleware::from_fn_with_state(cache, require_bearer));
    Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({ "status": "ok", "service": "vtop-mcp" })) }),
        )
        .merge(protected)
}
