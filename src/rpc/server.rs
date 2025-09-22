use axum::{
    extract::{Extension, Json, Path, WebSocketUpgrade},
    routing::{get, post},
    Router, response::IntoResponse, http::StatusCode,
    Json as AxumJson,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::rpc::handlers::{RpcHandler, RpcDeps};
use crate::rpc::auth::{AuthConfig, require_hmac};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;

/// JSON-RPC 2.0 request structure (simplified)
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: Option<Value>,
}

impl JsonRpcResponse {
    fn result(id: Option<Value>, v: Value) -> Self {
        Self { jsonrpc: "2.0".into(), result: Some(v), error: None, id }
    }
    fn error(id: Option<Value>, code: i32, message: &str) -> Self {
        Self { jsonrpc: "2.0".into(), result: None, error: Some(serde_json::json!({"code": code, "message": message})), id }
    }
}

/// RpcServer ties together the HTTP server and handler implementations.
pub struct RpcServer<D: RpcDeps> {
    addr: SocketAddr,
    deps: Arc<D>,
    auth: Arc<AuthConfig>,
}

impl<D: RpcDeps> RpcServer<D> {
    pub fn new(addr: SocketAddr, deps: Arc<D>, auth: AuthConfig) -> Self {
        Self { addr, deps, auth: Arc::new(auth) }
    }

    /// Construct router and spawn server (returns handle)
    pub async fn start(self) -> anyhow::Result<()> {
        let handler = RpcHandler::new(self.deps.clone());

        let rpc_handler = handler.clone();
        // build the Axum router
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .route("/metrics", get(metrics_handler))
            .route("/rpc", post(move |Json(payload): Json<Value>, Extension(rh): Extension<Arc<RpcHandler<D>>>| async move {
                json_rpc_endpoint(rh, payload).await
            }))
            .route("/block/:slot", get(move |Path(slot): Path<u64>, Extension(rh): Extension<Arc<RpcHandler<D>>>| async move {
                // simple block getter
                match rh.get_block(slot).await {
                    Ok(Some(b)) => AxumJson(serde_json::json!({"slot": slot, "block_hex": hex::encode(b)})).into_response(),
                    Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
                    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("err: {:?}", e)).into_response(),
                }
            }))
            .route("/account/:key", get({
                let rh = Arc::new(rpc_handler);
                move |Path(key): Path<String>, Extension(rh): Extension<Arc<RpcHandler<D>>>| async move {
                    match rh.get_account(key).await {
                        Ok(Some(acc)) => AxumJson(acc).into_response(),
                        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
                        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("err: {:?}", e)).into_response(),
                    }
                }
            }))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(Extension(Arc::new(handler)))
                    .layer(Extension(self.deps.clone()))
                    .layer(Extension(Arc::new(self.auth.clone())))
            );

        // Apply auth middleware as axum middleware for routes that need protection (example omitted)
        // For simplicity we skip global middleware wiring here; see auth::require_hmac for example usage.

        info!("Starting RPC server on {}", self.addr);
        axum::Server::bind(&self.addr).serve(app.into_make_service()).await?;
        Ok(())
    }
}

/// simple /metrics placeholder
async fn metrics_handler() -> &'static str {
    "# metrics\n"
}

/// JSON-RPC router: single endpoint POST /rpc
async fn json_rpc_endpoint<D: RpcDeps>(rh: Arc<RpcHandler<D>>, payload: Value) -> impl IntoResponse {
    // parse into JsonRpcRequest
    let req: Result<JsonRpcRequest, _> = serde_json::from_value(payload.clone());
    if let Err(_) = req {
        let resp = JsonRpcResponse::error(None, -32700, "Parse error");
        return AxumJson(resp).into_response();
    }
    let req = req.unwrap();
    let id = req.id.clone();

    // dispatch few known methods
    match req.method.as_str() {
        "status" => {
            match rh.status().await {
                Ok(v) => AxumJson(JsonRpcResponse::result(id, v)).into_response(),
                Err(e) => AxumJson(JsonRpcResponse::error(id, -32000, &format!("{:?}", e))).into_response(),
            }
        }
        "get_block" => {
            // params: [slot] or {slot: n}
            let slot = if let Some(params) = req.params {
                if params.is_array() {
                    params[0].as_u64().unwrap_or(0)
                } else if params.is_object() {
                    params.get("slot").and_then(|v| v.as_u64()).unwrap_or(0)
                } else { 0 }
            } else { 0 };
            match rh.get_block(slot).await {
                Ok(Some(data)) => AxumJson(JsonRpcResponse::result(id, serde_json::json!( { "slot": slot, "block_hex": hex::encode(data) } ))).into_response(),
                Ok(None) => AxumJson(JsonRpcResponse::error(id, -32602, "Block not found")).into_response(),
                Err(e) => AxumJson(JsonRpcResponse::error(id, -32001, &format!("{:?}", e))).into_response(),
            }
        }
        "submit_tx" => {
            // params: [tx_obj]
            if let Some(params) = req.params {
                let tx_val = if params.is_array() { params[0].clone() } else { params.clone() };
                match serde_json::from_value::<crate::txpool::pool::Tx>(tx_val) {
                    Ok(tx) => {
                        match rh.submit_tx(tx).await {
                            Ok(txres) => AxumJson(JsonRpcResponse::result(id, serde_json::json!(format!("{:?}", txres)))).into_response(),
                            Err(e) => AxumJson(JsonRpcResponse::error(id, -32002, &format!("{:?}", e))).into_response(),
                        }
                    }
                    Err(e) => AxumJson(JsonRpcResponse::error(id, -32602, &format!("invalid params: {:?}", e))).into_response(),
                }
            } else {
                AxumJson(JsonRpcResponse::error(id, -32602, "missing params")).into_response()
            }
        }
        _ => {
            AxumJson(JsonRpcResponse::error(id, -32601, "Method not found")).into_response()
        }
    }
}
