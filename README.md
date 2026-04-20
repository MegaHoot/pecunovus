<p align="center">
  <strong>PECU NOVUS</strong><br>
  <em>Blockchain Infrastructure for the Next Era of Finance</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Network-Live-brightgreen" alt="Network Live" />
  <img src="https://img.shields.io/badge/Consensus-Hybrid%20PoT%20%2B%20PoS-blue" alt="Hybrid PoT + PoS" />
  <img src="https://img.shields.io/badge/TPS-110%2C000%2B-orange" alt="110K+ TPS" />
  <img src="https://img.shields.io/badge/Carbon-Neutral-success" alt="Carbon Neutral" />
  <img src="https://img.shields.io/badge/Protocol-PNP16%20%2B%20ERC--20%20%2B%20ERC--1400-blueviolet" alt="Protocol" />
  <img src="https://img.shields.io/badge/Languages-Golang%20%2B%20Rust%20%2B%20Solidity-informational" alt="Golang + Rust + Solidity" />
  <img src="https://img.shields.io/badge/EVM-Compatible-purple" alt="EVM Compatible" />
  <img src="https://img.shields.io/badge/Tests-88_Rust_%7C_73_Python_%7C_21_Go-brightgreen" alt="Tests" />
  <img src="https://img.shields.io/badge/Chain_ID-27272727-blue" alt="Chain ID" />
  <img src="https://img.shields.io/badge/License-Apache_2.0-green" alt="Apache 2.0" />
</p>

---

## Overview

Pecu Novus is a high-performance, carbon-neutral Layer-1 blockchain network purpose-built for real-world financial infrastructure. Originally conceived in 2016 and launched in January 2017, the network has undergone continuous evolution — from its foundational architecture through the complete **Pecu 2.0** overhaul in 2022, to the current **Pecu 3.0 Themis** upgrade rolling out across 2025–2027.

At its core, Pecu Novus is designed to solve the fundamental problems that have held back institutional blockchain adoption: **speed without compromise, compliance without friction, and scalability without centralization.**

The network operates on a **hybrid Proof of Time (PoT) and Proof of Stake (PoS)** consensus mechanism — a proprietary, energy-efficient model that combines time-based validation with stake-weighted participation, rooted in Byzantine Fault Tolerance (BFT) principles. This allows Pecu Novus to process over **110,000 transactions per second** through its **PNP16 protocol**, with throughput scaling 3–5x as the global Validator node network expands.

### Network at a Glance

