# NODE CONTRACT — zKAS node v1.0.6 (consumed binary)
### Revision: r1 · 2026-08-28 · Pinned node release: firecash zkas-v1.0.6 @ tag 25be83a (engine rusty-kaspa v2.0.1, p2p protocol 10)
### Status: DERIVED document. We CONSUME this node; we do not build it (meta-principle 6).
### Revision trigger: adoption of a new official node release. Update protocol: DELETE-then-ADD
### in the project mount (conduct law 2e); filename carries the node version. On any revision,
### re-review BRIDGE-SPEC.md §3 (Engine Contract) — that section cites this document.
### Source-of-truth hierarchy: node source at the pinned release > firecash Pool Integration
### Guide > this document. File paths below are relative to github.com/firecash/zkas-rusty.
### v1.0.6 adoption evidence (session 2026-08-28): full tag-to-tag diff read; every
### consensus/pow/crypto/merkle/p2p-convert hunk verified rustfmt/test-only via `git diff -w`.
### §4 contracts UNCHANGED — BRIDGE-SPEC §3 re-reviewed, zero line edits required.

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

- Binary: OFFICIAL release zip only (`zkas-zkas-v1.0.6-win64.zip`; zip binary is
  `kaspad.exe`, deployed as `C:\zkas\node-v106\zkas-node.exe`). Never a source
  build for consensus duty (meta-principle 6).
- **NO VERSION BANNER EXISTS.** `kaspad_env::version()` = workspace
  `CARGO_PKG_VERSION` = `2.0.1` on BOTH v1.0.5 and v1.0.6 — no git hash, no zkas
  version. **SHA-256 of the exe is the ONLY node identity proof** (BL-047's
  banner rail has no node-side analog). Upstream release assets have been
  re-clobbered post-tag before (v1.0.5 era; manual `--clobber` recipe sits in
  their deploy.yaml) — pin the hash AT DOWNLOAD, compare on any re-download
  (BL-002 rule). Deployed identities live in the ledger/session record.
- **Forward-only boundary = commit `e49ce61` (2026-08-06), NOT the release tag.**
  It inserted `coinbase_commitments` mid-struct in bincode scan records:
  pre-`e49ce61` binaries misparse post-`e49ce61` records and run off the end;
  only v1.0.6 (`5d94f7f`) reads a MIXED archive. Our pre-cutover binary probed
  pre-`e49ce61` (`ShieldedHistoryChunk` string absent, 2026-08-28). Rollback
  below v1.0.6 after first v1.0.6 launch = datadir restore ONLY.
- Config: `--configfile C:\zkas\node\zkas-node-v106.toml`, appdir
  `C:/zkas/node-data`. **CONFIG-KEY DROP TRAP (args.rs, verified at tag):**
  `shielded-history`, `verify-shielded-history`, `consensus-diag`,
  `shielded-anchor-overrides`, `externalip` are parsed from TOML then SILENTLY
  DISCARDED (Args assembly takes CLI-only for these, no `.or(defaults)`;
  BL-017 class, pre-existing in v1.0.5). These flags ride the LAUNCHER command
  line only. Corollary: `externalip` in the TOML was dead for the operation's
  whole v1.0.5 life; inbound peering works regardless (16811 forward).
- Launch: ONLY via `C:\zkas\node-v106\run-zkas-node.cmd` (bakes
  `--shielded-history=on`; the `=` is REQUIRED — `require_equals`, same flag
  class as BL-001's overrides). Mirrors the run-rc-merged.cmd law; this
  launcher is the H2 service-migration target.
- Mode: **archival** (`--archival` MANDATORY — BL-002; untested `--yes` would
  prune). `--utxoindex` on. `shielded-history` on (archival default; baked
  explicitly anyway). `perf-metrics = true` (TOML-honored key): node-side
  process/DB counters incl. resident memory into the node log every 10s
  (`perf-metrics-interval-sec` to slow) — the memory-slope instrument.
  `nologfiles = false` — file logging has been ON the whole time; the BL-032
  witness gap was log CONTENT/readership, not absence.
- Ports: 16810 gRPC / 16811 p2p. p2p 16811 is the only WAN-forwardable port
  (BL-023). **rpclisten deployed `0.0.0.0:16810` by decision 2026-08-28**
  (deviation from the historical loopback pin): LAN gRPC scoped by Windows
  firewall to the MacBook IP only (H7). Loopback (bridge→16810) never
  traverses the firewall; rigs speak stratum only — both unaffected by the
  scoping. Firewall rules: plain Allow, ALL profiles including Public.
- Peers: 16 out / 8 in (BL-022 sweep). Re-sweep after topology changes.
- Sync health: "IBD completed successfully" is NOT health. Health = advancing
  DAA score + UTXO-validated > 0 (BL-001). Fork windows: re-download + re-hash
  (same filename ≠ same file, BL-002); cross-check a canonical tip.
- **Anchor pins: FORBIDDEN.** The published pin file is obsolete AND harmful
  (`2e7b3dd`): all 360 entries below the advanced pruning point; the bulk
  variant was MEASURED to corrupt shielded state at DAA 477,656. Never apply.
  (Historical: BL-001; NU1 is the durable fix.)

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
  attach_aux_pow). Native blocks are unaffected. Corollary: `aux_pow` is also
  STRIPPED from RPC *responses* — you cannot read a block's aux proof back.
