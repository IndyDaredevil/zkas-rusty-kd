# NODE CONTRACT — zKAS node v1.0.5 (consumed binary)
### Revision: r1 · 2026-08-26 · Pinned node release: firecash zkas-v1.0.5 (engine rusty-kaspa v2.0.1, p2p protocol 10)
### Status: DERIVED document. We CONSUME this node; we do not build it (meta-principle 6).
### Revision trigger: adoption of a new official node release. Update protocol: DELETE-then-ADD
### in the project mount (conduct law 2e); filename carries the node version. On any revision,
### re-review BRIDGE-SPEC.md §3 (Engine Contract) — that section cites this document.
### Source-of-truth hierarchy: node source at the pinned release > firecash Pool Integration
### Guide > this document. File paths below are relative to github.com/firecash/zkas-rusty.

---

## 1. IDENTITY & CHAIN PARAMETERS

| Item | Value | Source |
|---|---|---|
| Chain | zKAS (zkas-mainnet), fresh genesis 2026-07-26 | ledger repo map |
| Engine | rusty-kaspa v2.0.1 lineage (Toccata-era) | ledger, banner echo |
| PoW | kHeavyHash, BYTE-IDENTICAL to Kaspa mainnet | Pool Guide §0 |
| Block rate | 1 BPS (target_time_per_block = 1000 ms) | config/params.rs |
| Base unit | 1 ZKAS = 100,000,000 sompi (1e8) | Pool Guide §0 |
| Address HRP | `zkas:` (test `zkastest:`, dev `zkasdev:`, sim `zkassim:`) | crypto/addresses |
| Shielded address version | 9 (ShieldedOrchard), 43-byte raw Orchard SPK, NonStandard script class | crypto/addresses/src/lib.rs, txscript/script_class.rs |
| Coinbase maturity | 100 blocks (~100 s) | constants.rs COINBASE_MATURITY_SECONDS |
| Shielded spendable | 600 blocks (~10 min anchor depth) | params.rs shielded_anchor_depth |
| Hardforks active from genesis | crescendo + toccata `always()`; merged_mining `always()` | params.rs |
| Own hardfork history | ZKAS-NU1 @ DAA 757,000 (multi-producer anchor resolution; dev-fee accrual change) | BL-001/BL-002 |
| Default ports | gRPC 16810, P2P 16811 (Kaspa: 16110/16111 — no clash on one host) | network.rs |

## 2. OUR DEPLOYMENT PINS (Kron)

- Binary: OFFICIAL release zip only (`zkas-v1.0.5` win64 + `zkas-anchor-pins.tsv`).
  Never a source build for consensus duty (meta-principle 6; BL-001 repro was on
  the official binary — provenance question first, always).
- Mode: **archival** (`--archival` MANDATORY for the NU1 snapshot lineage — an
  untested `--yes` would prune; BL-002). `--utxoindex` on.
- Ports: node runs its defaults, 16810 gRPC / 16811 p2p. RPC is **loopback only,
  NEVER forwarded**; p2p 16811 is the only forwardable port (BL-023). Windows
  firewall rules: plain Allow, ALL profiles including Public (adapter
  classification is the classic silent failure).
- Peers: zKAS sweep-validated 16 out / 8 in (BL-022). Knees are hardware/load
  dependent — re-sweep after topology changes.
- Sync health test: "IBD completed successfully" is NOT health. Health =
  advancing DAA score + UTXO-validated > 0 (BL-001). During fork windows:
  re-download artifacts and re-verify sha256 (same filename ≠ same file, BL-002),
  and cross-check against a canonical tip before trusting "synced".
- Anchor pins: `--shielded-anchor-overrides=<file>` — the `=` is REQUIRED
  (BL-001). Historical instrument; NU1 is the durable fix.

## 3. RPC SURFACE WE CONSUME

### 3.1 Template / submit (bridge leg)
- `GetBlockTemplateRequest { pay_address: "zkas:...", extra_data: <extranonce> }`
  — behaves exactly as Kaspa. Node rejects mismatched HRP:
  "invalid prefix ..." = wrong address family for this node
  (rpc/service/src/service.rs, get_block_template_call prefix check).
