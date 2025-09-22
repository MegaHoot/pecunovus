use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;
use tracing::warn;

/// Simple HMAC token-based auth middleware.
/// Add header `x-auth-token: <hex-hmac>` where hex-hmac = HMAC_SHA256(secret, path || body)
///
/// For production, use mTLS, JWT, or more robust schemes. This middleware demonstrates pluggable auth.

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct AuthConfig {
    pub enabled: bool,
    pub secret: Arc<Vec<u8>>,
}

impl AuthConfig {
    pub fn disabled() -> Self {
        Self { enabled: false, secret: Arc::new(vec![]) }
    }

    pub fn new(secret: Vec<u8>) -> Self {
        Self { enabled: true, secret: Arc::new(secret) }
    }
}

/// Validate header token against computed HMAC of path + body.
/// This middleware assumes the handler will run after reading request body; for streaming body you'd adapt.
pub async fn require_hmac<B>(
    auth: std::sync::Arc<AuthConfig>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    if !auth.enabled {
        return Ok(next.run(req).await);
    }

    // extract token header
    let headers = req.headers();
    let token_header = headers.get("x-auth-token");
    if token_header.is_none() {
        warn!("missing auth header");
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token = token_header.unwrap().to_str().unwrap_or("");
    // compute local HMAC over method + uri + content-length (we don't have body here)
    // For demo we only HMAC the path
    let path = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("");
    let mut mac = HmacSha256::new_from_slice(&auth.secret).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    mac.update(path.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if expected != token {
        warn!("invalid auth token");
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(next.run(req).await)
}
