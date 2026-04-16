# Contributing to Pecu Novus Blockchain (Rust)

Thank you for your interest in contributing! This project is licensed under the
**Apache License 2.0** and welcomes contributions from the community.

---

## Before You Start

By submitting a contribution, you confirm that:

1. You have the right to submit the contribution under the Apache 2.0 license.
2. You grant the Pecu Novus project a perpetual, worldwide, royalty-free license
   to use your contribution under the terms of the Apache 2.0 License.
3. Your contribution does not introduce any license incompatible with Apache 2.0.

For significant contributions, you may be asked to sign a Contributor License
Agreement (CLA) to protect both contributors and the project.

---

## How to Contribute

### 1. Reporting Issues

- Check existing issues before opening a new one.
- Include: Rust version (`rustc --version`), OS, reproduction steps, expected vs actual behavior.
- For security vulnerabilities, **do not open a public issue** — email security@pecunovus.com.

### 2. Pull Requests

```bash
# 1. Fork the repository
# 2. Create a feature branch
git checkout -b feature/your-feature-name

# 3. Make your changes
# 4. Run tests — all must pass
cargo test

# 5. Run clippy (no warnings)
cargo clippy -- -D warnings

# 6. Format code
cargo fmt

# 7. Commit with a clear message
git commit -m "feat: add zero-knowledge proof integration for PoT"

# 8. Push and open a PR
git push origin feature/your-feature-name
```

### 3. Commit Message Format

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat:     new feature
fix:      bug fix
docs:     documentation only
test:     adding or updating tests
refactor: code change that neither fixes a bug nor adds a feature
perf:     performance improvement
chore:    build system or tooling changes
```

---

## Development Setup

```bash
# Clone
git clone https://github.com/MegaHoot/pecu-novus-rust
cd pecu-novus-rust

# Build
cargo build

# Test (88 tests must pass)
cargo test

# Run node
cargo run --bin pecu-node

# Lint
cargo clippy
cargo fmt --check
```

---

## Project Structure

```
src/
├── crypto/     Cryptographic primitives (SHA-512, VDF, CBC, Keccak)
├── chain/      Core blockchain: Block, Transaction, state machine
├── consensus/  PoT + PoS hybrid consensus, validator logic
├── tokens/     PNP16, ERC-20, ERC-1400 token standards
├── escrow/     MVault, escrow contracts, Transfer Cards
├── wallet/     Key management, GAK, DAK
├── storage/    Sled-based persistence
└── rpc/        JSON-RPC server (45+ methods)
```

---

## Code Standards

- **No unsafe code** without explicit justification and review.
- All public APIs must have doc comments (`///`).
- New modules must include unit tests.
- Integration tests live in `tests/integration_tests.rs`.
- Maintain backward compatibility with the JSON-RPC API spec.
- Adhere to the Pecu Novus whitepaper specifications for constants
  (gas fees, halving schedule, supply cap, reward ranges).

---

## Areas We Welcome Contributions

| Area | Description |
|------|-------------|
| **PVM / Smart Contracts** | Pecu Virtual Machine (Golang smart contract executor) |
| **Zero-Knowledge Proofs** | ZKP integration for privacy-preserving transactions |
| **P2P Networking** | libp2p-based peer discovery and block propagation |
| **EVM Execution** | Full Solidity/EVM bytecode execution layer |
| **CLI Tool** | `pecu-cli` command-line wallet and node management |
| **Themis Governance** | On-chain voting and proposal system |
| **Cross-chain Bridge** | Interoperability with Ethereum, Solana |
| **Performance** | TPS benchmarking and optimization |
| **Documentation** | Tutorials, API docs, architecture diagrams |

---

## License

All contributions are made under the **Apache License 2.0**.
See [LICENSE](./LICENSE) for the full license text.

Copyright 2017–2026 Pecu Novus Network / MegaHoot Technologies
