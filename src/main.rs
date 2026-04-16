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

// main.rs - Pecu Novus Node Entry Point
mod chain;
mod consensus;
mod crypto;
mod escrow;
mod rpc;
mod storage;
mod tokens;
mod wallet;

use chain::{Blockchain, Transaction, TransactionType};
use consensus::{HalvingSchedule, ProofOfTime, Validator, VestingSchedule};
use escrow::MVault;
use rpc::{AppState, RpcServer};
use tokens::{AssetClass, PNP16Token, TokenRegistry};
use wallet::Wallet;

use chrono::Utc;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();
    print_banner();

    let state = AppState::new();
    seed_demo_data(&state);
    print_startup_summary(&state);

    let bc_clone = Arc::clone(&state.blockchain);
    let pot_clone = Arc::clone(&state.pot);

    // Background block producer
    tokio::spawn(async move {
        info!("Block producer started (PoT interval: 2s)");
        loop {
            sleep(Duration::from_secs(2)).await;
            let txs = bc_clone.drain_mempool(1000);
            if txs.is_empty() {
                continue;
            }
            let latest = bc_clone.latest_block();
            let seed = format!(
                "{}_{}",
                latest.hash,
                Utc::now().timestamp_nanos_opt().unwrap_or(0)
            );
            let (proof, validator_addr) = pot_clone.write().generate_pot_proof(&seed);
            let height = bc_clone.block_height() + 1;
            let block = chain::Block::new(height, &latest.hash, txs, &validator_addr, proof);
            let bh = block.hash[..16].to_string();
            let tc = block.transactions.len();
            match bc_clone.commit_block(block) {
                Ok(_) => info!(
                    "Block #{height} committed | {bh}... | {tc} txs | validator: {validator_addr}"
                ),
                Err(e) => warn!("Block commit failed: {e}"),
            }
        }
    });

    // Background validator reward issuer
    let pot_r = Arc::clone(&state.pot);
    let bc_r = Arc::clone(&state.blockchain);
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            let rewards = pot_r.write().issue_daily_rewards();
            if rewards.is_empty() {
                continue;
            }
            info!("Issuing {} validator rewards", rewards.len());
            for reward in &rewards {
                let nonce = bc_r.get_nonce("ValidatorRewardSystem");
                let tx = Transaction::new(
                    TransactionType::ValidatorReward,
                    "ValidatorRewardSystem",
                    &reward.validator_address,
                    reward.amount,
                    Some(format!("Validator reward block {}", reward.block_height)),
                    None,
                    false,
                    None,
                    None,
                    nonce,
                );
                let _ = bc_r.add_to_mempool(tx);
            }
        }
    });

    let port = std::env::var("PECU_RPC_PORT")
        .unwrap_or_else(|_| "8545".to_string())
        .parse::<u16>()
        .unwrap_or(8545);

    let server = RpcServer::new(state, port);
    server.run().await;
}

fn print_banner() {
    println!(
        "
  ╔══════════════════════════════════════════════════════════════════╗"
    );
    println!("  ║          PECU NOVUS BLOCKCHAIN  |  RUST IMPLEMENTATION           ║");
    println!("  ║  Consensus: Hybrid Proof of Time (PoT) + Proof of Stake (PoS)   ║");
    println!("  ║  110,000+ TPS | PNP16 + ERC-20 + ERC-1400 | EVM Compatible      ║");
    println!("  ║  Pecu 3.0 Themis | Chain ID: 27272727 | Carbon Neutral              ║");
    println!("  ║  Maintained by MegaHoot Technologies | Est. January 15, 2017    ║");
    println!(
        "  ╚══════════════════════════════════════════════════════════════════╝
"
    );
}

fn print_startup_summary(state: &AppState) {
    let stats = state.blockchain.stats();
    info!(
        "Chain Height: {} | Validators: {} | Tokens: {} | Wallets: {}",
        stats.block_height,
        state.pot.read().validators.len(),
        state.token_registry.read().list_tokens().len(),
        state.wallets.read().len()
    );
    info!("Halving schedule: 2017→20M, 2027→10M, 2037→5M, 2047→2.5M, 2057→1.25M PECU/year");
}

fn seed_demo_data(state: &AppState) {
    let alice = Wallet::new();
    let bob = Wallet::new();
    let carol = Wallet::new();
    let alice_addr = alice.keypair.evm_address.clone();
    let bob_addr = bob.keypair.evm_address.clone();
    let carol_addr = carol.keypair.evm_address.clone();

    {
        let mut balances = state.blockchain.balances.write();
        balances.insert(alice_addr.clone(), 10_000_000_000_000_000_000u128);
        balances.insert(bob_addr.clone(), 5_000_000_000_000_000_000u128);
        balances.insert(carol_addr.clone(), 2_500_000_000_000_000_000u128);
    }

    {
        let mut wallets = state.wallets.write();
        wallets.insert(alice_addr.clone(), alice);
        wallets.insert(bob_addr.clone(), bob);
        wallets.insert(carol_addr.clone(), carol);
    }

    {
        let mut pot = state.pot.write();
        pot.register_validator(Validator::new(&alice_addr, 1_000_000_000_000_000_000u128));
        pot.register_validator(Validator::new(&bob_addr, 500_000_000_000_000_000u128));
        pot.register_validator(Validator::new(&carol_addr, 250_000_000_000_000_000u128));
    }

    {
        let pecu_gold = PNP16Token::new(
            "PecuGold",
            "PGLD",
            18,
            1_000_000_000_000_000_000_000_000u128,
            Some(10_000_000_000_000_000_000_000_000u128),
            AssetClass::PhysicalCommodity,
            &alice_addr,
            "DAK_DEMO_001",
        );
        let pecu_realty = PNP16Token::new(
            "PecuRealty",
            "PRTY",
            6,
            100_000_000_000u128,
            None,
            AssetClass::FractionalRealEstate,
            &bob_addr,
            "DAK_DEMO_002",
        );
        let mut registry = state.token_registry.write();
        let ga = registry.deploy_pnp16(pecu_gold);
        let ra = registry.deploy_pnp16(pecu_realty);
        info!("PecuGold (PGLD) deployed at {}", &ga[..20]);
        info!("PecuRealty (PRTY) deployed at {}", &ra[..20]);
    }

    {
        let nonce_a = state.blockchain.get_nonce(&alice_addr);
        let tx = Transaction::new(
            TransactionType::Transfer,
            &alice_addr,
            &bob_addr,
            1_000_000_000_000_000_000u128,
            Some("Demo transfer Alice->Bob".to_string()),
            None,
            false,
            None,
            None,
            nonce_a,
        );
        let _ = state.blockchain.add_to_mempool(tx);

        let nonce_b = state.blockchain.get_nonce(&bob_addr);
        let escrow_tx = Transaction::new(
            TransactionType::Escrow,
            &bob_addr,
            &carol_addr,
            500_000_000_000_000_000u128,
            Some("Real estate deposit".to_string()),
            Some("Property deposit escrow".to_string()),
            true,
            Some(Utc::now().timestamp() + 7 * 86400),
            None,
            nonce_b,
        );
        let _ = state.blockchain.add_to_mempool(escrow_tx);
    }

    info!("Demo data seeded: 3 wallets, 3 validators, 2 tokens, 2 pending txs");
}