- `submitBlock`: native blocks need no aux data.
- **CAVEAT (load-bearing):** the plain gRPC `RpcBlockHeader -> Header`
  conversion sets `aux_pow = None` — it DROPS the aux proof. Merged blocks MUST
  be submitted via the **RpcRawHeader / wRPC path**, where `aux_pow` travels as
  a borsh-hex string (rpc/core/src/model/header.rs — aux_pow_to_hex /
  attach_aux_pow). Native blocks are unaffected. Corollary (verified in source,
  kaspa-side enrichment design): `aux_pow` is also STRIPPED from RPC *responses*
  — you cannot read a block's aux proof back over RPC.
- Wire reference: `Header.aux_pow: Option<Box<AuxPow>>`, excluded from hashing;
  p2p BlockHeader.auxPow = field 15; gRPC RpcBlockHeader.auxPow = field 16.

### 3.2 Shielded observation (accounting leg)
- `getShieldedBlocks` — streams shielded effects per block. **Coinbase mints are
  PUBLIC**: recipient and value visible with no viewing key. This is the
  reward-discovery and history-recovery instrument (SESSION-STATE 08-21 §4.5).
- `getShieldedTreeState` — frontier for wallet fast-sync.

## 4. CONSENSUS CONTRACTS THE BRIDGE MUST HONOR
(Cited by BRIDGE-SPEC §3; authority lives here + node source. The three
mandatory deltas vs a Kaspa pool, plus AuxPoW.)

1. **Shielded coinbase output.** Template coinbase pays a version-0 SPK whose
   script is a raw 43-byte Orchard address — NonStandard class. Any
   "coinbase must look like P2PK/P2SH" validation rejects every template.
2. **Coinbase VERBATIM.** 5% dev fee is CONSENSUS-ENFORCED as an appended
   coinbase output (consensus/src/processes/coinbase.rs —
   expected_coinbase_transaction). Rebuilding the coinbase without reproducing
   it byte-for-byte → "coinbase transaction is not built as expected" (native)
   or "block has invalid proof-of-work" (merged, via H_zk mismatch).
   The LAST coinbase output is the dev fee; second-to-last may be red-block
   reward. Never rewrite `outputs.last()` (Pool Guide §3 — the ~80% block-loss
   bug class).
3. **Payload +32 bytes.** zKAS inserts `shielded_root(32)` between `subsidy`
   and the SPK block:
   `blue_score(8) | subsidy(8) | shielded_root(32) | spk_version(2) | spk_len(1) | spk | extra_data`
   Hardcoded extranonce offsets corrupt the payload; parse, don't index
   (serialize/modify/deserialize_coinbase_payload; LENGTH_OF_SHIELDED_COMMITMENT = 32).
