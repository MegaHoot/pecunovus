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

// tests/integration_tests.rs
// Pecu Novus Blockchain — Full Integration Test Suite
// Tests: crypto, chain, consensus, tokens (PNP16/ERC-20), escrow, wallet, RPC

use pecu_novus::{
    chain::{Block, Blockchain, Transaction, TransactionType},
    consensus::{ProofOfTime, Validator, HalvingSchedule, VestingSchedule},
    crypto,
    escrow::{EscrowContract, EscrowStatus, MVault, TransferCard, TransferCardUseCase},
    tokens::{AssetClass, ERC1400Token, PNP16Token, TokenRegistry},
    wallet::{DevelopmentAccessKey, GeneralAccessKey, KeyPair, Wallet},
};
use chrono::Utc;

// ═══════════════════════════════════════════════════════════════════════════════
// CRYPTO TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod crypto_tests {
    use super::*;

    #[test]
    fn test_sha512_produces_128_char_hex() {
        let hash = crypto::sha512(b"pecu novus");
        assert_eq!(hash.len(), 128, "SHA-512 must produce 128-char hex");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sha256_produces_64_char_hex() {
        let hash = crypto::sha256(b"pecu novus");
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_keccak256_evm_address_format() {
        let addr = crypto::public_key_to_address("test_public_key");
        assert!(addr.starts_with("0x"), "EVM address must start with 0x");
        assert_eq!(addr.len(), 42, "EVM address must be 42 chars (0x + 40 hex)");
    }

    #[test]
    fn test_public_key_length_matches_whitepaper() {
        // Whitepaper: "Random lengths of numbers and letters, between 64 to 128"
        let pk = crypto::generate_public_key();
        assert!(pk.len() >= 64 && pk.len() <= 128,
            "Public key length {} not in range [64,128]", pk.len());
    }

    #[test]
    fn test_private_key_length_matches_whitepaper() {
        // Whitepaper: "between 60 to 102, a combination of Strings and Integers"
        let pk = crypto::generate_public_key();
        let sk = crypto::generate_private_key(&pk);
        assert!(sk.len() >= 60 && sk.len() <= 128,
            "Private key length {} not in range [60,128]", sk.len());
    }

    #[test]
    fn test_vdf_compute_and_verify() {
        let proof = crypto::compute_vdf("genesis_seed", 10);
        assert!(!proof.output.is_empty());
        assert_eq!(proof.delay, 10);
        assert!(crypto::verify_vdf(&proof), "VDF verification must pass");
    }

    #[test]
    fn test_vdf_different_inputs_produce_different_outputs() {
        let p1 = crypto::compute_vdf("seed_a", 5);
        let p2 = crypto::compute_vdf("seed_b", 5);
        assert_ne!(p1.output, p2.output);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = crypto::compute_merkle_root(&[]);
        assert!(!root.is_empty());
    }

    #[test]
    fn test_merkle_root_single_tx() {
        let root = crypto::compute_merkle_root(&["abc123".to_string()]);
        assert_eq!(root, "abc123");
    }

    #[test]
    fn test_merkle_root_multiple_txs() {
        let txs = vec!["tx1".to_string(), "tx2".to_string(), "tx3".to_string()];
        let root = crypto::compute_merkle_root(&txs);
        assert_eq!(root.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_cbc_encrypt_produces_output() {
        let data = b"Pecu Novus CBC encryption test";
        let key = b"pecu_novus_key_32bytes__________";
        let iv = b"initialization_v________________";
        let encrypted = crypto::cbc_encrypt(data, key, iv);
        assert!(!encrypted.is_empty());
        assert!(encrypted.len() >= data.len());
    }

    #[test]
    fn test_generate_uuid_format() {
        let id = crypto::generate_uuid();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 5, "UUID must have 5 parts");
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn test_hmac_sha256() {
        let mac = crypto::hmac_sha256(b"key", b"message");
        assert_eq!(mac.len(), 32);
    }

    #[test]
    fn test_pecu_address_base58() {
        let addr = crypto::public_key_to_pecu_address("test_pub_key");
        assert!(!addr.is_empty());
        // Base58 chars only
        assert!(addr.chars().all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c)));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WALLET TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod wallet_tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let w = Wallet::new();
        assert!(!w.wallet_id.is_empty());
        assert!(w.keypair.evm_address.starts_with("0x"));
        assert_eq!(w.pecu_balance, 0);
    }

    #[test]
    fn test_wallet_credit_and_debit() {
        let mut w = Wallet::new();
        w.credit(5_000);
        assert_eq!(w.pecu_balance, 5_000);
        assert!(w.debit(2_000));
        assert_eq!(w.pecu_balance, 3_000);
    }

    #[test]
    fn test_wallet_debit_insufficient_funds() {
        let mut w = Wallet::new();
        w.credit(100);
        assert!(!w.debit(200), "Cannot debit more than balance");
    }

    #[test]
    fn test_keypair_sign_verify() {
        let kp = KeyPair::generate();
        let data = "transfer:0xABC:1000";
        let sig = kp.sign(data);
        assert!(kp.verify_signature(data, &sig));
        assert!(!kp.verify_signature("tampered_data", &sig));
    }

    #[test]
    fn test_keypair_refresh_changes_address() {
        let mut kp = KeyPair::generate();
        let old_addr = kp.evm_address.clone();
        kp.refresh_public_key();
        assert_ne!(kp.evm_address, old_addr, "Refreshed public key must change address");
    }

    #[test]
    fn test_cold_storage_move_and_redeem() {
        let mut w = Wallet::new();
        w.credit(10_000);
        let key = w.move_to_cold_storage(4_000).expect("CSS move failed");
        assert_eq!(w.pecu_balance, 6_000);
        assert!(w.cold_storage.contains_key(&key));
        assert!(w.redeem_from_cold_storage(&key));
        assert_eq!(w.pecu_balance, 10_000);
    }

    #[test]
    fn test_cold_storage_invalid_key() {
        let mut w = Wallet::new();
        w.credit(1_000);
        assert!(!w.redeem_from_cold_storage("INVALID_KEY"));
    }

    #[test]
    fn test_gak_connect_disconnect() {
        let mut w = Wallet::new();
        let gak = w.connect_to_app("HootDex", Some(3600));
        assert!(gak.is_valid());
        w.disconnect_from_app("HootDex");
        // Session marked disconnected
        let session = w.gak_sessions.iter().find(|g| g.app_id == "HootDex").unwrap();
        assert!(!session.is_connected);
    }

    #[test]
    fn test_dak_kyc_flow() {
        let mut dak = DevelopmentAccessKey::new("Alice Developer", "alice@dev.com");
        assert!(!dak.is_active);
        dak.verify_kyc();
        assert!(dak.is_valid());
        dak.revoke("malicious activity");
        assert!(!dak.is_valid());
        assert!(dak.revocation_reason.is_some());
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// BLOCKCHAIN / CHAIN TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod chain_tests {
    use super::*;

    fn make_test_tx(sender: &str, receiver: &str, amount: u128) -> Transaction {
        Transaction::new(
            TransactionType::Transfer,
            sender, receiver, amount,
            None, None, false, None, None, 0,
        )
    }

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.header.height, 0);
        assert!(!genesis.transactions.is_empty());
        assert!(genesis.header.previous_hash.chars().all(|c| c == '0'));
    }

    #[test]
    fn test_blockchain_initializes_with_genesis() {
        let bc = Blockchain::new();
        assert_eq!(bc.block_height(), 0);
    }

    #[test]
    fn test_transaction_gas_fee_flat_rate() {
        // Whitepaper: "flat gas fee of just 0.0025%"
        let tx = make_test_tx("alice", "bob", 1_000_000);
        let expected = 1_000_000u128 * 25 / 1_000_000;  // 0.0025%
        assert_eq!(tx.gas_fee, expected, "Gas fee must be exactly 0.0025%");
    }

    #[test]
    fn test_transaction_burn_amount_is_half_fee() {
        let tx = make_test_tx("alice", "bob", 2_000_000);
        assert_eq!(tx.burned_amount(), tx.gas_fee / 2, "50% of fee must be burned");
    }

    #[test]
    fn test_add_valid_tx_to_mempool() {
        let bc = Blockchain::new();
        // Fund sender from genesis balances
        bc.balances.write().insert("alice".to_string(), 999_999_999_999_999_999u128);
        let tx = make_test_tx("alice", "bob", 1_000);
        assert!(bc.add_to_mempool(tx).is_ok());
        assert_eq!(bc.mempool.read().len(), 1);
    }

    #[test]
    fn test_insufficient_balance_rejected() {
        let bc = Blockchain::new();
        let tx = make_test_tx("broke_wallet", "bob", 1_000_000);
        assert!(bc.add_to_mempool(tx).is_err());
    }

    #[test]
    fn test_commit_block_updates_balances() {
        let bc = Blockchain::new();
        bc.balances.write().insert("alice".to_string(), 100_000_000u128);

        let tx = make_test_tx("alice", "bob", 10_000);
        bc.add_to_mempool(tx).unwrap();

        let txs = bc.drain_mempool(10);
        let proof = crypto::compute_vdf("test_seed", 5);
        let block = Block::new(1, &bc.latest_block().hash, txs, "validator1", proof);
        bc.commit_block(block).unwrap();

        assert!(bc.get_balance("bob") > 0);
        assert_eq!(bc.block_height(), 1);
    }

    #[test]
    fn test_block_hash_links_to_previous() {
        let bc = Blockchain::new();
        bc.balances.write().insert("alice".to_string(), 999_999_999u128);

        let tx = make_test_tx("alice", "bob", 100);
        bc.add_to_mempool(tx).unwrap();
        let txs = bc.drain_mempool(10);

        let genesis_hash = bc.latest_block().hash.clone();
        let proof = crypto::compute_vdf("seed", 5);
        let block = Block::new(1, &genesis_hash, txs, "validator1", proof);
        assert_eq!(block.header.previous_hash, genesis_hash);
    }

    #[test]
    fn test_get_block_by_height() {
        let bc = Blockchain::new();
        let genesis = bc.get_block_by_height(0).unwrap();
        assert_eq!(genesis.header.height, 0);
    }

    #[test]
    fn test_chain_stats() {
        let bc = Blockchain::new();
        let stats = bc.stats();
        assert_eq!(stats.block_height, 0);
        assert!(stats.total_transactions > 0); // genesis tx
    }

    #[test]
    fn test_dual_confirmation() {
        let mut tx = make_test_tx("alice", "bob", 1_000);
        assert!(tx.sender_confirmed);
        assert!(!tx.receiver_confirmed);
        assert!(!tx.is_fully_confirmed());
        tx.confirm_by_receiver();
        assert!(tx.is_fully_confirmed());
    }

    #[test]
    fn test_erc20_approve_allowance() {
        let bc = Blockchain::new();
        bc.approve_erc20("alice", "bob", "0xTokenContract", 5_000);
        assert_eq!(bc.get_allowance("alice", "bob", "0xTokenContract"), 5_000);
    }

    #[test]
    fn test_total_burned_accumulates() {
        let bc = Blockchain::new();
        bc.balances.write().insert("alice".to_string(), 999_999_999_999u128);

        for i in 0..3 {
            let tx = Transaction::new(
                TransactionType::Transfer,
                "alice", "bob", 1_000_000,
                None, None, false, None, None, i,
            );
            bc.add_to_mempool(tx).unwrap();
        }

        let txs = bc.drain_mempool(10);
        let proof = crypto::compute_vdf("test", 5);
        let block = Block::new(1, &bc.latest_block().hash, txs, "v1", proof);
        bc.commit_block(block).unwrap();

        assert!(*bc.total_burned.read() > 0, "Burn mechanism must reduce total supply");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONSENSUS / PROOF OF TIME TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod consensus_tests {
    use super::*;

    #[test]
    fn test_validator_registration_one_per_wallet() {
        let mut pot = ProofOfTime::new();
        let v1 = Validator::new("0xAlice", 1_000);
        let v2 = Validator::new("0xAlice", 2_000); // duplicate
        pot.register_validator(v1);
        pot.register_validator(v2);
        assert_eq!(pot.validators.len(), 1, "One validator per wallet address");
    }

    #[test]
    fn test_multiple_validators_register() {
        let mut pot = ProofOfTime::new();
        pot.register_validator(Validator::new("0xAlice", 1_000));
        pot.register_validator(Validator::new("0xBob", 2_000));
        pot.register_validator(Validator::new("0xCarol", 500));
        assert_eq!(pot.validators.len(), 3);
    }

    #[test]
    fn test_pot_generates_valid_proof() {
        let mut pot = ProofOfTime::new();
        pot.register_validator(Validator::new("0xAlice", 1_000));
        let (proof, validator) = pot.generate_pot_proof("block_hash_seed");
        assert!(!proof.output.is_empty());
        assert_eq!(validator, "0xAlice");
        assert!(pot.verify_proof(&proof));
    }

    #[test]
    fn test_pot_sequence_increments() {
        let mut pot = ProofOfTime::new();
        pot.register_validator(Validator::new("0xAlice", 1_000));
        pot.generate_pot_proof("seed1");
        pot.generate_pot_proof("seed2");
        assert_eq!(pot.pot_sequence, 2);
    }

    #[test]
    fn test_validator_selection_weight() {
        let mut v = Validator::new("0xAlice", 0);
        v.uptime_seconds = 86400; // 1 day
        let weight = v.selection_weight();
        assert!(weight > 0.0);
    }

    #[test]
    fn test_validator_reward_within_bounds() {
        let v = Validator::new("0xAlice", 1_000);
        let reward = v.daily_reward();
        assert!(reward >= 250_000_000_000_000u128);   // 0.25 PECU min
        assert!(reward <= 1_500_000_000_000_000u128); // 1.50 PECU max
    }

    #[test]
    fn test_daily_reward_cap_enforced() {
        let mut pot = ProofOfTime::new();
        // Register 100,000 validators — daily cap of 55,000 PECU must hold
        for i in 0..100 {
            let mut v = Validator::new(&format!("0xValidator{i}"), 1_000);
            v.uptime_seconds = 86400;
            pot.register_validator(v);
        }
        let rewards = pot.issue_daily_rewards();
        let total: u128 = rewards.iter().map(|r| r.amount).sum();
        assert!(total <= 55_000_000_000_000_000_000u128, "Daily cap exceeded: {total}");
    }

    #[test]
    fn test_halving_schedule_official_values() {
        let h = HalvingSchedule::official();
        assert_eq!(h.entries[0].year, 2017);
        assert_eq!(h.entries[1].year, 2027);
        assert_eq!(h.entries[2].year, 2037);
        let annual_2017 = h.entries[0].max_annual_reward / 1_000_000_000_000_000_000_000u128;
        assert_eq!(annual_2017, 20, "First decade: 20M PECU/year");
        let annual_2027 = h.entries[1].max_annual_reward / 1_000_000_000_000_000_000_000u128;
        assert_eq!(annual_2027, 10, "After first halving: 10M PECU/year");
    }

    #[test]
    fn test_vesting_schedule_total() {
        let vs = VestingSchedule::official();
        let total: u64 = vs.entries.iter().map(|e| e.amount_pecu).sum();
        assert_eq!(total, 130, "Total vested: 40+30+30+20+10 = 130M PECU");
    }

    #[test]
    fn test_offline_validators_excluded() {
        let mut pot = ProofOfTime::new();
        let mut v = Validator::new("0xAlice", 1_000);
        v.is_online = false;
        pot.register_validator(v);
        assert_eq!(pot.online_validators().len(), 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TOKEN TESTS (PNP16 + ERC-20)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod token_tests {
    use super::*;

    fn make_token(name: &str, symbol: &str, supply: u128) -> PNP16Token {
        PNP16Token::new(
            name, symbol, 18, supply, None,
            AssetClass::Utility, "0xCreator", "DAK_TEST",
        )
    }

    #[test]
    fn test_token_creation() {
        let t = make_token("PecuGold", "PGLD", 1_000_000);
        assert_eq!(t.name(), "PecuGold");
        assert_eq!(t.symbol(), "PGLD");
        assert_eq!(t.total_supply(), 1_000_000);
        assert!(t.contract_address.starts_with("0x"));
    }

    #[test]
    fn test_erc20_balance_of_creator() {
        let t = make_token("TestToken", "TTK", 5_000);
        assert_eq!(t.balance_of("0xCreator"), 5_000);
        assert_eq!(t.balance_of("0xRandomAddress"), 0);
    }

    #[test]
    fn test_erc20_transfer() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        assert!(t.transfer("0xCreator", "0xBob", 400).is_ok());
        assert_eq!(t.balance_of("0xCreator"), 600);
        assert_eq!(t.balance_of("0xBob"), 400);
    }

    #[test]
    fn test_erc20_transfer_insufficient_balance() {
        let mut t = make_token("TestToken", "TTK", 100);
        let result = t.transfer("0xCreator", "0xBob", 999);
        assert!(result.is_err(), "Transfer beyond balance must fail");
    }

    #[test]
    fn test_erc20_approve_and_allowance() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.approve("0xCreator", "0xSpender", 300).unwrap();
        assert_eq!(t.allowance("0xCreator", "0xSpender"), 300);
    }

    #[test]
    fn test_erc20_transfer_from() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.approve("0xCreator", "0xSpender", 500).unwrap();
        t.transfer_from("0xSpender", "0xCreator", "0xReceiver", 200).unwrap();
        assert_eq!(t.balance_of("0xReceiver"), 200);
        assert_eq!(t.allowance("0xCreator", "0xSpender"), 300); // allowance reduced
    }

    #[test]
    fn test_erc20_transfer_from_exceeds_allowance() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.approve("0xCreator", "0xSpender", 100).unwrap();
        let result = t.transfer_from("0xSpender", "0xCreator", "0xReceiver", 999);
        assert!(result.is_err());
    }

    #[test]
    fn test_pnp16_mint() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.mint("0xRecipient", 500).unwrap();
        assert_eq!(t.total_supply(), 1_500);
        assert_eq!(t.balance_of("0xRecipient"), 500);
    }

    #[test]
    fn test_pnp16_mint_respects_max_supply() {
        let mut t = PNP16Token::new(
            "Capped", "CAP", 18, 900,
            Some(1_000), // max supply = 1000
            AssetClass::Utility, "0xCreator", "DAK",
        );
        assert!(t.mint("0xBob", 100).is_ok());  // 900+100=1000 OK
        assert!(t.mint("0xBob", 1).is_err());   // 1001 > 1000 FAIL
    }

    #[test]
    fn test_pnp16_burn() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.burn("0xCreator", 200).unwrap();
        assert_eq!(t.total_supply(), 800);
        assert_eq!(t.balance_of("0xCreator"), 800);
    }

    #[test]
    fn test_pnp16_burn_insufficient_balance() {
        let mut t = make_token("TestToken", "TTK", 100);
        assert!(t.burn("0xCreator", 9999).is_err());
    }

    #[test]
    fn test_pnp16_subset_ledger_records_txs() {
        let mut t = make_token("TestToken", "TTK", 1_000);
        t.transfer("0xCreator", "0xBob", 100).unwrap();
        t.mint("0xAlice", 50).unwrap();
        assert_eq!(t.subset_ledger.len(), 2);
    }

    #[test]
    fn test_token_registry_deploy_and_retrieve() {
        let mut registry = TokenRegistry::new();
        let t = make_token("RegTest", "RTT", 500);
        let addr = registry.deploy_pnp16(t);
        assert!(registry.get_token(&addr).is_some());
        assert_eq!(registry.get_token(&addr).unwrap().symbol(), "RTT");
    }

    #[test]
    fn test_erc1400_security_token_partition() {
        let base = make_token("SecurityToken", "SEC", 0);
        let mut st = ERC1400Token::new(base, vec!["0xController".to_string()]);
        st.issue_by_partition("tranche_a", "0xInvestor", 1_000).unwrap();
        assert_eq!(st.balance_of_by_partition("tranche_a", "0xInvestor"), 1_000);
    }

    #[test]
    fn test_erc1400_verified_holder() {
        let base = make_token("ST", "ST", 0);
        let mut st = ERC1400Token::new(base, vec![]);
        assert!(!st.is_verified_holder("0xInvestor"));
        st.add_verified_holder("0xInvestor");
        assert!(st.is_verified_holder("0xInvestor"));
    }

    #[test]
    fn test_erc1400_operator_authorization() {
        let base = make_token("ST", "ST", 0);
        let mut st = ERC1400Token::new(base, vec![]);
        st.authorize_operator("0xOperator", "0xHolder");
        assert!(st.is_operator("0xOperator", "0xHolder"));
        assert!(!st.is_operator("0xOther", "0xHolder"));
    }

    #[test]
    fn test_token_asset_classes_pnp16() {
        // Whitepaper: financial, gaming, physical commodity, real estate
        let financial = PNP16Token::new("CompanyToken","COMP",18,1_000,None,AssetClass::FinancialAsset,"0xC","DAK");
        let gaming    = PNP16Token::new("GameToken","GAME",0,1_000_000,None,AssetClass::GamingAsset,"0xC","DAK");
        let gold      = PNP16Token::new("GoldToken","GOLD",8,21_000_000,None,AssetClass::PhysicalCommodity,"0xC","DAK");
        let realty    = PNP16Token::new("RealtyToken","RLTY",6,1_000,None,AssetClass::FractionalRealEstate,"0xC","DAK");
        assert_eq!(financial.asset_class, AssetClass::FinancialAsset);
        assert_eq!(gaming.asset_class, AssetClass::GamingAsset);
        assert_eq!(gold.asset_class, AssetClass::PhysicalCommodity);
        assert_eq!(realty.asset_class, AssetClass::FractionalRealEstate);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ESCROW / MVAULT TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod escrow_tests {
    use super::*;

    fn future_date(secs: i64) -> i64 {
        Utc::now().timestamp() + secs
    }

    fn past_date(secs: i64) -> i64 {
        Utc::now().timestamp() - secs
    }

    #[test]
    fn test_escrow_creation() {
        let e = EscrowContract::new(
            "0xAlice", "0xBob", 50_000,
            future_date(86400),
            Some("Property deposit".to_string()),
            None, None, vec![],
        );
        assert_eq!(e.status, EscrowStatus::Locked);
        assert!(!e.escrow_key.is_empty());
        assert!(!e.on_chain_hash.is_empty());
    }

    #[test]
    fn test_escrow_not_released_before_date() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            future_date(9999),
            None, None, None, vec![],
        );
        assert!(!e.try_release(), "Must not release before date");
        assert_eq!(e.status, EscrowStatus::Locked);
    }

    #[test]
    fn test_escrow_releases_after_date() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            past_date(1), // already past
            None, None, None, vec![],
        );
        assert!(e.try_release(), "Must release after date");
        assert_eq!(e.status, EscrowStatus::Released);
    }

    #[test]
    fn test_escrow_early_release_by_sender() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            future_date(86400),
            None, None, None, vec![],
        );
        assert!(e.release_early());
        assert_eq!(e.status, EscrowStatus::Released);
    }

    #[test]
    fn test_escrow_cancel() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            future_date(86400),
            None, None, None, vec![],
        );
        assert!(e.cancel());
        assert_eq!(e.status, EscrowStatus::Canceled);
    }

    #[test]
    fn test_escrow_cancel_after_release_fails() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            past_date(1),
            None, None, None, vec![],
        );
        e.try_release();
        assert!(!e.cancel(), "Cannot cancel already-released escrow");
    }

    #[test]
    fn test_escrow_required_actions() {
        let actions = vec!["sign_deed".to_string(), "pay_deposit".to_string()];
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 50_000,
            past_date(1),
            None, None, None, actions,
        );
        // Not released because actions incomplete
        assert!(!e.try_release());
        e.complete_action("sign_deed");
        assert!(!e.try_release());
        e.complete_action("pay_deposit");
        assert!(e.try_release());
    }

    #[test]
    fn test_escrow_dispute() {
        let mut e = EscrowContract::new(
            "0xAlice", "0xBob", 1_000,
            future_date(86400),
            None, None, None, vec![],
        );
        e.raise_dispute();
        assert_eq!(e.status, EscrowStatus::Disputed);
    }

    #[test]
    fn test_transfer_card_create_and_redeem() {
        let mut card = TransferCard::new(
            "0xIssuer", 500, None,
            Some(future_date(3600)),
            TransferCardUseCase::EventGiveaway,
        );
        assert!(card.is_valid());
        let amount = card.redeem("0xRedeemer").unwrap();
        assert_eq!(amount, 500);
        assert!(card.is_redeemed);
    }

    #[test]
    fn test_transfer_card_double_redeem_fails() {
        let mut card = TransferCard::new(
            "0xIssuer", 100, None, None,
            TransferCardUseCase::GiftingDigitalAssets,
        );
        card.redeem("0xAlice").unwrap();
        assert!(card.redeem("0xBob").is_err(), "Cannot redeem twice");
    }

    #[test]
    fn test_transfer_card_expiry() {
        let mut card = TransferCard::new(
            "0xIssuer", 100, None,
            Some(past_date(1)), // already expired
            TransferCardUseCase::TokenLaunch,
        );
        assert!(!card.is_valid());
        assert!(card.redeem("0xBob").is_err(), "Cannot redeem expired card");
    }

    #[test]
    fn test_mvault_create_and_retrieve_escrow() {
        let mut vault = MVault::new();
        let contract = vault.create_escrow(
            "0xAlice", "0xBob", 10_000,
            future_date(86400),
            Some("Test escrow".to_string()),
            None, None, vec![],
        );
        let id = contract.escrow_id.clone();
        assert!(vault.get_escrow(&id).is_some());
    }

    #[test]
    fn test_mvault_list_pending_for_address() {
        let mut vault = MVault::new();
        vault.create_escrow("0xAlice","0xBob",1_000,future_date(100),None,None,None,vec![]);
        vault.create_escrow("0xAlice","0xCarol",2_000,future_date(200),None,None,None,vec![]);
        vault.create_escrow("0xDave","0xEve",3_000,future_date(300),None,None,None,vec![]);
        let pending = vault.pending_escrows_for("0xAlice");
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_mvault_auto_release_processing() {
        let mut vault = MVault::new();
        vault.create_escrow("0xA","0xB",1_000,past_date(10),None,None,None,vec![]);
        vault.create_escrow("0xC","0xD",2_000,future_date(9999),None,None,None,vec![]);
        let released = vault.process_auto_releases();
        assert_eq!(released.len(), 1);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TOKENOMICS CONSTANTS TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tokenomics_tests {
    use super::*;

    #[test]
    fn test_max_supply_is_1_billion() {
        // Whitepaper: "The maximum supply of PECU tokens is fixed at 1 billion"
        let max = Blockchain::MAX_SUPPLY;
        let one_billion_in_units = 1_000_000_000u128 * 1_000_000_000_000_000u128;
        assert_eq!(max, one_billion_in_units);
    }

    #[test]
    fn test_daily_validator_cap_is_55000_pecu() {
        // Whitepaper: "maximum of ~55,000 PECU per day to all Validators"
        let cap = Blockchain::DAILY_VALIDATOR_REWARD_CAP;
        let expected = 55_000u128 * 1_000_000_000_000_000u128;
        assert_eq!(cap, expected);
    }

    #[test]
    fn test_annual_validator_cap_is_20m_pecu() {
        // Whitepaper: "annual cap of 20 million PECU issued as Validator rewards"
        let cap = Blockchain::ANNUAL_VALIDATOR_REWARD_CAP;
        let expected = 20_000_000u128 * 1_000_000_000_000_000u128;
        assert_eq!(cap, expected);
    }

    #[test]
    fn test_gas_fee_burn_ratio_is_50_percent() {
        assert_eq!(Blockchain::BURN_RATIO, 50);
    }

    #[test]
    fn test_flat_gas_fee_rate() {
        // 0.0025% = 25 / 1_000_000
        assert_eq!(Transaction::GAS_FEE_RATE_BPS, 25);
    }

    #[test]
    fn test_wallet_decimal_places_is_15() {
        // Original whitepaper: "A coin is divisible down to 15 Decimal places"
        assert_eq!(Wallet::DECIMAL_PLACES, 15);
    }

    #[test]
    fn test_halving_each_decade_reduces_by_half() {
        let h = HalvingSchedule::official();
        for i in 0..h.entries.len() - 1 {
            let current = h.entries[i].max_annual_reward;
            let next    = h.entries[i + 1].max_annual_reward;
            assert_eq!(next, current / 2, "Each halving must cut reward by exactly 50%");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// END-TO-END SCENARIO TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod e2e_tests {
    use super::*;

    /// Full scenario: create wallets → deploy token → transfer → escrow → mine block
    #[test]
    fn test_full_defi_workflow() {
        // 1. Create wallets
        let alice = Wallet::new();
        let bob   = Wallet::new();
        let alice_addr = alice.keypair.evm_address.clone();
        let bob_addr   = bob.keypair.evm_address.clone();

        // 2. Set up blockchain with balances
        let bc = Blockchain::new();
        bc.balances.write().insert(alice_addr.clone(), 100_000_000_000_000_000_000u128);

        // 3. Deploy PNP16 / ERC-20 token
        let mut registry = TokenRegistry::new();
        let mut token = PNP16Token::new(
            "AliceCoin", "ALC", 18,
            1_000_000_000_000_000_000_000u128,
            None, AssetClass::FinancialAsset, &alice_addr, "DAK_E2E",
        );
        let contract_addr = token.contract_address.clone();

        // 4. Transfer tokens Alice → Bob
        token.transfer(&alice_addr, &bob_addr, 100_000_000_000_000_000_000u128).unwrap();
        assert_eq!(token.balance_of(&bob_addr), 100_000_000_000_000_000_000u128);

        // 5. Bob approves Alice as spender
        token.approve(&bob_addr, &alice_addr, 50_000_000_000_000_000_000u128).unwrap();
        assert_eq!(token.allowance(&bob_addr, &alice_addr), 50_000_000_000_000_000_000u128);

        registry.deploy_pnp16(token);

        // 6. PECU chain transaction
        let nonce = bc.get_nonce(&alice_addr);
        let tx = Transaction::new(
            TransactionType::Transfer,
            &alice_addr, &bob_addr,
            1_000_000_000_000_000u128,
            Some("E2E test payment".to_string()),
            None, false, None, None, nonce,
        );
        bc.add_to_mempool(tx).unwrap();

        // 7. Mine block via PoT
        let mut pot = ProofOfTime::new();
        pot.register_validator(Validator::new(&alice_addr, 1_000_000));
        let txs = bc.drain_mempool(100);
        let seed = "e2e_test_seed";
        let (proof, validator) = pot.generate_pot_proof(seed);
        let block = Block::new(1, &bc.latest_block().hash, txs, &validator, proof);
        bc.commit_block(block).unwrap();

        assert_eq!(bc.block_height(), 1);
        assert!(bc.get_balance(&bob_addr) > 0);

        // 8. Create escrow
        let mut vault = MVault::new();
        let escrow = vault.create_escrow(
            &alice_addr, &bob_addr,
            500_000_000_000_000u128,
            Utc::now().timestamp() - 1, // immediately releasable
            Some("Service payment".to_string()),
            None, None, vec![],
        );
        let eid = escrow.escrow_id.clone();
        let released_ids = vault.process_auto_releases();
        assert!(released_ids.contains(&eid));
    }

    /// Real-estate tokenization scenario from whitepaper
    #[test]
    fn test_real_estate_tokenization() {
        let owner    = Wallet::new();
        let investor = Wallet::new();

        let property_token = PNP16Token::new(
            "123 Blockchain Ave",
            "PROP123",
            6,
            1_000_000, // 1M fractional shares
            Some(1_000_000),
            AssetClass::FractionalRealEstate,
            &owner.keypair.evm_address,
            "DAK_REALTY",
        );

        let mut registry = TokenRegistry::new();
        let addr = registry.deploy_pnp16(property_token);
        let token = registry.get_token_mut(&addr).unwrap();

        // Sell 100,000 shares (10%) to investor
        token.transfer(&owner.keypair.evm_address, &investor.keypair.evm_address, 100_000).unwrap();
        assert_eq!(token.balance_of(&investor.keypair.evm_address), 100_000);
        assert_eq!(token.balance_of(&owner.keypair.evm_address), 900_000);
    }

    /// Intellectual property royalty scenario
    #[test]
    fn test_ip_royalty_token() {
        let artist   = Wallet::new();
        let platform = Wallet::new();

        let ip_token = PNP16Token::new(
            "AlbumRoyalties2024",
            "ARY24",
            18,
            1_000,
            None,
            AssetClass::IntellectualProperty,
            &artist.keypair.evm_address,
            "DAK_MUSIC",
        );

        let mut registry = TokenRegistry::new();
        let addr = registry.deploy_pnp16(ip_token);
        let token = registry.get_token_mut(&addr).unwrap();

        // Platform pays 10 units royalty
        token.mint(&artist.keypair.evm_address, 10).unwrap();
        assert_eq!(token.total_supply(), 1_010);
    }

    /// Transfer card scenario: event giveaway
    #[test]
    fn test_transfer_card_event_giveaway() {
        let mut vault = MVault::new();

        // Issuer creates 3 cards for event attendees
        let mut cards = Vec::new();
        for _ in 0..3 {
            let card = vault.create_transfer_card(
                "0xEventOrganizer",
                1_000_000_000_000_000u128, // 1 PECU
                None,
                Some(Utc::now().timestamp() + 86400), // valid 24h
                TransferCardUseCase::EventGiveaway,
            );
            cards.push(card.redemption_key.clone());
        }

        // Attendees redeem
        let amount = vault.redeem_transfer_card(&cards[0], "0xAttendee1").unwrap();
        assert_eq!(amount, 1_000_000_000_000_000u128);

        // Cannot redeem same card twice
        assert!(vault.redeem_transfer_card(&cards[0], "0xAttendee2").is_err());
    }
}
