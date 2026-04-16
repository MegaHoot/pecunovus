# Pecu Novus Blockchain — Rust Implementation

<p><img src="https://img.shields.io/badge/License-Apache_2.0-green" alt="Apache 2.0">
<img src="https://img.shields.io/badge/EVM-Compatible-blue" alt="EVM Compatible">
<img src="https://img.shields.io/badge/Protocol-ERC--20_%2B_ERC--1400_%2B_PNP16-blue" alt="ERC-20">
<img src="https://img.shields.io/badge/Tests-88_passing-brightgreen" alt="Tests">
</p>

**Pecu 2.0 / 3.0 Themis** | Hybrid Proof of Time (PoT) + Proof of Stake (PoS)
> PNP16 + ERC-20 + ERC-1400 | 110,000+ TPS | EVM Compatible | Chain ID: 27272727
> Maintained by MegaHoot Technologies | Est. January 15, 2017

---

## Overview

This is a complete Rust implementation of the Pecu Novus blockchain network, built
faithfully from the 2018 and 2024 official whitepapers and the `pecu-rpc` specification.

### Architecture

```
pecu-novus/
├── src/
│   ├── main.rs          Node entry point, block producer loop, validator rewards
│   ├── lib.rs           Library exports
│   ├── crypto/          SHA-512, SHA-256, Keccak-256, VDF, CBC encryption, Merkle
│   ├── chain/           Block, Transaction, Blockchain state machine
│   ├── consensus/       Hybrid PoT+PoS, Validator registry, Halving/Vesting schedules
│   ├── tokens/          PNP16, ERC-20, ERC-1400, TokenRegistry
│   ├── escrow/          MVault, EscrowContract, TransferCards, Cold Storage
│   ├── wallet/          KeyPair, Wallet, GAK, DAK (KYC)
│   ├── storage/         Sled-based persistent storage
│   └── rpc/             45+ JSON-RPC methods (eth_*, erc20_*, pecu_*, pnp16_*, escrow_*)
└── tests/
    └── integration_tests.rs   88 tests covering all modules
```

---

## Quick Start

### Prerequisites
- Rust 1.75+ (`rustup update stable`)
- Cargo

### Build & Run

```bash
# Build
cargo build --release

# Run node (default port 8545)
./target/release/pecu-node

# Custom port
PECU_RPC_PORT=9000 ./target/release/pecu-node

# Run tests (88 tests)
cargo test
```

---

## JSON-RPC API

The node exposes a JSON-RPC server on `http://localhost:8545`.
All requests use `POST /` or `POST /rpc` with `Content-Type: application/json`.

### Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": [...],
  "id": 1
}
```

---

## EVM / Ethereum-Compatible Methods

These methods make Pecu Novus compatible with **MetaMask**, **Ethers.js**, and **Web3.js**.
Simply point your wallet/dApp to `http://localhost:8545` with Chain ID `27272727`.

| Method | Description |
|--------|-------------|
| `eth_chainId` | Returns `0x19FAFB7` (27272727) |
| `net_version` | Returns `"27272727"` |
| `eth_blockNumber` | Latest block height (hex) |
| `eth_getBalance` | PECU balance for address |
| `eth_getBlockByNumber` | Block by height or `"latest"` |
| `eth_getBlockByHash` | Block by hash |
| `eth_getTransactionByHash` | Transaction details |
| `eth_sendRawTransaction` | Submit signed transaction |
| `eth_call` | Call smart contract (read-only) |
| `eth_gasPrice` | Current gas price |
| `eth_estimateGas` | Estimate gas for transaction |
| `eth_getTransactionCount` | Nonce for address |
| `eth_getLogs` | Event logs |
| `web3_clientVersion` | `"PecuNovus/v2.0.0-rust/Pecu3.0Themis"` |
| `eth_syncing` | Sync status |
| `eth_accounts` | Available accounts |

### Example: Get Block Number