- Wire reference: `Header.aux_pow: Option<Box<AuxPow>>`, excluded from hashing;
  p2p BlockHeader.auxPow = field 15; gRPC RpcBlockHeader.auxPow = field 16.

### 3.2 Shielded observation (accounting leg)
- `getShieldedBlocks` — streams shielded effects per block. **Coinbase mints are
  PUBLIC**: recipient and value visible with no viewing key. Reward-discovery
  and history-recovery instrument (SESSION-STATE 08-21 §4.5).
- **NEW in v1.0.6 (both additive, wire-compatible with old clients):**
  (a) request flag `metadataOnly` (gRPC field 3; borsh request v2) —
  hash/blue/DAA/timestamp only, skips the scan archive; cursor-discovery fast
  path. (b) `RpcShieldedCoinbaseOutput.commitment` (gRPC field 3, optional
  32B) — the CONSENSUS-COMPUTED note commitment; borsh legacy serialization
  deliberately omits it. Candidate column for the zkas_blocks enrichment
  design: consensus-attested, no client-side derivation.
- Server-side: a page is now ONE `spawn_blocking`, not one per block (was
  ~1.75s of pure scheduling per 1,000-block page) — relevant to any future
  reporter paging, and removes blocking-pool pressure per page.
- `getShieldedTreeState` — frontier for wallet fast-sync.

## 4. CONSENSUS CONTRACTS THE BRIDGE MUST HONOR
(Cited by BRIDGE-SPEC §3. **VERIFIED UNCHANGED v1.0.5→v1.0.6** — every hunk in
coinbase.rs, pow/auxpow.rs, pow/lib.rs, pow/pegin.rs, merkle, pow_hashers,
p2p convert/header.rs is formatting/test-only; `git diff -w` residuals read.)

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
     erring safe (consensus/core/src/auxpow.rs). Hence the bridge tag-suffix
     sanitizer rejecting "ZKMM" (BL-009); hence the FCMM lockfile kill (BL-005).
   - Verifier: (a) committed_hash(parent_coinbase) == H_zk; (b) merkle branch
     folds to parent hash_merkle_root (MAX_COINBASE_MERKLE_BRANCH = 64);
     (c) kHeavyHash(parent_header, nonce) <= ZKAS target
     (consensus/pow/src/auxpow.rs — verify_aux_pow / check_pow_dual /
     check_pow_gated). Native tried first; a valid native block can never be
     invalidated by malformed aux data.
   - Active from genesis on mainnet (merged_mining_activation = always).
   - Parent need not be a valid Kaspa block; stolen PoW fails binding.
