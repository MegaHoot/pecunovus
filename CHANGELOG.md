# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [2.0.0] — 2026-04-16

### Added

**Core Blockchain**
- `Block`, `BlockHeader`, `Transaction` with full serialization
- `Blockchain` state machine: mempool, balance ledger, nonce tracking, ERC-20 allowances
- Genesis block with January 15, 2017 launch timestamp
- Flat 0.0025% gas fee with automatic 50% burn mechanism
- Dual confirmation (sender + receiver) per original whitepaper
- Escrow transaction type with dated auto-release

**Cryptography (`crypto/`)**
- SHA-512 key generation (64–128 char public key, 60–128 char private key) per whitepaper
- SHA-256 block hashing and Merkle tree computation
- Keccak-256 for EVM-compatible address derivation (`0x...`)
- Base58 native Pecu address format
- Verifiable Delay Function (VDF): `y = x^(2^T) mod N` (iterated SHA-256 approximation)
- VDF proof generation + instant verification
- Cipher Block Chaining (CBC) encryption with XOR chaining
- PoT sequence signing via `sign_with_pot_sequence`
- HMAC-SHA256 message authentication
- UUID generation without external crate dependency

**Consensus (`consensus/`)**
- Proof of Time (PoT) engine with VDF-based block sequencing
- Hybrid PoT + Proof of Stake (Pecu 3.0 Themis) with BFT weighting
- Validator registry with one-node-per-wallet policy
- Weighted validator selection: `uptime × √(stake + 1)`
- Randomized validator rewards: 0.25–1.50 PECU/node/24h
- Daily reward cap: ~55,000 PECU; Annual cap: 20,000,000 PECU
- Official halving schedule: 2017→2027→2037→2047→2057 (50% each decade)
- Official vesting schedule: 2026/2028/2030/2032/2034 unlock tranches

**Token Standards (`tokens/`)**
- **PNP16**: Native Pecu Novus token standard
- **ERC-20**: Full interface — `transfer`, `approve`, `allowance`, `transferFrom`, `mint`, `burn`
- **ERC-1400**: Security token standard — partitions, operator authorization, KYC holder registry
- 8 asset classes: FinancialAsset, GamingAsset, PhysicalCommodity, FractionalRealEstate, IP, Stablecoin, SecurityToken, Utility
- Per-token subset ledger (on-chain transaction history per whitepaper)
- `TokenRegistry` for multi-token deployment management

**Escrow / MVault (`escrow/`)**
- `EscrowContract` with: date-locked release, early release, cancellation, required actions, dispute flag
- Auto-release processing via `MVault::process_auto_releases`
- Token escrow (PNP16/ERC-20) alongside PECU escrow
- **Transfer Cards**: unique redemption key, digital/physical support, expiry with reversion
- Transfer card use cases: EventGiveaway, GiftingDigitalAssets, TokenLaunch, MarketingCampaign

**Wallet (`wallet/`)**
- `Wallet` with PECU balance (15 decimal places), token balance map
- Public key refresh (security feature per whitepaper)
- Cold Storage System (CSS): move assets offline with unique key, redeem anytime
- **GAK** (General Access Key): connect/disconnect wallet from apps with TTL
- **DAK** (Development Access Key): KYC-verified developer registration + revocation

**Storage (`storage/`)**
- Sled embedded database for blocks, transactions, tokens, wallets
- In-memory mode for testing
- Indexed by both block height and hash

**JSON-RPC Server (`rpc/`) — 45+ methods**
- EVM-compatible: `eth_chainId`, `eth_blockNumber`, `eth_getBalance`, `eth_getBlockByNumber`, `eth_getBlockByHash`, `eth_getTransactionByHash`, `eth_sendRawTransaction`, `eth_call`, `eth_gasPrice`, `eth_estimateGas`, `eth_getTransactionCount`, `eth_getLogs`, `web3_clientVersion`, `eth_syncing`, `eth_accounts`
- ERC-20: `erc20_balanceOf`, `erc20_transfer`, `erc20_approve`, `erc20_allowance`, `erc20_transferFrom`, `erc20_totalSupply`
- Native Pecu: `pecu_getNetworkInfo`, `pecu_getChainStats`, `pecu_sendTransaction`, `pecu_getBalance`, `pecu_createWallet`, `pecu_getWallet`, `pecu_getValidators`, `pecu_registerValidator`, `pecu_mineBlock`, `pecu_getHalvingSchedule`, `pecu_getVestingSchedule`, `pecu_getTokenomics`
- PNP16: `pnp16_deployToken`, `pnp16_listTokens`, `pnp16_getToken`, `pnp16_mint`, `pnp16_burn`, `pnp16_transfer`
- Escrow: `escrow_create`, `escrow_release`, `escrow_cancel`, `escrow_get`, `escrow_listByAddress`, `transfercard_create`, `transfercard_redeem`
- Cold Storage: `css_moveToColdStorage`, `css_redeemColdStorage`
- Access Keys: `gak_connect`, `gak_disconnect`, `dak_register`, `dak_verifyKyc`
- CORS enabled for browser-based dApps

**Background Services**
- Async block producer loop (PoT-driven, 2s interval)
- Async validator reward issuer (daily, simulated as 60s in dev)

**Tests**
- 88 integration tests across 7 test modules
- Coverage: crypto, wallet, chain, consensus, tokens (PNP16/ERC-20/ERC-1400), escrow/MVault, tokenomics constants, end-to-end scenarios

**Open Source**
- Apache License 2.0 (`LICENSE`, `NOTICE`, Cargo.toml declaration)
- `CONTRIBUTING.md` with PR workflow, commit conventions, development areas
- `SECURITY.md` with responsible disclosure policy
- `CHANGELOG.md` (this file)
- `.gitignore` for Rust projects
- Example clients: `examples/rpc_client.sh` (curl), `examples/rpc_client.js` (Node.js)

### Technical Notes
- Chain ID: 3001 (Pecu Novus Mainnet)
- MetaMask compatible (custom network: RPC `http://localhost:8545`, Chain ID `3001`)
- Rust edition 2021, minimum Rust version 1.75

---

## [1.0.0] — 2022-08-27 (Pecu 2.0 "Code Falcon" reference)

*Historical reference — this was the original Pecu 2.0 network overhaul.*
*The Rust implementation begins at v2.0.0 above.*
