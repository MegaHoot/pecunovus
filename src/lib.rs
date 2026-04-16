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

// Pecu Novus Blockchain - Rust Implementation
// Pecu 2.0 / 3.0 Themis | PNP16 + ERC-20 Compatible
// Based on official whitepapers (2018, 2024) and pecu-rpc spec

pub mod chain;
pub mod consensus;
pub mod crypto;
pub mod escrow;
pub mod rpc;
pub mod storage;
pub mod tokens;
pub mod wallet;

pub use chain::{Block, BlockHeader, Blockchain, Transaction, TransactionType};
pub use consensus::{ProofOfTime, Validator, ValidatorReward};
pub use escrow::{EscrowContract, EscrowStatus};
pub use rpc::RpcServer;
pub use tokens::{ERC20Token, PNP16Token, TokenRegistry, TokenStandard};
pub use wallet::{KeyPair, Wallet};
