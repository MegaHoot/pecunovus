#!/usr/bin/env bash
# examples/rpc_client.sh
# Pecu Novus JSON-RPC Example Client
# Run the node first: cargo run --bin pecu-node
# Then: bash examples/rpc_client.sh

RPC="http://localhost:8545"
BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

rpc() {
  local method="$1"
  local params="${2:-[]}"
  echo -e "${CYAN}▶ $method${NC}"
  curl -s "$RPC" \
    -X POST -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
    | python3 -m json.tool 2>/dev/null || echo "(no json.tool available)"
  echo ""
}

echo -e "${BOLD}════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Pecu Novus Blockchain — RPC Client Demo${NC}"
echo -e "${BOLD}════════════════════════════════════════════════════════${NC}\n"

echo -e "${BOLD}── 1. Network Info ──────────────────────────────────────${NC}"
rpc "pecu_getNetworkInfo"
rpc "eth_chainId"
rpc "web3_clientVersion"

echo -e "${BOLD}── 2. Chain Stats ───────────────────────────────────────${NC}"
rpc "eth_blockNumber"
rpc "pecu_getChainStats"

echo -e "${BOLD}── 3. Create a Wallet ───────────────────────────────────${NC}"
WALLET_RESPONSE=$(curl -s "$RPC" \
  -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"pecu_createWallet","params":[],"id":1}')
echo "$WALLET_RESPONSE" | python3 -m json.tool 2>/dev/null
ALICE_ADDR=$(echo "$WALLET_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['result']['evmAddress'])" 2>/dev/null)
echo -e "${GREEN}Alice address: $ALICE_ADDR${NC}\n"

echo -e "${BOLD}── 4. Get Balance ───────────────────────────────────────${NC}"
rpc "pecu_getBalance" "[\"$ALICE_ADDR\"]"
rpc "eth_getBalance" "[\"$ALICE_ADDR\",\"latest\"]"

echo -e "${BOLD}── 5. Register as Validator ─────────────────────────────${NC}"
rpc "pecu_registerValidator" "[\"$ALICE_ADDR\",\"1000000000000000000\"]"
rpc "pecu_getValidators"

echo -e "${BOLD}── 6. Tokenomics ────────────────────────────────────────${NC}"
rpc "pecu_getTokenomics"
rpc "pecu_getHalvingSchedule"
rpc "pecu_getVestingSchedule"

echo -e "${BOLD}── 7. Deploy PNP16 / ERC-20 Token ───────────────────────${NC}"
TOKEN_RESPONSE=$(curl -s "$RPC" \
  -X POST -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"pnp16_deployToken\",\"params\":[
    \"PecuGold\",\"PGLD\",18,
    \"1000000000000000000000000\",
    \"$ALICE_ADDR\",\"DAK_DEMO_001\"
  ],\"id\":1}")
echo "$TOKEN_RESPONSE" | python3 -m json.tool 2>/dev/null
CONTRACT=$(echo "$TOKEN_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['result']['contractAddress'])" 2>/dev/null)
echo -e "${GREEN}Contract: $CONTRACT${NC}\n"

echo -e "${BOLD}── 8. ERC-20 Operations ─────────────────────────────────${NC}"
rpc "erc20_totalSupply" "[\"$CONTRACT\"]"
rpc "erc20_balanceOf" "[\"$CONTRACT\",\"$ALICE_ADDR\"]"

# Approve spender
BOB_ADDR="0x0000000000000000000000000000000000000bob"
rpc "erc20_approve" "[\"$CONTRACT\",\"$ALICE_ADDR\",\"$BOB_ADDR\",\"500000000000000000000\"]"
rpc "erc20_allowance" "[\"$CONTRACT\",\"$ALICE_ADDR\",\"$BOB_ADDR\"]"

echo -e "${BOLD}── 9. List All Tokens ───────────────────────────────────${NC}"
rpc "pnp16_listTokens"

echo -e "${BOLD}── 10. Create Escrow ────────────────────────────────────${NC}"
RELEASE=$(date -d "+7 days" +%s 2>/dev/null || date -v+7d +%s 2>/dev/null || echo "1735689600")
rpc "escrow_create" "[
  \"$ALICE_ADDR\",
  \"$BOB_ADDR\",
  \"5000000000000000000000\",
  $RELEASE,
  \"Property deposit: 123 Blockchain Ave\",
  \"Release on deed transfer\"
]"

echo -e "${BOLD}── 11. Transfer Card (Event Giveaway) ───────────────────${NC}"
CARD_RESPONSE=$(curl -s "$RPC" \
  -X POST -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"transfercard_create\",\"params\":[
    \"$ALICE_ADDR\",
    \"1000000000000000000\",
    null,
    $(( $(date +%s) + 86400 )),
    \"EventGiveaway\"
  ],\"id\":1}")
echo "$CARD_RESPONSE" | python3 -m json.tool 2>/dev/null
CARD_KEY=$(echo "$CARD_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['result']['redemptionKey'])" 2>/dev/null)

echo -e "${BOLD}── 12. Redeem Transfer Card ─────────────────────────────${NC}"
rpc "transfercard_redeem" "[\"$CARD_KEY\",\"0xAttendee1\"]"

echo -e "${BOLD}── 13. Cold Storage ─────────────────────────────────────${NC}"
rpc "css_moveToColdStorage" "[\"$ALICE_ADDR\",\"100000000000000000\"]"

echo -e "${BOLD}── 14. Access Keys (GAK/DAK) ────────────────────────────${NC}"
rpc "gak_connect" "[\"$ALICE_ADDR\",\"HootDex\",3600]"
rpc "dak_register" "[\"Alice Developer\",\"alice@dev.com\"]"

echo -e "${BOLD}── 15. Mine a Block ─────────────────────────────────────${NC}"
rpc "pecu_mineBlock"
rpc "eth_blockNumber"

echo -e "${BOLD}════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✅  Demo complete — Pecu Novus RPC client walkthrough done${NC}"
echo -e "${BOLD}════════════════════════════════════════════════════════${NC}"
