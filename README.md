# 📖 Pecunovus

Pecu Novus is a **Layer-1** blockchain network designed for financial applications, focusing on hybrid architecture, scalability, and user-friendliness to bridge traditional finance and blockchain technology.
It supports **parallel runtime execution, RocksDB-backed state, validator voting with Tower BFT, and modular components** for networking, consensus, storage, and runtime.

---

## 🚀 Features

- **Hybrid Consensus (PoT + PoS)** with Tower BFT finality  
- **Validator voting + block proposal** integrated with networking  
- **RocksDB-backed ledger & state storage** (migratable from in-memory)  
- **Parallel runtime executor** with account locks & caching  
- **Transaction pool (TxPool)** with forwarding and ingestion  
- **Pluggable crypto module** (Ed25519, VRF support)  
- **P2P networking layer** with gossip, peer discovery, and secure handshakes  
- **JSON-RPC API** for clients and wallets  
- **Devnet tooling**: run multiple nodes locally, submit transactions, and verify block inclusion

---

## 📂 Project Structure

```
pecunovus/
├── src/
│   ├── api/          # External API definitions
│   ├── node/         # Node bootstrap, CLI, services
│   ├── network/      # P2P transport, gossip, peer management
│   ├── consensus/    # PoT, PoS, Tower BFT, voting
│   ├── ledger/       # Blockstore, snapshotting, pruning
│   ├── runtime/      # Executor, VM, program loader
│   ├── state/        # Accounts DB, locks, caching
│   ├── txpool/       # Transaction pool & forwarding
│   ├── storage/      # RocksDB / Sled stores
│   ├── crypto/       # Keys, signing, VRFs
│   ├── rpc/          # JSON-RPC server + handlers
│   ├── utils/        # Logging, metrics, error handling
│   └── tests/        # Integration & fuzz testing
├── config/
│   ├── devnet.toml   # Devnet config
│   └── mainnet.toml  # Mainnet config
├── Cargo.toml
├── Dockerfile
└── README.md
```

---

## ⚡ Getting Started

### 1. Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Clone & Build
```bash
git clone https://github.com/your-org/pecunovus.git
cd pecunovus
cargo build --release
```

### 3. Run a Node
```bash
cargo run -- --bind 127.0.0.1:7001 --rpc 127.0.0.1:8081 --db /tmp/node1
```

Run a second node and connect to the first:
```bash
cargo run -- --bind 127.0.0.1:7002 --rpc 127.0.0.1:8082 --peers 127.0.0.1:7001 --db /tmp/node2
```

---

## 🧩 RPC API (coming soon)

- `get_balance(pubkey)` → account balance  
- `send_transaction(tx)` → submit signed tx  
- `submit_block(block)` → inject block proposal  
- `get_block(slot)` → fetch block  

---

## 🐳 Running with Docker

Build image:
```bash
docker build -t pecunovus-node .
```

Run:
```bash
docker run -p 7001:7001 -p 8081:8081 pecunovus-node   --bind 0.0.0.0:7001 --rpc 0.0.0.0:8081 --db /data/db
```

---

## ⚙️ Roadmap

- [x] Project scaffolding & module layout  
- [x] Networking + CLI bootstrapping  
- [x] Consensus PoT + PoS base logic  
- [x] Full P2P gossip layer  
- [x] Parallel runtime execution  
- [x] JSON-RPC routes  
- [x] Devnet harness with multiple nodes  

---

## 👩‍💻 Contributing

1. Fork this repo  
2. Create a feature branch (`git checkout -b feature/awesome`)  
3. Commit changes (`git commit -m 'Add awesome feature'`)  
4. Push branch (`git push origin feature/awesome`)  
5. Open a Pull Request 🎉  

---

## 📜 License

MIT License © 2025 Pecunovus Authors