```bash
curl http://localhost:8545 \
  -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

```json
{"jsonrpc":"2.0","result":"0x1","id":1}
```

---

## ERC-20 Token Methods

Full ERC-20 interface for PNP16 tokens.

| Method | Params | Description |
|--------|--------|-------------|
| `erc20_balanceOf` | `[contract, address]` | Token balance |
| `erc20_transfer` | `[contract, from, to, amount]` | Transfer tokens |
| `erc20_approve` | `[contract, owner, spender, amount]` | Approve spender |
| `erc20_allowance` | `[contract, owner, spender]` | Check allowance |
| `erc20_transferFrom` | `[contract, spender, from, to, amount]` | Transfer on behalf |
| `erc20_totalSupply` | `[contract]` | Total token supply |

### Example: ERC-20 Transfer

```bash
curl http://localhost:8545 \
  -X POST -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","method":"erc20_transfer","id":1,
    "params":["0xContractAddress","0xFrom","0xTo","1000000000000000000"]
  }'
```

---

## Pecu Novus Native Methods

| Method | Params | Description |
|--------|--------|-------------|
| `pecu_getNetworkInfo` | `[]` | Network details, consensus, TPS |
| `pecu_getChainStats` | `[]` | Block height, tx count, burned PECU |
| `pecu_sendTransaction` | `[from, to, amount, note?]` | Send PECU |
| `pecu_getBalance` | `[address]` | PECU balance (raw + display) |
| `pecu_createWallet` | `[]` | Generate new keypair + addresses |
| `pecu_getWallet` | `[address]` | Wallet info |
| `pecu_getValidators` | `[]` | All validators + weights |
| `pecu_registerValidator` | `[address, stake]` | Register validator node |
| `pecu_mineBlock` | `[]` | Produce a new PoT block |
| `pecu_getHalvingSchedule` | `[]` | Reward halving table |
| `pecu_getVestingSchedule` | `[]` | Token unlock schedule |
| `pecu_getTokenomics` | `[]` | Full tokenomics summary |

### Example: Create Wallet

```bash
curl http://localhost:8545 \
  -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"pecu_createWallet","params":[],"id":1}'
```

```json
{
  "jsonrpc": "2.0",
  "result": {
    "walletId": "a3f1b2c4-...",
    "evmAddress": "0x1a2b3c4d...",
    "pecuAddress": "5HueCGU8...",
    "publicKey": "a1b2c3d4...",
    "createdAt": 1713200000
  },
  "id": 1
}
```

---

## PNP16 Token Methods

| Method | Params | Description |
|--------|--------|-------------|
| `pnp16_deployToken` | `[name, symbol, decimals, supply, creator, dak]` | Deploy new token |
| `pnp16_listTokens` | `[]` | List all deployed tokens |
| `pnp16_getToken` | `[contractAddress]` | Token details |
| `pnp16_mint` | `[contract, to, amount]` | Mint new tokens |
| `pnp16_burn` | `[contract, from, amount]` | Burn tokens |
| `pnp16_transfer` | `[contract, from, to, amount]` | Transfer tokens |

### Supported Asset Classes (PNP16)

| Class | Description |
|-------|-------------|
| `FinancialAsset` | Company stake / equity |
| `GamingAsset` | In-game rewards, points, skills |
| `PhysicalCommodity` | Gold, silver, oil, agricultural products |
| `FractionalRealEstate` | Tokenized property ownership |
| `IntellectualProperty` | Music, film, software royalties |
| `Stablecoin` | Fiat-pegged tokens |
| `SecurityToken` | ERC-1400 regulated securities |
| `Utility` | General utility |

### Example: Deploy PNP16 / ERC-20 Token

```bash
curl http://localhost:8545 \
  -X POST -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","method":"pnp16_deployToken","id":1,
    "params":["PecuGold","PGLD",18,"1000000000000000000000000","0xYourAddress","YOUR_DAK"]
  }'
