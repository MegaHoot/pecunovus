// SPDX-License-Identifier: Apache-2.0
// Copyright 2017-2026 Pecu Novus Network / MegaHoot Technologies
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// rpc/mod.rs
// Pecu Novus JSON-RPC Server
// Implements:
//   - Standard EVM JSON-RPC methods (eth_* — ERC-20 compatibility)
//   - Pecu native methods (pecu_*)
//   - PNP16 token methods
//   - Escrow / MVault methods
//
// Compatible with MetaMask, Ethers.js, Web3.js via EVM methods.

use crate::chain::{Blockchain, Transaction, TransactionType};
use crate::consensus::ProofOfTime;
use crate::crypto;
use crate::escrow::MVault;
use crate::tokens::TokenRegistry;
use crate::wallet::Wallet;

use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

// ─── Shared App State ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub blockchain: Arc<Blockchain>,
    pub token_registry: Arc<RwLock<TokenRegistry>>,
    pub mvault: Arc<RwLock<MVault>>,
    pub pot: Arc<RwLock<ProofOfTime>>,
    pub wallets: Arc<RwLock<std::collections::HashMap<String, Wallet>>>,
    pub chain_id: u64,
    pub network_name: String,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            blockchain: Arc::new(Blockchain::new()),
            token_registry: Arc::new(RwLock::new(TokenRegistry::new())),
            mvault: Arc::new(RwLock::new(MVault::new())),
            pot: Arc::new(RwLock::new(ProofOfTime::new())),
            wallets: Arc::new(RwLock::new(std::collections::HashMap::new())),
            chain_id: 27272727, // Pecu Novus chain ID
            network_name: "Pecu Novus Mainnet".to_string(),
        }
    }
}

// ─── JSON-RPC Request / Response ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        RpcResponse {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }
    pub fn err(id: Option<Value>, code: i64, message: &str) -> Self {
        RpcResponse {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
            }),
            id,
        }
    }
}

// ─── RPC Server ───────────────────────────────────────────────────────────────

pub struct RpcServer {
    pub state: AppState,
    pub port: u16,
}

impl RpcServer {
    pub fn new(state: AppState, port: u16) -> Self {
        RpcServer { state, port }
    }

    pub async fn run(self) {
        let state = Arc::new(self.state);
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any);

        let app = Router::new()
            .route("/", post(handle_rpc))
            .route("/rpc", post(handle_rpc))
            .layer(cors)
            .with_state(state);

        let addr = format!("0.0.0.0:{}", self.port);
        info!("🚀 Pecu Novus RPC Server listening on http://{}", addr);
        info!("   Chain ID: 27272727 | Network: Pecu Novus Mainnet");
        info!("   EVM Compatible: eth_* methods available");
        info!("   Native: pecu_* | pnp16_* | escrow_* methods available");

