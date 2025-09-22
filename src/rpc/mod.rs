//! RPC module
//!
//! - JSON-RPC 2.0 endpoint at POST /rpc
//! - Diagnostic endpoints: /health, /metrics
//! - Simple HMAC auth middleware (optional)
//!
//! To integrate: create implementations of the `RpcDeps` trait (wrapping
//! consensus, ledger, txpool, state) and pass to `RpcServer::new()`.

pub mod server;
pub mod handlers;
pub mod auth;

pub use server::RpcServer;
pub use handlers::{RpcDeps, RpcHandler};