```

---

## Escrow / MVault Methods

Based on the MVault system from the whitepaper — no-code escrow deployment.

| Method | Params | Description |
|--------|--------|-------------|
| `escrow_create` | `[sender, receiver, amount, releaseDate, note?, agreement?]` | Create escrow |
| `escrow_release` | `[escrowId, earlyRelease?]` | Release funds |
| `escrow_cancel` | `[escrowId]` | Cancel escrow (sender only) |
| `escrow_get` | `[escrowId]` | Escrow details |
| `escrow_listByAddress` | `[address]` | Pending escrows for address |
| `transfercard_create` | `[issuer, amount, tokenContract?, expiresAt?, useCase]` | Create Transfer Card |
| `transfercard_redeem` | `[redemptionKey, redeemer]` | Redeem Transfer Card |

### Example: Create Escrow (Real Estate)

```bash
curl http://localhost:8545 \
  -X POST -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0","method":"escrow_create","id":1,
    "params":[
      "0xBuyer","0xSeller","5000000000000000000000",
      1735689600,
      "Property deposit: 123 Blockchain Ave",
      "Release on deed transfer completion"
    ]
  }'
```

---

## Cold Storage (CSS) Methods

| Method | Params | Description |
|--------|--------|-------------|
| `css_moveToColdStorage` | `[address, amount]` | Move PECU offline, returns unique key |
| `css_redeemColdStorage` | `[address, storageKey]` | Bring assets back online |

---

## Access Key Methods

| Method | Params | Description |
|--------|--------|-------------|
| `gak_connect` | `[address, appId, ttlSeconds?]` | Connect wallet to app (GAK) |
| `gak_disconnect` | `[address, appId]` | Disconnect wallet from app |
| `dak_register` | `[developerName, email]` | Register for Development Access Key |
| `dak_verifyKyc` | `[dakId]` | Verify KYC (activates DAK) |

---

## Consensus: Hybrid PoT + PoS

### Proof of Time (PoT)

Based on Verifiable Delay Functions (VDF):

```
y = x^(2^T) mod N
```

- `x` = input seed (transaction hash / block hash)
- `T` = delay steps (sequential, non-parallelizable)
- `y` = output proof (instantly verifiable)

### Hybrid Selection (Pecu 3.0 Themis)

Validator weight = `uptime_seconds × √(stake + 1)`

This combines:
- **Time commitment** (PoT) — long-running nodes weighted higher
- **Economic stake** (PoS) — stake adds accountability without centralizing
- **BFT guarantees** — tolerates Byzantine failures up to ⅓ of validators

### Validator Rewards (Whitepaper)

| Parameter | Value |
|-----------|-------|
| Reward per node per 24h | 0.25 – 1.50 PECU (randomized) |
| Daily cap (all validators) | ~55,000 PECU |
| Annual cap | 20,000,000 PECU |
| First halving | 2027 |
| Halving frequency | Every 10 years |

---

## Tokenomics

| Metric | Value |
|--------|-------|
| Max supply | 1,000,000,000 PECU (fixed, never exceeded) |
| Decimal places | 15 |
| Gas fee | 0.0025% flat (all transaction types) |
| Fee burn | 50% of all gas fees permanently burned |
| Validator gets | 50% of gas fees |

### Halving Schedule

| Period | Max Annual Reward |
|--------|-------------------|
| 2017 – 2027 | 20,000,000 PECU |
| 2027 – 2037 | 10,000,000 PECU |
| 2037 – 2047 | 5,000,000 PECU |
| 2047 – 2057 | 2,500,000 PECU |
| 2057+ | 1,250,000 PECU |

### Vesting Schedule (Locked Tokens)

| Release Year | Amount |
|-------------|--------|
| 2026 | 40,000,000 PECU |
| 2028 | 30,000,000 PECU |
| 2030 | 30,000,000 PECU |
| 2032 | 20,000,000 PECU |
| 2034 | 10,000,000 PECU |

---

## MetaMask Integration

Add Pecu Novus to MetaMask as a custom network:

| Field | Value |
|-------|-------|
| Network Name | Pecu Novus Mainnet |
| RPC URL | `https://mainnet.pecunovus.net` |
| Chain ID | `27272727` |
| Currency Symbol | `PECU` |
| Block Explorer | `https://pecuscan.com` |

---

## License

Apache License 2.0 — © 2017–2026 Pecu Novus Network / MegaHoot Technologies