        axum::Server::bind(&addr.parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}

// ─── Main RPC Handler ─────────────────────────────────────────────────────────

async fn handle_rpc(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RpcRequest>,
) -> (StatusCode, Json<RpcResponse>) {
    let id = req.id.clone();
    let params = req.params.clone().unwrap_or(json!([]));

    info!("RPC call: {}", req.method);

    let response = match req.method.as_str() {
        // ── EVM / Ethereum-compatible methods ─────────────────────────────────
        "eth_chainId" => eth_chain_id(&state, id),
        "net_version" => net_version(&state, id),
        "eth_blockNumber" => eth_block_number(&state, id),
        "eth_getBalance" => eth_get_balance(&state, id, &params),
        "eth_getBlockByNumber" => eth_get_block_by_number(&state, id, &params),
        "eth_getBlockByHash" => eth_get_block_by_hash(&state, id, &params),
        "eth_getTransactionByHash" => eth_get_tx_by_hash(&state, id, &params),
        "eth_sendRawTransaction" => eth_send_raw_transaction(&state, id, &params),
        "eth_call" => eth_call(&state, id, &params),
        "eth_gasPrice" => eth_gas_price(&state, id),
        "eth_estimateGas" => eth_estimate_gas(&state, id, &params),
        "eth_getTransactionCount" => eth_get_transaction_count(&state, id, &params),
        "eth_getLogs" => eth_get_logs(&state, id, &params),
        "web3_clientVersion" => web3_client_version(&state, id),
        "eth_syncing" => eth_syncing(&state, id),
        "eth_accounts" => eth_accounts(&state, id),

        // ── ERC-20 token calls (via eth_call ABI dispatch) ────────────────────
        "erc20_balanceOf" => erc20_balance_of(&state, id, &params),
        "erc20_transfer" => erc20_transfer(&state, id, &params),
        "erc20_approve" => erc20_approve(&state, id, &params),
        "erc20_allowance" => erc20_allowance(&state, id, &params),
        "erc20_transferFrom" => erc20_transfer_from(&state, id, &params),
        "erc20_totalSupply" => erc20_total_supply(&state, id, &params),

        // ── Pecu Novus native methods ─────────────────────────────────────────
        "pecu_getNetworkInfo" => pecu_get_network_info(&state, id),
        "pecu_getChainStats" => pecu_get_chain_stats(&state, id),
        "pecu_sendTransaction" => pecu_send_transaction(&state, id, &params),
        "pecu_getBalance" => pecu_get_balance(&state, id, &params),
        "pecu_createWallet" => pecu_create_wallet(&state, id),
        "pecu_getWallet" => pecu_get_wallet(&state, id, &params),
        "pecu_getValidators" => pecu_get_validators(&state, id),
        "pecu_registerValidator" => pecu_register_validator(&state, id, &params),
        "pecu_getHalvingSchedule" => pecu_get_halving_schedule(&state, id),
        "pecu_getVestingSchedule" => pecu_get_vesting_schedule(&state, id),
        "pecu_mineBlock" => pecu_mine_block(&state, id),
        "pecu_getTokenomics" => pecu_get_tokenomics(&state, id),

        // ── PNP16 token methods ───────────────────────────────────────────────
        "pnp16_deployToken" => pnp16_deploy_token(&state, id, &params),
        "pnp16_listTokens" => pnp16_list_tokens(&state, id),
        "pnp16_getToken" => pnp16_get_token(&state, id, &params),
        "pnp16_mint" => pnp16_mint(&state, id, &params),
        "pnp16_burn" => pnp16_burn(&state, id, &params),
        "pnp16_transfer" => pnp16_transfer(&state, id, &params),

        // ── Escrow / MVault methods ───────────────────────────────────────────
        "escrow_create" => escrow_create(&state, id, &params),
        "escrow_release" => escrow_release(&state, id, &params),
        "escrow_cancel" => escrow_cancel(&state, id, &params),
        "escrow_get" => escrow_get(&state, id, &params),
        "escrow_listByAddress" => escrow_list_by_address(&state, id, &params),
        "transfercard_create" => transfer_card_create(&state, id, &params),
        "transfercard_redeem" => transfer_card_redeem(&state, id, &params),

        // ── Cold storage ──────────────────────────────────────────────────────
        "css_moveToColdStorage" => css_move_to_cold_storage(&state, id, &params),
        "css_redeemColdStorage" => css_redeem_cold_storage(&state, id, &params),

        // ── Access Keys ───────────────────────────────────────────────────────
        "gak_connect" => gak_connect(&state, id, &params),
        "gak_disconnect" => gak_disconnect(&state, id, &params),
        "dak_register" => dak_register(&state, id, &params),
        "dak_verifyKyc" => dak_verify_kyc(&state, id, &params),

        method => RpcResponse::err(id, -32601, &format!("Method not found: {method}")),
    };

    (StatusCode::OK, Json(response))
}

// ─── EVM Methods ─────────────────────────────────────────────────────────────

fn eth_chain_id(state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(id, json!(format!("0x{:x}", state.chain_id)))
}

fn net_version(state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(id, json!(state.chain_id.to_string()))
}

fn eth_block_number(state: &AppState, id: Option<Value>) -> RpcResponse {
    let height = state.blockchain.block_height();
    RpcResponse::ok(id, json!(format!("0x{:x}", height)))
}

fn eth_get_balance(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("").to_lowercase();
    let balance = state.blockchain.get_balance(&address);
    // Return in hex Wei-equivalent
    RpcResponse::ok(id, json!(format!("0x{:x}", balance)))
}

fn eth_get_block_by_number(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let block_param = params[0].as_str().unwrap_or("latest");
    let height = if block_param == "latest" || block_param == "pending" {
        state.blockchain.block_height()
    } else {
        u64::from_str_radix(block_param.trim_start_matches("0x"), 16).unwrap_or(0)
    };

    match state.blockchain.get_block_by_height(height) {
        Some(block) => RpcResponse::ok(id, block_to_eth_json(&block)),
        None => RpcResponse::ok(id, Value::Null),
    }
}

fn eth_get_block_by_hash(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let hash = params[0].as_str().unwrap_or("");
    match state.blockchain.get_block_by_hash(hash) {
        Some(block) => RpcResponse::ok(id, block_to_eth_json(&block)),
        None => RpcResponse::ok(id, Value::Null),
    }
}

fn eth_get_tx_by_hash(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let hash = params[0].as_str().unwrap_or("");
    match state.blockchain.get_transaction(hash) {
        Some(tx) => RpcResponse::ok(id, tx_to_eth_json(&tx)),
        None => RpcResponse::ok(id, Value::Null),
    }
}

fn eth_send_raw_transaction(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    // Accept hex-encoded JSON transaction for compatibility
    let raw = params[0].as_str().unwrap_or("");
    let decoded = if let Ok(decoded) = hex::decode(raw.trim_start_matches("0x")) {
        decoded
    } else {
        return RpcResponse::err(id, -32602, "Invalid raw transaction");
    };

    if let Ok(tx) = serde_json::from_slice::<Transaction>(&decoded) {
        match state.blockchain.add_to_mempool(tx.clone()) {
            Ok(hash) => RpcResponse::ok(id, json!(hash)),
            Err(e) => RpcResponse::err(id, -32000, &e),
        }
    } else {
        RpcResponse::err(id, -32602, "Cannot decode transaction")
    }
}

fn eth_call(_state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    // Stub for EVM call — in full impl would execute Solidity/PVM
    let _to = params[0]["to"].as_str().unwrap_or("");
    let _data = params[0]["data"].as_str().unwrap_or("0x");
    RpcResponse::ok(id, json!("0x"))
}

fn eth_gas_price(_state: &AppState, id: Option<Value>) -> RpcResponse {
    // Flat fee rate: 0.0025% — return as gwei equivalent
    RpcResponse::ok(id, json!("0x3B9ACA00")) // 1 gwei symbolic
}

fn eth_estimate_gas(_state: &AppState, id: Option<Value>, _params: &Value) -> RpcResponse {
    // Pecu uses flat gas fee, so estimate is always the same
    RpcResponse::ok(id, json!("0x5208")) // 21000 standard
}

fn eth_get_transaction_count(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("").to_lowercase();
    let nonce = state.blockchain.get_nonce(&address);
    RpcResponse::ok(id, json!(format!("0x{:x}", nonce)))
}

fn eth_get_logs(_state: &AppState, id: Option<Value>, _params: &Value) -> RpcResponse {
    RpcResponse::ok(id, json!([]))
}

fn web3_client_version(_state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(id, json!("PecuNovus/v2.0.0-rust/Pecu3.0Themis"))
}

fn eth_syncing(_state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(id, json!(false))
}

fn eth_accounts(_state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(id, json!([]))
}

// ─── ERC-20 Methods ───────────────────────────────────────────────────────────

fn erc20_balance_of(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let address = params[1].as_str().unwrap_or("");
    let registry = state.token_registry.read();
    match registry.get_token(contract) {
        Some(t) => RpcResponse::ok(id, json!(t.balance_of(address).to_string())),
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn erc20_transfer(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let from = params[1].as_str().unwrap_or("");
    let to = params[2].as_str().unwrap_or("");
    let amount = params[3]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut registry = state.token_registry.write();
    match registry.get_token_mut(contract) {
        Some(t) => match t.transfer(from, to, amount) {
            Ok(r) => RpcResponse::ok(id, json!(r)),
            Err(e) => RpcResponse::err(id, -32000, &e),
        },
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn erc20_approve(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let owner = params[1].as_str().unwrap_or("");
    let spender = params[2].as_str().unwrap_or("");
    let amount = params[3]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut registry = state.token_registry.write();
    match registry.get_token_mut(contract) {
        Some(t) => match t.approve(owner, spender, amount) {
            Ok(r) => RpcResponse::ok(id, json!(r)),
            Err(e) => RpcResponse::err(id, -32000, &e),
        },
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn erc20_allowance(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let owner = params[1].as_str().unwrap_or("");
    let spender = params[2].as_str().unwrap_or("");

    let registry = state.token_registry.read();
    match registry.get_token(contract) {
        Some(t) => RpcResponse::ok(id, json!(t.allowance(owner, spender).to_string())),
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn erc20_transfer_from(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let spender = params[1].as_str().unwrap_or("");
    let from = params[2].as_str().unwrap_or("");
    let to = params[3].as_str().unwrap_or("");
    let amount = params[4]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut registry = state.token_registry.write();
    match registry.get_token_mut(contract) {
        Some(t) => match t.transfer_from(spender, from, to, amount) {
            Ok(r) => RpcResponse::ok(id, json!(r)),
            Err(e) => RpcResponse::err(id, -32000, &e),
        },
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn erc20_total_supply(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let registry = state.token_registry.read();
    match registry.get_token(contract) {
        Some(t) => RpcResponse::ok(id, json!(t.total_supply().to_string())),
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

// ─── Pecu Native Methods ─────────────────────────────────────────────────────

fn pecu_get_network_info(state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(
        id,
        json!({
            "network": "Pecu Novus Mainnet",
            "version": "Pecu 3.0 Themis",
            "chainId": state.chain_id,
            "consensus": "Hybrid PoT + PoS (BFT)",
            "tps": "110,000+",
            "maxSupply": "1,000,000,000 PECU",
            "carbonNeutral": true,
            "evmCompatible": true,
            "protocols": ["PNP16", "ERC-20", "ERC-1400"],
            "launched": "2017-01-15",
            "developers": ["L. Velazquez", "A. Bhardwaj"],
            "maintainer": "MegaHoot Technologies"
        }),
    )
}

fn pecu_get_chain_stats(state: &AppState, id: Option<Value>) -> RpcResponse {
    let stats = state.blockchain.stats();
    RpcResponse::ok(id, serde_json::to_value(stats).unwrap_or(json!({})))
}

fn pecu_send_transaction(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let sender = params[0].as_str().unwrap_or("");
    let receiver = params[1].as_str().unwrap_or("");
    let amount = params[2]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);
    let note = params[3].as_str().map(|s| s.to_string());
    let nonce = state.blockchain.get_nonce(sender);

    let tx = Transaction::new(
        TransactionType::Transfer,
        sender,
        receiver,
        amount,
        note,
        None,
        false,
        None,
        None,
        nonce,
    );
    let hash = tx.tx_hash.clone();

    match state.blockchain.add_to_mempool(tx) {
        Ok(_) => RpcResponse::ok(id, json!({ "txHash": hash, "status": "pending" })),
        Err(e) => RpcResponse::err(id, -32000, &e),
    }
}

fn pecu_get_balance(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let balance = state.blockchain.get_balance(address);
    let display = balance as f64 / 1_000_000_000_000_000f64;
    RpcResponse::ok(
        id,
        json!({
            "address": address,
            "balance_raw": balance.to_string(),
            "balance_pecu": format!("{:.15}", display),
            "unit": "PECU"
        }),
    )
}

fn pecu_create_wallet(state: &AppState, id: Option<Value>) -> RpcResponse {
    let wallet = Wallet::new();
    let info = json!({
        "walletId": wallet.wallet_id,
        "evmAddress": wallet.keypair.evm_address,
        "pecuAddress": wallet.keypair.pecu_address,
        "publicKey": wallet.keypair.public_key,
        "createdAt": wallet.created_at,
        "note": "Keep your private key secret. It will not be shown again."
    });
    state
        .wallets
        .write()
        .insert(wallet.keypair.evm_address.clone(), wallet);
    RpcResponse::ok(id, info)
}

fn pecu_get_wallet(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let wallets = state.wallets.read();
    match wallets.get(address) {
        Some(w) => RpcResponse::ok(
            id,
            json!({
                "walletId": w.wallet_id,
                "evmAddress": w.keypair.evm_address,
                "pecuAddress": w.keypair.pecu_address,
                "balance": w.pecu_balance.to_string(),
                "validatorNodeId": w.validator_node_id,
                "coldStorageKeys": w.cold_storage.len()
            }),
        ),
        None => RpcResponse::err(id, -32602, "Wallet not found"),
    }
}

fn pecu_get_validators(state: &AppState, id: Option<Value>) -> RpcResponse {
    let pot = state.pot.read();
    let validators: Vec<Value> = pot
        .validators
        .iter()
        .map(|v| {
            json!({
                "nodeId": v.node_id,
                "walletAddress": v.wallet_address,
                "stake": v.stake.to_string(),
                "uptimeSeconds": v.uptime_seconds,
                "blocksValidated": v.blocks_validated,
                "isOnline": v.is_online,
                "isLead": v.is_lead,
                "totalRewardsEarned": v.total_rewards_earned.to_string(),
                "selectionWeight": v.selection_weight()
            })
        })
        .collect();
    RpcResponse::ok(id, json!(validators))
}

fn pecu_register_validator(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    use crate::consensus::Validator;
    let address = params[0].as_str().unwrap_or("");
    let stake = params[1]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let validator = Validator::new(address, stake);
    let node_id = validator.node_id.clone();
    state.pot.write().register_validator(validator);

    // Update wallet if exists
    if let Some(w) = state.wallets.write().get_mut(address) {
        w.validator_node_id = Some(node_id.clone());
    }

    RpcResponse::ok(
        id,
        json!({
            "nodeId": node_id,
            "walletAddress": address,
            "stake": stake.to_string(),
            "status": "registered"
        }),
    )
}

fn pecu_get_halving_schedule(_state: &AppState, id: Option<Value>) -> RpcResponse {
    use crate::consensus::HalvingSchedule;
    let schedule = HalvingSchedule::official();
    let entries: Vec<Value> = schedule
        .entries
        .iter()
        .map(|e| {
            json!({
                "year": e.year,
                "maxAnnualRewardPecu": e.max_annual_reward / 1_000_000_000_000_000u128
            })
        })
        .collect();
    RpcResponse::ok(
        id,
        json!({
            "schedule": entries,
            "currentMaxAnnual": schedule.current_max_annual_reward() / 1_000_000_000_000_000u128,
            "note": "Rewards halve every decade. First halving: 2027."
        }),
    )
}

fn pecu_get_vesting_schedule(_state: &AppState, id: Option<Value>) -> RpcResponse {
    use crate::consensus::VestingSchedule;
    let schedule = VestingSchedule::official();
    let entries: Vec<Value> = schedule
        .entries
        .iter()
        .map(|e| {
            json!({
                "releaseYear": e.release_year,
                "amountMillionPecu": e.amount_pecu
            })
        })
        .collect();
    RpcResponse::ok(id, json!(entries))
}

fn pecu_mine_block(state: &AppState, id: Option<Value>) -> RpcResponse {
    use crate::chain::Block;

    let txs = state.blockchain.drain_mempool(1000);
    let latest = state.blockchain.latest_block();
    let seed = format!(
        "{}_{}",
        latest.hash,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let (pot_proof, validator_addr) = state.pot.write().generate_pot_proof(&seed);

    let height = state.blockchain.block_height() + 1;
    let block = Block::new(height, &latest.hash, txs, &validator_addr, pot_proof);
    let block_hash = block.hash.clone();
    let tx_count = block.transactions.len();

    match state.blockchain.commit_block(block) {
        Ok(_) => RpcResponse::ok(
            id,
            json!({
                "blockHash": block_hash,
                "height": height,
                "txCount": tx_count,
                "validator": validator_addr,
                "status": "committed"
            }),
        ),
        Err(e) => RpcResponse::err(id, -32000, &e),
    }
}

fn pecu_get_tokenomics(_state: &AppState, id: Option<Value>) -> RpcResponse {
    RpcResponse::ok(
        id,
        json!({
            "maxSupply": "1,000,000,000 PECU",
            "initialSupply": "200,000,000 PECU",
            "circulatingSupply": "~301,565,915 PECU (as of 04/2026)",
            "totalBurned": "~9,372,828 PECU (as of 04/2026)",
            "gasFeeRate": "0.0025% (flat)",
            "burnMechanism": "50% of all gas fees burned permanently",
            "validatorRewardRange": "0.25 - 1.50 PECU per 24h per node",
            "dailyValidatorCap": "55,000 PECU",
            "annualValidatorCap": "20,000,000 PECU",
            "halvingFrequency": "Every 10 years",
            "nextHalving": "2027",
            "allocationBreakdown": {
                "reserveFund": "46%",
                "founders": "15%",
                "teamMembers": "12%",
                "validators": "12%",
                "institutions": "15%"
            },
            "vestingSchedule": [
                {"year": 2026, "amountMillion": 40},
                {"year": 2028, "amountMillion": 30},
                {"year": 2030, "amountMillion": 30},
                {"year": 2032, "amountMillion": 20},
                {"year": 2034, "amountMillion": 10}
            ]
        }),
    )
}

// ─── PNP16 Methods ───────────────────────────────────────────────────────────

fn pnp16_deploy_token(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    use crate::tokens::{AssetClass, PNP16Token};

    let name = params[0].as_str().unwrap_or("MyToken");
    let symbol = params[1].as_str().unwrap_or("MTK");
    let decimals = params[2].as_u64().unwrap_or(18) as u8;
    let initial_supply = params[3]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);
    let creator = params[4].as_str().unwrap_or("");
    let dak = params[5].as_str().unwrap_or("");

    let token = PNP16Token::new(
        name,
        symbol,
        decimals,
        initial_supply,
        None,
        AssetClass::Utility,
        creator,
        dak,
    );
    let addr = token.contract_address.clone();

    let mut registry = state.token_registry.write();
    registry.deploy_pnp16(token);

    RpcResponse::ok(
        id,
        json!({
            "contractAddress": addr,
            "name": name,
            "symbol": symbol,
            "decimals": decimals,
            "initialSupply": initial_supply.to_string(),
            "standard": "PNP16 + ERC-20",
            "status": "deployed"
        }),
    )
}

fn pnp16_list_tokens(state: &AppState, id: Option<Value>) -> RpcResponse {
    let registry = state.token_registry.read();
    let tokens = registry.list_tokens();
    RpcResponse::ok(id, serde_json::to_value(tokens).unwrap_or(json!([])))
}

fn pnp16_get_token(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let registry = state.token_registry.read();
    match registry.get_token(contract) {
        Some(t) => RpcResponse::ok(
            id,
            json!({
                "contractAddress": t.contract_address,
                "name": t.name,
                "symbol": t.symbol,
                "decimals": t.decimals,
                "totalSupply": t.total_supply.to_string(),
                "creator": t.creator,
                "createdAt": t.created_at,
                "isPublic": t.is_public,
                "ledgerCount": t.subset_ledger.len()
            }),
        ),
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn pnp16_mint(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let to = params[1].as_str().unwrap_or("");
    let amount = params[2]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut registry = state.token_registry.write();
    match registry.get_token_mut(contract) {
        Some(t) => match t.mint(to, amount) {
            Ok(_) => RpcResponse::ok(
                id,
                json!({ "success": true, "newSupply": t.total_supply.to_string() }),
            ),
            Err(e) => RpcResponse::err(id, -32000, &e),
        },
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn pnp16_burn(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let contract = params[0].as_str().unwrap_or("");
    let from = params[1].as_str().unwrap_or("");
    let amount = params[2]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut registry = state.token_registry.write();
    match registry.get_token_mut(contract) {
        Some(t) => match t.burn(from, amount) {
            Ok(_) => RpcResponse::ok(
                id,
                json!({ "success": true, "newSupply": t.total_supply.to_string() }),
            ),
            Err(e) => RpcResponse::err(id, -32000, &e),
        },
        None => RpcResponse::err(id, -32602, "Token not found"),
    }
}

fn pnp16_transfer(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    erc20_transfer(state, id, params)
}

// ─── Escrow Methods ───────────────────────────────────────────────────────────

fn escrow_create(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let sender = params[0].as_str().unwrap_or("");
    let receiver = params[1].as_str().unwrap_or("");
    let amount = params[2]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);
    let release_date = params[3]
        .as_i64()
        .unwrap_or(chrono::Utc::now().timestamp() + 86400);
    let note = params[4].as_str().map(|s| s.to_string());
    let agreement = params[5].as_str().map(|s| s.to_string());

    let mut mvault = state.mvault.write();
    let contract = mvault.create_escrow(
        sender,
        receiver,
        amount,
        release_date,
        note,
        None,
        agreement,
        vec![],
    );

    RpcResponse::ok(
        id,
        json!({
            "escrowId": contract.escrow_id,
            "escrowKey": contract.escrow_key,
            "onChainHash": contract.on_chain_hash,
            "sender": contract.sender,
            "receiver": contract.receiver,
            "amount": contract.amount.to_string(),
            "releaseDate": contract.release_date,
            "status": "locked"
        }),
    )
}

fn escrow_release(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let escrow_id = params[0].as_str().unwrap_or("");
    let early = params[1].as_bool().unwrap_or(false);

    let mut mvault = state.mvault.write();
    match mvault.get_escrow_mut(escrow_id) {
        Some(e) => {
            let released = if early {
                e.release_early()
            } else {
                e.try_release()
            };
            RpcResponse::ok(
                id,
                json!({ "released": released, "status": format!("{:?}", e.status) }),
            )
        }
        None => RpcResponse::err(id, -32602, "Escrow not found"),
    }
}

fn escrow_cancel(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let escrow_id = params[0].as_str().unwrap_or("");
    let mut mvault = state.mvault.write();
    match mvault.get_escrow_mut(escrow_id) {
        Some(e) => {
            let canceled = e.cancel();
            RpcResponse::ok(
                id,
                json!({ "canceled": canceled, "status": format!("{:?}", e.status) }),
            )
        }
        None => RpcResponse::err(id, -32602, "Escrow not found"),
    }
}

fn escrow_get(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let escrow_id = params[0].as_str().unwrap_or("");
    let mvault = state.mvault.read();
    match mvault.get_escrow(escrow_id) {
        Some(e) => RpcResponse::ok(id, serde_json::to_value(e).unwrap_or(json!({}))),
        None => RpcResponse::err(id, -32602, "Escrow not found"),
    }
}

fn escrow_list_by_address(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let mvault = state.mvault.read();
    let escrows: Vec<Value> = mvault
        .pending_escrows_for(address)
        .iter()
        .map(|e| serde_json::to_value(e).unwrap_or(json!({})))
        .collect();
    RpcResponse::ok(id, json!(escrows))
}

fn transfer_card_create(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    use crate::escrow::TransferCardUseCase;

    let issuer = params[0].as_str().unwrap_or("");
    let amount = params[1]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);
    let token = params[2].as_str().map(|s| s.to_string());
    let expires_at = params[3].as_i64();
    let use_case_str = params[4].as_str().unwrap_or("Custom");
    let use_case = match use_case_str {
        "EventGiveaway" => TransferCardUseCase::EventGiveaway,
        "GiftingDigitalAssets" => TransferCardUseCase::GiftingDigitalAssets,
        "TokenLaunch" => TransferCardUseCase::TokenLaunch,
        "MarketingCampaign" => TransferCardUseCase::MarketingCampaign,
        other => TransferCardUseCase::Custom(other.to_string()),
    };

    let mut mvault = state.mvault.write();
    let card = mvault.create_transfer_card(issuer, amount, token, expires_at, use_case);

    RpcResponse::ok(
        id,
        json!({
            "cardId": card.card_id,
            "redemptionKey": card.redemption_key,
            "amount": card.amount.to_string(),
            "expiresAt": card.expires_at,
            "isValid": card.is_valid()
        }),
    )
}

fn transfer_card_redeem(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let key = params[0].as_str().unwrap_or("");
    let redeemer = params[1].as_str().unwrap_or("");

    let mut mvault = state.mvault.write();
    match mvault.redeem_transfer_card(key, redeemer) {
        Ok(amount) => RpcResponse::ok(
            id,
            json!({ "redeemed": true, "amount": amount.to_string() }),
        ),
        Err(e) => RpcResponse::err(id, -32000, &e),
    }
}

// ─── Cold Storage Methods ─────────────────────────────────────────────────────

fn css_move_to_cold_storage(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let amount = params[1]
        .as_str()
        .unwrap_or("0")
        .parse::<u128>()
        .unwrap_or(0);

    let mut wallets = state.wallets.write();
    match wallets.get_mut(address) {
        Some(w) => match w.move_to_cold_storage(amount) {
            Some(key) => RpcResponse::ok(
                id,
                json!({ "storageKey": key, "amount": amount.to_string() }),
            ),
            None => RpcResponse::err(id, -32000, "Insufficient balance"),
        },
        None => RpcResponse::err(id, -32602, "Wallet not found"),
    }
}

fn css_redeem_cold_storage(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let storage_key = params[1].as_str().unwrap_or("");

    let mut wallets = state.wallets.write();
    match wallets.get_mut(address) {
        Some(w) => {
            let ok = w.redeem_from_cold_storage(storage_key);
            RpcResponse::ok(id, json!({ "redeemed": ok }))
        }
        None => RpcResponse::err(id, -32602, "Wallet not found"),
    }
}

// ─── Access Key Methods ───────────────────────────────────────────────────────

fn gak_connect(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let app_id = params[1].as_str().unwrap_or("");
    let ttl = params[2].as_i64();

    let mut wallets = state.wallets.write();
    match wallets.get_mut(address) {
        Some(w) => {
            let gak = w.connect_to_app(app_id, ttl);
            RpcResponse::ok(id, json!({ "keyId": gak.key_id, "connected": true }))
        }
        None => RpcResponse::err(id, -32602, "Wallet not found"),
    }
}

fn gak_disconnect(state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let address = params[0].as_str().unwrap_or("");
    let app_id = params[1].as_str().unwrap_or("");

    let mut wallets = state.wallets.write();
    if let Some(w) = wallets.get_mut(address) {
        w.disconnect_from_app(app_id);
    }
    RpcResponse::ok(id, json!({ "disconnected": true }))
}

fn dak_register(_state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    use crate::wallet::DevelopmentAccessKey;

    let name = params[0].as_str().unwrap_or("Developer");
    let email = params[1].as_str().unwrap_or("dev@example.com");

    let dak = DevelopmentAccessKey::new(name, email);
    RpcResponse::ok(
        id,
        json!({
            "dakId": dak.dak_id,
            "developerName": dak.developer_name,
            "isKycVerified": dak.is_kyc_verified,
            "isActive": dak.is_active,
            "note": "KYC verification required to activate DAK"
        }),
    )
}

fn dak_verify_kyc(_state: &AppState, id: Option<Value>, params: &Value) -> RpcResponse {
    let dak_id = params[0].as_str().unwrap_or("");
    // In production: would check against KYC provider
    RpcResponse::ok(
        id,
        json!({
            "dakId": dak_id,
            "isKycVerified": true,
            "isActive": true,
            "note": "DAK activated. Developer can now deploy tokens and dApps."
        }),
    )
}

// ─── Helper: Convert Block to Ethereum JSON format ────────────────────────────

fn block_to_eth_json(block: &crate::chain::Block) -> Value {
    json!({
        "number": format!("0x{:x}", block.header.height),
        "hash": block.hash,
        "parentHash": block.header.previous_hash,
        "stateRoot": block.header.state_hash,
        "transactionsRoot": block.header.merkle_root,
        "miner": block.header.validator,
        "timestamp": format!("0x{:x}", block.header.timestamp),
        "transactions": block.transactions.iter().map(|t| t.tx_hash.clone()).collect::<Vec<_>>(),
        "gasUsed": "0x0",
        "gasLimit": "0xffffffff",
        "extraData": format!("PecuNovus-v{}", block.header.version),
        "potProof": {
            "output": block.header.pot_proof.output,
            "delay": block.header.pot_proof.delay,
            "sequenceCount": block.header.pot_proof.sequence_count
        }
    })
}

fn tx_to_eth_json(tx: &Transaction) -> Value {
    json!({
        "hash": tx.tx_hash,
        "from": tx.sender,
        "to": tx.receiver,
        "value": format!("0x{:x}", tx.amount),
        "gas": format!("0x{:x}", tx.gas_fee),
        "nonce": format!("0x{:x}", tx.nonce),
        "input": tx.call_data.clone().unwrap_or("0x".to_string()),
        "blockNumber": tx.block_height.map(|h| format!("0x{:x}", h)),
        "timestamp": tx.timestamp,
        "type": format!("{:?}", tx.tx_type),
        "note": tx.note,
        "isEscrow": tx.is_escrow,
        "escrowReleaseDate": tx.escrow_release_date,
        "senderConfirmed": tx.sender_confirmed,
        "receiverConfirmed": tx.receiver_confirmed
    })
}