4. **AuxPoW.** Block valid if NATIVE (own header clears zKAS target) OR AUX
   (parent kHeavyHash block clears ZKAS'S OWN target and is bound):
   - Binding chain: pow(parent_header) -> parent.hash_merkle_root ->
     parent_coinbase -> H_zk.
   - H_zk is computed over explicit header fields, EXCLUDES aux_pow
     (hashing/header.rs), and commits the zKAS coinbase via hash_merkle_root —
     hence contract (2) above is also the merged-mode contract.
   - Commitment: `MERGE_MINE_MAGIC "ZKMM" (4B) || H_zk (32B)`, EXACTLY ONCE in
     the parent coinbase extra_data. Duplicate/ambiguous commitments rejected
     erring safe (consensus/core/src/auxpow.rs). This is why the bridge's
     tag-suffix sanitizer must reject "ZKMM" (BL-009) — and why the FCMM-pinned
     lockfile killed 100% of merged submissions (BL-005).
   - Verifier: (a) committed_hash(parent_coinbase) == H_zk; (b) merkle branch
     folds to parent hash_merkle_root (MAX_COINBASE_MERKLE_BRANCH = 64);
     (c) kHeavyHash(parent_header, nonce) <= ZKAS target
     (consensus/pow/src/auxpow.rs — verify_aux_pow / check_pow_dual /
     check_pow_gated). Native tried first; a valid native block can never be
     invalidated by malformed aux data.
   - Active from genesis on mainnet (merged_mining_activation = always).
   - Parent need not be a valid Kaspa block; stolen PoW fails binding.

## 5. EMISSION & REWARD FACTS (accounting contracts)

- Initial subsidy 60 ZKAS/block; dev fee 5% skimmed FROM subsidy (miner gets 57
  of 60 initially); halving interval 3 months; two-step perpetual tail
  (6 ZKAS/s to real month 24, then 0.6 ZKAS/s ≈ 18.9M/yr). No fixed max supply.
  (consensus/src/processes/coinbase.rs.)
- Subsidy is STEPPED, not continuously decaying — current observed step
  53.80083582 ZKAS, zero decimal drift across blocks (BL-003). Post-NU1 the dev
  fee accrues per ~1000 DAA rather than per block (firecash deac95c).
- **Mergeset deferred payout (misread twice — BL-003):** a block's coinbase pays
  the miners of its MERGESET, never its own miner. Your reward lands in a LATER
  chain block's coinbase. Explorer block pages show what that block pays OTHERS.

## 6. WALLETD CONTRACT (zkas-walletd v1.0.5, companion consumed binary)

- Deployment: Kron, `127.0.0.1:8501`, loopback only. Passphrase plaintext in
  launch script = accepted single-user-box tradeoff; paper seed is recovery
  (BL-023).
- Auth: `X-Wallet-Token: <wallet-filename-stem>` header (operator-held value —
  NOT recorded in committed docs). **Unauthenticated calls return a ZEROED
  object, not 401** — a zero balance without auth is not evidence of anything.
- Endpoints consumed: `/api/wallet/history` (paginated; coinbase-only filter;
  coinbase entries are RETROACTIVELY complete because mints are public chain
  record — transfers are not), `/api/wallet/balance`, `/api/wallet/consolidate`,
  `/api/status?wallet=<name>`.
- Known mislabel: consolidate outputs can appear as kind=coinbase in history
  (287.58 investigation, SESSION-STATE 08-21 §4.3; firecash bug report pending).
- Shielded tx limits (payout/consolidation planning): 38 spends / 38 actions /
  37 payees per tx; limits count NOTES not coins; proving ≈ 2.4 core-s PER NOTE
  SPENT. Consolidation (`--auto-consolidate`, ceiling 500; `--proof-threads`)
  is the fix, not more cores. (sdk/wallet-engine/src/payment.rs; docs/WALLETD.md.)

## 7. ERROR MESSAGE → CAUSE (operational quick table)

| Message | Cause |
|---|---|
| "coinbase transaction is not built as expected" | Rebuilt/rewrote coinbase (dev-fee output or payload-offset class); OR — historically — the BL-001 anchor wedge on fresh syncs (pre-NU1) |
| "block has invalid proof-of-work" (merged submission) | Committed H_zk ≠ hash of the submitted block; binding failed and merged has no native fallback. Usually same root as above; historically the FCMM lockfile (BL-005) |
| "invalid prefix zkas"/"invalid prefix kaspa" | Address HRP vs node network mismatch |
| Aux accepted by bridge, node says native-only | Activation gate below block's DAA score (not applicable on mainnet — always()) |
| "N disqualified vs. 0 valid chain blocks" on fresh sync | BL-001 anchor-index wedge (pre-NU1 binaries/data) |
| "coinbase mismatch at daa X" from pre-fork binaries | Benign straggler warnings post-NU1 (per dev, BL-002) |

## 8. UPGRADE PROTOCOL (what to do when firecash ships a node release)

1. Read release notes + diff params/coinbase/auxpow source files listed above.
2. Verify sha256 of downloaded artifacts (BL-002 rule; twice during fork windows).
3. Cut NODE-CONTRACT-v<new>.md (DELETE-then-ADD of this file in the mount).
4. Re-review BRIDGE-SPEC §3 (Engine Contract) line by line against the diff;
   banner engine-prefix disagreement with the node engine = rebase-drift alarm
   (BL-030).
5. Canary before fleet (meta-principle 8): +100/canary port offsets, control-rig
   holdout.

## 9. STANDING OPEN ITEMS AGAINST THE NODE (as of r1)

- node v1.0.6 adoption window (carried from SESSION-STATE 08-20/08-21).
- zkas-node memory slope observation (carried).
- walletd consolidate-as-coinbase history mislabel — bug report to firecash.
- Desktop wallet `.dmg` unsealed bundle signature (all releases back to v1.0.6
  of the wallet app) — `.zip` install path is the workaround; report pending.