5. (Informational, v1.0.6) Fast-sync anchor verdicts gain an attested-blue-score
   fallback that fires ONLY when local ghostdag is missing (below a syncing
   node's pruning point). A synced node's path is bit-identical. Not fail-open:
   unattested sources still rejected (their audit F-04 stands).

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

## 6. WALLETD CONTRACT (companion consumed binary)

- **PINNED DEPLOYED VERSION: zkas-walletd v1.0.5** — deliberately NOT cut over
  with the node (blast-radius separation). v1.0.5 walletd ↔ v1.0.6 node is
  wire-compatible: request/response additions are version-gated / legacy-borsh
  omitted by design. Walletd cutover is its own future window.
- Deployment: Kron, `127.0.0.1:8501`, loopback only. Passphrase plaintext in
  launch script = accepted single-user-box tradeoff; paper seed is recovery
  (BL-023). Launch command line contains `--wallet-secret` — NEVER paste a
  walletd cmdline readback into any record or chat.
- Auth: `X-Wallet-Token: <wallet-filename-stem>` header (operator-held value —
  NOT recorded in committed docs). **Unauthenticated calls return a ZEROED
  object, not 401** — a zero balance without auth is not evidence of anything.
- Endpoints consumed: `/api/wallet/history` (paginated; coinbase-only filter;
  coinbase entries RETROACTIVELY complete — public mints; transfers are not),
  `/api/wallet/balance`, `/api/wallet/consolidate`, `/api/status?wallet=<name>`.
- Known mislabel: consolidate outputs can appear as kind=coinbase in history
  (287.58 investigation, SESSION-STATE 08-21 §4.3; firecash report pending).
- Shielded tx limits: 38 spends / 38 actions / 37 payees per tx; limits count
  NOTES not coins; proving ≈ 2.4 core-s PER NOTE SPENT. Consolidation
  (`--auto-consolidate`, ceiling 500; `--proof-threads`) is the fix.
- **v1.0.6 walletd deltas (queue for its cutover window):**
  (a) BREAKING for `notes` consumers: `/api/wallet/balance` returns `notes`
  ONLY with `?notes=1`; new always-present `note_count`. Check the treasury
  page's field use BEFORE that cutover. (b) Wallet checkpoints move to v8 —
  forward-only; cold-copy the wallet dir first. (c) `--no-custodial` is
  hosted-multi-tenant only (would 403 our consolidate) — NOT for us.
  (d) `--max-concurrent-proves` default min(2,cores) matches current behavior.
  (e) Perf: shared commitment tree, witness climb off the send path, batched
  trial decryption — first sync 656s→~76s claimed.

## 7. ERROR MESSAGE → CAUSE (operational quick table)

| Message | Cause |
|---|---|
| "coinbase transaction is not built as expected" | Rebuilt/rewrote coinbase (dev-fee output or payload-offset class); OR — historically — the BL-001 anchor wedge on fresh syncs (pre-NU1) |
| "block has invalid proof-of-work" (merged submission) | Committed H_zk ≠ hash of the submitted block; binding failed and merged has no native fallback. Usually same root as above; historically the FCMM lockfile (BL-005) |
| "invalid prefix zkas"/"invalid prefix kaspa" | Address HRP vs node network mismatch |
| Aux accepted by bridge, node says native-only | Activation gate below block's DAA score (not applicable on mainnet — always()) |
| "N disqualified vs. 0 valid chain blocks" on fresh sync | BL-001 anchor-index wedge (pre-NU1 binaries/data) |
| "coinbase mismatch at daa X" from pre-fork binaries | Benign straggler warnings post-NU1 (per dev, BL-002) |
| "failed parsing config file ..." at launch | `deny_unknown_fields` on Args — a TOML key the binary doesn't know; fails LOUD (the good direction; check binary↔config version pairing) |
| Wallet sync frozen retrying one block forever | Mixed scan-record layouts served by a pre-`5d94f7f` binary (the `e49ce61` boundary) — serve the archive with v1.0.6+ |
| Older peer closes connection on history request | Pre-v1.0.6 peer receiving `RequestShieldedHistory` — burns one of a fresh-syncer's 8-peer budget; benign to us, fixed by upgrading |

## 8. UPGRADE PROTOCOL (what to do when firecash ships a node release)

0. Provenance first: the release TAG is not the release ASSET (clobber history);
   there is NO version banner — the downloaded zip's sha256, pinned at download,
   is the artifact identity. Probe the asset by HEAD before trusting the page.
1. Read release notes + diff params/coinbase/auxpow source files listed above
   (`git diff -w` to separate formatting churn from semantics).
2. Pin sha256 of downloaded artifacts (BL-002 rule; twice during fork windows).
3. Cut NODE-CONTRACT-v<new>.md (DELETE-then-ADD in the mount, law 2e/2f).
4. Re-review BRIDGE-SPEC §3 line by line against the diff. The bridge banner's
   engine prefix (BL-030) only alarms on ENGINE drift — it is STRUCTURALLY
   BLIND to a zkas-version drift (both read 2.0.1); the §3 re-review is the
   only rail for that.
5. Cold-copy datadir (+ wallet dir if walletd rides along) BEFORE first launch —
   the layout boundary makes rollback a restore, not a downgrade.
6. Canary/health before trusting: advancing DAA + UTXO-validated > 0; deliberate
   `--verify-shielded-history` run.

## 9. STANDING OPEN ITEMS AGAINST THE NODE (as of r1)

- walletd v1.0.6 cutover — own window; §6 delta list is the checklist.
- zkas-node memory slope — INSTRUMENTED as of this cutover (perf-metrics on;
  windows_exporter deployment still pending, H6).
- BL-032 wedge — root cause OPEN; v1.0.6 fixes nothing here (verified: no RPC
  lifecycle change in the range). Upstream report ships the class-level
  observation (RPC sessions can starve behind the fair session lock, no fault
  raised); the backfill-deadlock candidate was RETRACTED (unreachable in
  shipped builds; code absent from our pre-cutover binary, probed 08-28).
- externalip config key dead on this binary (config-key drop class) — candidate
  line for the upstream report; low impact, inbound peering works.
- walletd consolidate-as-coinbase history mislabel — report to firecash.
- Desktop wallet `.dmg` unsealed bundle signature — `.zip` path workaround;
  report pending.