| Metric | Value |
|---|---|
| **Consensus** | Hybrid Proof of Time (PoT) + Proof of Stake (PoS) with BFT |
| **Throughput** | 110,000+ TPS (stress-tested) |
| **Max Supply** | 1,000,000,000 PECU |
| **Total Supply** | ~537,372,828 PECU (as of 04/2026) |
| **Circulating Supply** | ~301,565,915 PECU (as of 04/2026) |
| **Architecture** | Hybrid — public chain + permissioned private forks |
| **Smart Contracts** | MVault system with no-code interface |
| **Token Standards** | PNP16 + ERC-20 + ERC-1400 |
| **Core Languages** | Golang + Rust + Solidity |
| **EVM Compatibility** | Full ERC-20 / EVM support via Pecu 3.0 Themis Upgrade |
| **Chain ID** | 27272727 |
| **Carbon Status** | Carbon Neutral |
| **Block Explorer** | [Pecuscan](https://pecuscan.com) |

---

## Repository Structure

```
pecu-novus/
├── README.md                      ← You are here
├── WHITEPAPER_2018.md             ← Original whitepaper (Gauss & Ram, 2018)
├── WHITEPAPER.md                  ← Technical whitepaper (Velazquez & Bhardwaj, 2024)
├── src/
│   ├── main.rs                    Node entry point, block producer loop, validator rewards
│   ├── lib.rs                     Library exports
│   ├── crypto/                    SHA-512, SHA-256, Keccak-256, VDF, CBC encryption, Merkle
│   ├── chain/                     Block, Transaction, Blockchain state machine
│   ├── consensus/                 Hybrid PoT+PoS, Validator registry, Halving/Vesting schedules
│   ├── tokens/                    PNP16, ERC-20, ERC-1400, TokenRegistry
│   ├── escrow/                    MVault, EscrowContract, TransferCards, Cold Storage
│   ├── wallet/                    KeyPair, Wallet, GAK, DAK (KYC)
│   ├── storage/                   Sled-based persistent storage
│   └── rpc/                       45+ JSON-RPC methods
├── tests/
│   └── integration_tests.rs       88 tests covering all modules
└── packages/
    ├── sdk-typescript/            TypeScript/JS SDK (@pecunovus/sdk)
    ├── sdk-python/                Python SDK (pecu-sdk)
    └── sdk-go/                    Go SDK (pecu-sdk-go)
```

---

## Why Pecu Novus?

### Speed That Matches Real-World Demand

Pecu Novus was stress-tested at **110,000+ TPS** in real-time conditions — not theoretical projections. That throughput is designed to scale 3–5x as more Validator nodes join the network.

| Network | Transactions Per Second |
|---|---|
| Bitcoin (BTC) | ~7 |
| Ethereum (ETH) | ~30 |
| Solana (SOL) | ~65,000 |
| **Pecu Novus (PECU)** | **110,000+** |

### Institutional-Grade Compliance — Built In, Not Bolted On

With **ERC-1400** (security token standard) alongside **PNP16**, Pecu Novus embeds compliance, identity-aware transfers, and regulatory controls directly into the protocol layer:

- Regulated tokenized securities with audit-ready transfer logic
- Deep, transparent asset metadata via PNP16's high-fidelity data framework
- Permissioned private forks for enterprise and government deployments
- ERC-20 compatibility for seamless interoperability with the broader Ethereum ecosystem
- Solidity smart contract deployment — developers can write and deploy EVM-compatible contracts directly on Pecu Novus

### Energy-Efficient by Design

The hybrid PoT/PoS model eliminates the energy arms race of Proof of Work networks. No specialized mining hardware required — standard devices can run Validator nodes. The network maintains **carbon-neutral status** aligned with global sustainability mandates.

### A Consensus Mechanism That Is Actually Fair

- **Time-based validation** uses random wait times to ensure equal opportunity for block creation — no hardware arms race
- **Stake-weighted participation** adds economic accountability without plutocratic control
- **BFT foundation** ensures network integrity even when individual nodes behave unpredictably

### Built-In Escrow and Smart Contract Infrastructure

- **Timed escrow** with automatic dated release for real estate, trade finance, and import/export
- **Dual confirmation** requiring both sender and receiver to authorize transactions
- **MVault smart contracts** with a no-code deployment interface
- **Transaction notes** permanently recorded on-chain for audit trails

---

## Ecosystem

| Platform | Function |
|---|---|
| **[Pecu Wallet & Terminal](https://pecunovus.net)** | DeFi wallet with integrated terminal |
| **[HootDex](https://hootdex.com)** | Peer-to-peer digital asset swapping |
| **[XMG Fintech](https://xmgfintech.com)** | Stablecoin, RWA tokenization, and payment portal |
| **[Pecuscan](https://pecuscan.com)** | Blockchain explorer |
| **[MegaHoot ChatHive](https://mchathive.com)** | Messaging superapp |

---

## Quick Start (Rust Node)

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

## SDK Installation

**TypeScript / JavaScript**
```bash
npm install @pecunovus/sdk
```

**Python**
```bash
pip install pecu-sdk
```

**Go**
```bash
go get github.com/MegaHoot/pecu-sdk-go
```

### Quick Start Examples

**TypeScript**
```ts
import { PecuClient } from "@pecunovus/sdk";

const client = new PecuClient({ rpcUrl: "http://localhost:8545" });
const wallet = await client.createWallet();
const balance = await client.getBalance(wallet.evmAddress);
const tx = await client.sendTransaction(wallet.evmAddress, "0xRecipient", PecuClient.toRaw(100));
```

**Python**
```python
from pecu_sdk import PecuClient

client = PecuClient("http://localhost:8545")
wallet = client.create_wallet()
balance = client.get_balance(wallet["evmAddress"])
tx = client.send_transaction(wallet["evmAddress"], "0xRecipient", PecuClient.to_raw(100))
```

**Go**
```go
import "github.com/MegaHoot/pecu-sdk-go/pecu"

client := pecu.NewClient("http://localhost:8545")
wallet, _ := client.CreateWallet(ctx)
balance, _ := client.GetBalance(ctx, wallet.EvmAddress)
tx, _ := client.SendTransaction(ctx, wallet.EvmAddress, "0xRecipient", pecu.ToRaw(100), "")
```

---

## JSON-RPC API

The node exposes a JSON-RPC server on `http://localhost:8545`.
All requests use `POST /` or `POST /rpc` with `Content-Type: application/json`.

```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": [...],
  "id": 1
}
```

### RPC Coverage

| Namespace | Methods | Description |
|-----------|---------|-------------|
| `eth_*` | 15 | EVM / MetaMask-compatible |
| `erc20_*` | 6 | Full ERC-20 interface |
| `pecu_*` | 12 | Native Pecu operations |
| `pnp16_*` | 6 | PNP16 token deployment |
| `escrow_*` / `transfercard_*` | 7 | MVault escrow & Transfer Cards |
| `css_*` / `gak_*` / `dak_*` | 6 | Cold Storage, Access Keys |

### Key Methods

**EVM-Compatible**

| Method | Description |
|--------|-------------|
| `eth_chainId` | Returns `0x19FAFB7` (27272727) |
| `eth_blockNumber` | Latest block height (hex) |
| `eth_getBalance` | PECU balance for address |
| `eth_sendRawTransaction` | Submit signed transaction |
| `eth_call` | Call smart contract (read-only) |

**Native Pecu**

| Method | Description |
|--------|-------------|
| `pecu_createWallet` | Generate new keypair + addresses |
| `pecu_sendTransaction` | Send PECU with optional note |
| `pecu_getValidators` | All validators + weights |
| `pecu_getTokenomics` | Full tokenomics summary |
| `pecu_getHalvingSchedule` | Reward halving table |

**Escrow / MVault**

| Method | Description |
|--------|-------------|
| `escrow_create` | Create escrow with release date |
| `escrow_release` | Release funds |
| `escrow_cancel` | Cancel escrow (sender only) |
| `transfercard_create` | Create Transfer Card |
| `transfercard_redeem` | Redeem Transfer Card |

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

## For Developers

### Development Languages

| Language | Use Case | Ecosystem |
|---|---|---|
| **Solidity** | EVM-compatible smart contracts, DeFi, tokenized assets, NFTs | Hardhat, Foundry, Remix, OpenZeppelin, Ethers.js, Web3.js |
| **Golang** | Core protocol development, Validator node infrastructure | Native Pecu Novus SDK, gRPC interfaces |
| **Rust** | Performance-critical modules, cryptographic primitives | Systems-level integration |

### EVM Compatibility

With the Pecu 3.0 Themis upgrade, Pecu Novus offers **full EVM equivalence**:

- Deploy existing Solidity contracts without modification
- Use familiar tools — Hardhat, Foundry, Remix, Truffle
- Leverage OpenZeppelin libraries for battle-tested token standards
- Connect with MetaMask, WalletConnect, and any ERC-20-compatible wallet
- Interact via standard JSON-RPC — existing dApps can point to Pecu Novus with a simple RPC endpoint change

> **The difference:** Your Solidity contracts execute on a network delivering 110,000+ TPS with hybrid PoT + PoS consensus and built-in compliance primitives — capabilities unavailable on Ethereum mainnet.

### MetaMask Integration

| Field | Value |
|-------|-------|
| Network Name | Pecu Novus Mainnet |
| RPC URL | `https://mainnet.pecunovus.net` |
| Chain ID | `27272727` |
| Currency Symbol | `PECU` |
| Block Explorer | `https://pecuscan.com` |

### PNP16 Supported Asset Classes

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

### Validator Weight Formula (Pecu 3.0 Themis)

```
weight = uptime_seconds × √(stake + 1)
```

This combines time commitment (PoT) with economic stake (PoS) while BFT guarantees tolerate Byzantine failures up to ⅓ of validators.

### Validator Rewards

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
| Max supply | 1,000,000,000 PECU (fixed) |
| Decimal places | 15 |
| Gas fee | 0.0025% flat |
| Fee burn | 50% of all gas fees permanently burned |
| Validator share | 50% of gas fees |

### Token Allocation

| Allocation | Supply | Purpose |
|---|---|---|
| Ecosystem & Development | 24,000,000 | Network growth, partnerships, infrastructure |
| Community & Rewards | 300,000,000 | Validator incentives, staking rewards |
| Reserve | 150,000,000 | Stability fund, strategic deployment |
| Core Team & Founders | 54,000,000 | Vested over 8 years (2026–2034) |
| Public Circulation | 301,565,915 | Open market, liquidity |
| Total Burned | 9,372,828 | PECU burned |
| **Total Max Supply** | **1,000,000,000** | |

### Halving Schedule

| Period | Max Annual Reward |
|--------|-------------------|
| 2017 – 2027 | 20,000,000 PECU |
| 2027 – 2037 | 10,000,000 PECU |
| 2037 – 2047 | 5,000,000 PECU |
| 2047 – 2057 | 2,500,000 PECU |
| 2057+ | 1,250,000 PECU |

### Vesting Schedule

| Release Year | Amount |
|-------------|--------|
| 2026 | 40,000,000 PECU |
| 2028 | 30,000,000 PECU |
| 2030 | 30,000,000 PECU |
| 2032 | 20,000,000 PECU |
| 2034 | 10,000,000 PECU |

---

## Test Results

| SDK | Tests | Coverage |
|-----|-------|----------|
| Rust (integration) | **88 passing** | All modules |
| Python SDK | **73 passing** | Python 3.9–3.12 |
| Go SDK | **21 passing** | Go 1.21–1.22 |

---

## SDK Development

```bash
git clone https://github.com/MegaHoot/pecu-sdk
cd pecu-sdk

# Install TypeScript deps
cd packages/sdk-typescript && npm install && cd ../..

# Run all tests
./scripts/test.sh

# Build all packages
./scripts/build.sh

# Bump version across all packages
./scripts/version.sh patch    # 3.0.0 → 3.0.1
```

---

## Roadmap

| Year | Milestone |
|------|-----------|
| 2016 | Vision and foundational design (Gauss & Ram) |
| 2017 | Pecu Novus network launch — January 15, 2017 |
| 2018 | Original whitepaper published (Gauss & Ram) |
| 2022 | Pecu 2.0 "Code Falcon" — complete network overhaul; MegaHoot Technologies granted stewardship |
| 2024 | Technical whitepaper published (Velazquez & Bhardwaj); PNP16 stress-tested at 110,000+ TPS |
| 2025 | Pecu 3.0 "Themis" Phase 1 — Hybrid PoT + PoS consensus integration (October) |
| 2026 | Pecu 3.0 "Themis" Phase 2 — ERC-20 + GoLang integration (April) |
| 2026 | Phase 3 — ERC-1400 integration alongside PNP16 |
| TBD | Continued Validator node expansion · Cross-chain interoperability · Institutional partnership growth · Global regulatory alignment |

---

## Whitepapers

| Document | Authors | Date | Description |
|---|---|---|---|
| **[Original Whitepaper](WHITEPAPER_2018.md)** | Vinci Gauss, Sri Ram | January 2018 | Founding vision, network design, escrow system, tokenomics, and technical specifications |
| **[Technical Whitepaper 2024](WHITEPAPER.md)** | L. Velazquez, A. Bhardwaj | June 2024 | Applied architecture of Pecu 2.0, hybrid PoT/PoS consensus, PNP16 protocol, smart contracts, industry applications |

---

## Quick Links

| Resource | URL |
|---|---|
| 🌐 Website | [pecunovus.com](https://pecunovus.com) |
| 🔍 Block Explorer | [pecuscan.com](https://pecuscan.com) |
| 🏢 MegaHoot Technologies | [megahoot.com](https://megahoot.com) |
| 💱 HootDex | [hootdex.com](https://hootdex.com) |
| 📦 npm | [@pecunovus/sdk](https://www.npmjs.com/package/@pecunovus/sdk) |
| 🐍 PyPI | [pecu-sdk](https://pypi.org/project/pecu-sdk/) |
| 🔵 Go | [pecu-sdk-go](https://pkg.go.dev/github.com/MegaHoot/pecu-sdk-go/pecu) |

---

## Contributing & Security

Pecu Novus has been maintained by [MegaHoot Technologies](https://megahoot.com) since 2022. For partnership inquiries, developer access, or contribution guidelines, reach out via [pecunovus.com](https://pecunovus.com).

Report security vulnerabilities to **security@pecunovus.com**. See [SECURITY.md](./SECURITY.md) for the full disclosure policy.

---

## License & Disclaimer

Nothing in this repository constitutes a solicitation by the Pecu Novus Blockchain Network. MegaHoot Technologies publishes these materials solely as a record of achieved, applied results and to outline the potential utility of the Pecu Novus Blockchain Network across various industries.

> © 2017–2026 Pecu Novus Network · Licensed under the Apache License, Version 2.0. All rights reserved.
