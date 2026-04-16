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

// storage/mod.rs - Persistent storage (sled embedded database)
use crate::chain::{Block, Transaction};
use crate::tokens::PNP16Token;
use crate::wallet::Wallet;
use serde::{de::DeserializeOwned, Serialize};

pub struct ChainStorage {
    db: sled::Db,
    blocks_tree: sled::Tree,
    txs_tree: sled::Tree,
    tokens_tree: sled::Tree,
    wallets_tree: sled::Tree,
    state_tree: sled::Tree,
}

impl ChainStorage {
    pub fn open(path: &str) -> Result<Self, sled::Error> {
        let db = sled::open(path)?;
        let blocks_tree = db.open_tree("blocks")?;
        let txs_tree = db.open_tree("transactions")?;
        let tokens_tree = db.open_tree("tokens")?;
        let wallets_tree = db.open_tree("wallets")?;
        let state_tree = db.open_tree("state")?;
        Ok(ChainStorage {
            db,
            blocks_tree,
            txs_tree,
            tokens_tree,
            wallets_tree,
            state_tree,
        })
    }

    pub fn in_memory() -> Result<Self, sled::Error> {
        let db = sled::Config::new().temporary(true).open()?;
        let blocks_tree = db.open_tree("blocks")?;
        let txs_tree = db.open_tree("transactions")?;
        let tokens_tree = db.open_tree("tokens")?;
        let wallets_tree = db.open_tree("wallets")?;
        let state_tree = db.open_tree("state")?;
        Ok(ChainStorage {
            db,
            blocks_tree,
            txs_tree,
            tokens_tree,
            wallets_tree,
            state_tree,
        })
    }

    fn encode<T: Serialize>(value: &T) -> Vec<u8> {
        serde_json::to_vec(value).expect("serialize failed")
    }

    fn decode<T: DeserializeOwned>(bytes: &sled::IVec) -> Option<T> {
        serde_json::from_slice(bytes).ok()
    }

    pub fn save_block(&self, block: &Block) -> Result<(), sled::Error> {
        let key = block.header.height.to_be_bytes();
        self.blocks_tree.insert(key, Self::encode(block))?;
        self.blocks_tree
            .insert(block.hash.as_bytes(), Self::encode(block))?;
        for tx in &block.transactions {
            self.txs_tree
                .insert(tx.tx_hash.as_bytes(), Self::encode(tx))?;
        }
        Ok(())
    }

    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        let key = height.to_be_bytes();
        self.blocks_tree
            .get(key)
            .ok()
            .flatten()
            .and_then(|b| Self::decode(&b))
    }

    pub fn get_block_by_hash(&self, hash: &str) -> Option<Block> {
        self.blocks_tree
            .get(hash.as_bytes())
            .ok()
            .flatten()
            .and_then(|b| Self::decode(&b))
    }

    pub fn get_latest_block(&self) -> Option<Block> {
        let (_, val) = self.blocks_tree.iter().next_back()?.ok()?;
        Self::decode(&val)
    }

    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        self.txs_tree
            .get(tx_hash.as_bytes())
            .ok()
            .flatten()
            .and_then(|b| Self::decode(&b))
    }

    pub fn save_token(&self, token: &PNP16Token) -> Result<(), sled::Error> {
        self.tokens_tree
            .insert(token.contract_address.as_bytes(), Self::encode(token))?;
        Ok(())
    }

    pub fn get_token(&self, contract_address: &str) -> Option<PNP16Token> {
        self.tokens_tree
            .get(contract_address.as_bytes())
            .ok()
            .flatten()
            .and_then(|b| Self::decode(&b))
    }

    pub fn save_wallet(&self, wallet: &Wallet) -> Result<(), sled::Error> {
        self.wallets_tree
            .insert(wallet.keypair.evm_address.as_bytes(), Self::encode(wallet))?;
        Ok(())
    }

    pub fn get_wallet(&self, address: &str) -> Option<Wallet> {
        self.wallets_tree
            .get(address.as_bytes())
            .ok()
            .flatten()
            .and_then(|b| Self::decode(&b))
    }

    pub fn set_state(&self, key: &str, value: &str) -> Result<(), sled::Error> {
        self.state_tree.insert(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_state(&self, key: &str) -> Option<String> {
        let bytes = self.state_tree.get(key.as_bytes()).ok().flatten()?;
        String::from_utf8(bytes.to_vec()).ok()
    }

    pub fn flush(&self) -> Result<(), sled::Error> {
        self.db.flush()?;
        Ok(())
    }
}
