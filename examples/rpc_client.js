// examples/rpc_client.js
// Pecu Novus JSON-RPC JavaScript Client
// Compatible with Node.js (fetch API, Node 18+) or browser
// Run: node examples/rpc_client.js

const RPC_URL = "http://localhost:8545";
let reqId = 1;

async function rpc(method, params = []) {
  const res = await fetch(RPC_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", method, params, id: reqId++ }),
  });
  const data = await res.json();
  if (data.error) throw new Error(`RPC Error [${data.error.code}]: ${data.error.message}`);
  return data.result;
}

async function main() {
  console.log("═══════════════════════════════════════════════════════");
  console.log("  Pecu Novus Blockchain — JavaScript RPC Client Demo");
  console.log("═══════════════════════════════════════════════════════\n");

  // ── 1. Network Info ──────────────────────────────────────────────────────
  console.log("── 1. Network Info ──");
  const info = await rpc("pecu_getNetworkInfo");
  console.log(`Network:   ${info.network}`);
  console.log(`Version:   ${info.version}`);
  console.log(`Chain ID:  ${info.chainId}`);
  console.log(`Consensus: ${info.consensus}`);
  console.log(`TPS:       ${info.tps}`);
  console.log(`Max Supply:${info.maxSupply}`);
  console.log(`EVM:       ${info.evmCompatible}\n`);

  const chainId = await rpc("eth_chainId");
  console.log(`eth_chainId: ${chainId} (decimal: ${parseInt(chainId, 16)})\n`);

  // ── 2. Chain Stats ────────────────────────────────────────────────────────
  console.log("── 2. Chain Stats ──");
  const stats = await rpc("pecu_getChainStats");
  console.log(`Block Height:     ${stats.block_height}`);
  console.log(`Total Txs:        ${stats.total_transactions}`);
  console.log(`Total Burned:     ${stats.total_burned}`);
  console.log(`Mempool:          ${stats.mempool_size}\n`);

  // ── 3. Create Wallets ─────────────────────────────────────────────────────
  console.log("── 3. Create Wallets ──");
  const alice = await rpc("pecu_createWallet");
  const bob   = await rpc("pecu_createWallet");
  console.log(`Alice EVM:   ${alice.evmAddress}`);
  console.log(`Alice Pecu:  ${alice.pecuAddress}`);
  console.log(`Bob EVM:     ${bob.evmAddress}\n`);

  // ── 4. Register Validators ────────────────────────────────────────────────
  console.log("── 4. Register Validators ──");
  const v1 = await rpc("pecu_registerValidator", [alice.evmAddress, "1000000000000000000"]);
  const v2 = await rpc("pecu_registerValidator", [bob.evmAddress, "500000000000000000"]);
  console.log(`Alice Validator: ${v1.nodeId}`);
  console.log(`Bob Validator:   ${v2.nodeId}\n`);

  const validators = await rpc("pecu_getValidators");
  console.log(`Active validators: ${validators.length}`);
  for (const v of validators) {
    console.log(`  ${v.walletAddress.slice(0, 20)}... stake=${v.stake} weight=${v.selectionWeight.toFixed(2)}`);
  }
  console.log();

  // ── 5. Deploy PNP16 / ERC-20 Token ───────────────────────────────────────
  console.log("── 5. Deploy PNP16 / ERC-20 Token (PecuGold) ──");
  const deployed = await rpc("pnp16_deployToken", [
    "PecuGold", "PGLD", 18,
    "1000000000000000000000000", // 1M PGLD
    alice.evmAddress,
    "DAK_DEMO_001"
  ]);
  const contractAddr = deployed.contractAddress;
  console.log(`Contract:   ${contractAddr}`);
  console.log(`Standard:   ${deployed.standard}`);
  console.log(`Supply:     ${BigInt(deployed.initialSupply) / BigInt(10**18)} PGLD\n`);

  // ── 6. ERC-20 Operations ──────────────────────────────────────────────────
  console.log("── 6. ERC-20 Operations ──");

  const aliceBal = await rpc("erc20_balanceOf", [contractAddr, alice.evmAddress]);
  console.log(`Alice PGLD balance: ${BigInt(aliceBal) / BigInt(10**18)} PGLD`);

  // Transfer 10,000 PGLD Alice → Bob
  await rpc("erc20_transfer", [
    contractAddr,
    alice.evmAddress,
    bob.evmAddress,
    "10000000000000000000000"  // 10,000 PGLD
  ]);
  const bobBal = await rpc("erc20_balanceOf", [contractAddr, bob.evmAddress]);
  console.log(`Bob PGLD after transfer: ${BigInt(bobBal) / BigInt(10**18)} PGLD`);

  // Approve & transferFrom
  await rpc("erc20_approve", [
    contractAddr,
    alice.evmAddress,
    bob.evmAddress,
    "5000000000000000000000"  // 5,000 PGLD allowance
  ]);
  const allowance = await rpc("erc20_allowance", [contractAddr, alice.evmAddress, bob.evmAddress]);
  console.log(`Bob's allowance on Alice: ${BigInt(allowance) / BigInt(10**18)} PGLD`);

  const totalSupply = await rpc("erc20_totalSupply", [contractAddr]);
  console.log(`Total supply: ${BigInt(totalSupply) / BigInt(10**18)} PGLD\n`);

  // ── 7. PECU Native Transaction ────────────────────────────────────────────
  console.log("── 7. PECU Native Transaction ──");
  const tx = await rpc("pecu_sendTransaction", [
    alice.evmAddress,
    bob.evmAddress,
    "1000000000000000000000",  // 1000 PECU
    "Payment for services rendered"
  ]);
  console.log(`Tx Hash:  ${tx.txHash}`);
  console.log(`Status:   ${tx.status}\n`);

  // ── 8. Mine a Block ───────────────────────────────────────────────────────
  console.log("── 8. Mine Block (PoT) ──");
  const block = await rpc("pecu_mineBlock");
  console.log(`Block #${block.height}: ${block.blockHash.slice(0, 16)}...`);
  console.log(`Transactions: ${block.txCount}`);
  console.log(`Validator:    ${block.validator.slice(0, 20)}...\n`);

  const blockNum = await rpc("eth_blockNumber");
  console.log(`Current block: ${parseInt(blockNum, 16)}\n`);

  // ── 9. Create Escrow ─────────────────────────────────────────────────────
  console.log("── 9. Create Escrow (MVault) ──");
  const releaseDate = Math.floor(Date.now() / 1000) + 7 * 86400; // 7 days
  const escrow = await rpc("escrow_create", [
    alice.evmAddress,
    bob.evmAddress,
    "5000000000000000000000",  // 5,000 PECU
    releaseDate,
    "Real estate deposit: 123 Blockchain Ave",
    "Release on deed transfer and title recording"
  ]);
  console.log(`Escrow ID:    ${escrow.escrowId}`);
  console.log(`On-chain:     ${escrow.onChainHash.slice(0, 20)}...`);
  console.log(`Amount:       ${BigInt(escrow.amount) / BigInt(10**15)} PECU`);
  console.log(`Release:      ${new Date(escrow.releaseDate * 1000).toISOString()}\n`);

  // ── 10. Transfer Cards ────────────────────────────────────────────────────
  console.log("── 10. Transfer Cards ──");
  const card = await rpc("transfercard_create", [
    alice.evmAddress,
    "1000000000000000000",  // 1 PECU
    null,
    Math.floor(Date.now() / 1000) + 86400, // expires in 24h
    "EventGiveaway"
  ]);
  console.log(`Card ID:   ${card.cardId}`);
  console.log(`Valid:     ${card.isValid}`);

  const redemption = await rpc("transfercard_redeem", [card.redemptionKey, "0xAttendee1"]);
  console.log(`Redeemed:  ${redemption.redeemed}, Amount: ${BigInt(redemption.amount) / BigInt(10**15)} PECU\n`);

  // ── 11. Cold Storage ──────────────────────────────────────────────────────
  console.log("── 11. Cold Storage (CSS) ──");
  const css = await rpc("css_moveToColdStorage", [
    alice.evmAddress,
    "100000000000000000"  // 0.1 PECU
  ]);
  console.log(`Storage Key: ${css.storageKey?.slice(0, 30)}...`);

  // ── 12. Access Keys ───────────────────────────────────────────────────────
  console.log("\n── 12. Access Keys ──");
  const gak = await rpc("gak_connect", [alice.evmAddress, "HootDex", 3600]);
  console.log(`GAK Connected: keyId=${gak.keyId}`);

  const dak = await rpc("dak_register", ["Alice Developer", "alice@dev.com"]);
  console.log(`DAK Registered: ${dak.dakId}`);
  console.log(`KYC Required:   ${!dak.isKycVerified}\n`);

  // ── 13. Tokenomics Summary ────────────────────────────────────────────────
  console.log("── 13. Tokenomics ──");
  const tokenomics = await rpc("pecu_getTokenomics");
  console.log(`Max Supply:     ${tokenomics.maxSupply}`);
  console.log(`Gas Fee Rate:   ${tokenomics.gasFeeRate}`);
  console.log(`Burn Mechanism: ${tokenomics.burnMechanism}`);
  console.log(`Reward Range:   ${tokenomics.validatorRewardRange}`);
  console.log(`Next Halving:   ${tokenomics.nextHalving}\n`);

  const halving = await rpc("pecu_getHalvingSchedule");
  console.log("Halving Schedule:");
  for (const entry of halving.schedule) {
    console.log(`  ${entry.year}: ${entry.maxAnnualRewardPecu.toLocaleString()} PECU/year`);
  }

  // ── 14. Block Explorer style query ───────────────────────────────────────
  console.log("\n── 14. Block Explorer Queries ──");
  const latestBlock = await rpc("eth_getBlockByNumber", ["latest", false]);
  if (latestBlock) {
    console.log(`Latest Block #${parseInt(latestBlock.number, 16)}`);
    console.log(`  Hash:      ${latestBlock.hash?.slice(0, 20)}...`);
    console.log(`  Validator: ${latestBlock.miner?.slice(0, 20)}...`);
    console.log(`  Txs:       ${latestBlock.transactions?.length}`);
    if (latestBlock.potProof) {
      console.log(`  PoT Proof: seq=${latestBlock.potProof.sequenceCount} delay=${latestBlock.potProof.delay}`);
    }
  }

  console.log("\n═══════════════════════════════════════════════════════");
  console.log("✅  Pecu Novus RPC client demo complete");
  console.log("    PNP16 + ERC-20 | PoT Consensus | MVault Escrow");
  console.log("═══════════════════════════════════════════════════════");
}

main().catch(console.error);
