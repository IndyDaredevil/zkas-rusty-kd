# Findings — ZKas Consensus Audit (zkas-rusty @ fd86c69, branch ibd-shielded-import)

Static analysis only; nothing built or run. Fork base: upstream rusty-kaspa `33903e0`.
Severity/status per the audit contract. Line numbers verified against HEAD unless noted.

> **REMEDIATION STATUS (working tree, uncommitted, `cargo check` green):** F-01, F-02,
> F-03, F-04, F-05, F-06, F-07, F-08, F-09, F-15, F-17, F-21 (in `24d1d3f`), F-31 are
> patched in the working tree with regression tests. Design notes: F-04/F-05 resolved via a
> maximum anchor age (`pruning_depth/4`, fail-closed); F-02 via binding the import to the
> PP's selected-child coinbase commitment (no header change). F-10..F-14 (bridge seam) stay
> dormant by design. F-23 (launch pin) is a launch-params decision, not code.

---

## 1. CONFIRMED VULNERABILITIES

### F-01 (SH-01/VP-01) — Reorg nullifier reverts are invisible during the re-org up-walk → deterministic cross-node consensus fork
- **Severity: Critical · Confidence: Confirmed (3 independent reviews + parent verification) · Status: Confirmed**
- **Files:** `consensus/src/pipeline/virtual_processor/processor.rs:644` (batch created), `:660` (down-walk `revert_nullifiers_from_store`), `:693` (up-walk re-apply), `:759` (single end-of-walk `db.write`); `database/src/access.rs:93-98` (`has`), `:373-378` (`delete`)
- **Root cause:** commit `603afce` moved reorg nullifier deletes into one `WriteBatch` committed after the walk, claiming "read-visibility within the walk is unaffected". True for inserts (cache.insert → `has`=true), **false for deletes**: `delete` does `cache.remove` + staged DB delete; `has()` misses cache and falls through to RocksDB, where the key is **still present** until line 759. So during the up-walk, abandoned-branch nullifiers still read as spent.
- **Attacker input / preconditions:** ordinary double-spend attempt — same note spent in block A (branch 1) and block B (branch 2); attacker can influence which branch a node sees first via relay timing. No hashpower needed.
- **Attack:** Node X adopts A (nullifier `nf` committed), then reorgs to heavier B. Up-walk validates B while `nf` still reads present → B's spend **dropped**, outcome persisted. Node Y (always on B) **accepts** it. X and Y diverge on `shielded_state_root(B)`; the next block's #24 coinbase commitment turns the divergence into mutual `BadCoinbaseTransaction` rejection → **permanent fork**. Never re-healed: B already has `utxo_diffs`, so it is never re-validated.
- **Violated invariants:** S4, S5, S10, D1, A5. **Classification:** safety / determinism.
- **Why checks don't stop it:** every downstream check (pool-delta, turnstile, #24) is computed from the same locally-persisted wrong state; unit tests commit the revert batch before `compute` and never exercise the uncommitted path.
- **Invalidating conditions:** none found (parent agent verified `access.rs` semantics directly).
- **Remediation:** keep the atomic batch but restore visibility — thread a pending-revert overlay through `LayeredNullifierSet` (`pending || (store && !reverted)`), or commit the down-walk reverts in a first batch before the up-walk begins.
- **Regression test:** branch A spends `nf`, heavier branch B re-spends `nf`; drive node through A→reorg→B and a second node straight to B; assert identical `state_root_at(B)` and accepted sets.

### F-02 (IBD-01) — Shielded IBD import has no PoW-committed binding at the pruning point → malicious syncer + minority miner permanently forks syncee (theft possible)
- **Severity: Critical · Confidence: High · Status: Confirmed (static)**
- **Files:** `protocol/flows/src/ibd/flow.rs:793-843`; `consensus/src/pipeline/virtual_processor/processor.rs:366-385` (`seed_pruning_point_shielded`), `:713-717` (disqualify path); `consensus/src/processes/shielded.rs:494-518` (verify = internal-consistency only); `protocol/p2p/proto/p2p.proto:75-92` (header has `utxo_commitment`, **no shielded root**)
- **Root cause:** import verification proves only that streamed (nullifiers, MuHash, frontier, supply, state_root) are self-consistent — all attacker-controlled. No header commits a shielded root at the PP. The documented "real binding" is the #24 coinbase check on the first child — but a colluding miner simply commits to the **forged** root and passes; honest blocks committing the true root are disqualified (`StatusDisqualifiedFromChain`, persisted). The honest chain disqualifies itself on the victim's node; the attacker needs only minority hashrate (DAA retargets to the attacker's branch).
- **Cheapest variant (pure wedge):** syncer replies "PP has no shielded state" (`data: vec![]`, accepted unconditionally, `flow.rs:811-816`) → victim seeds empty state → all honest children disqualified.
- **Impact:** unbacked mint (inflated `cumulative_coinbase` baseline), double-spend of pre-PP notes (erased nullifiers), permanent fork; disqualified statuses persist in DB — no recovery short of DB wipe.
- **Violated invariants:** S8, S3, S4, A1. **Classification:** safety / trust-model break.
- **Remediation:** commit a shielded state root in the block header (mirror `utxo_commitment`) and check imported `state_root` against the PP header's commitment before seeding; never accept empty metadata without a header-committed empty root.
- **Regression test:** sync from a peer serving (a) forged-but-consistent metadata, (b) empty metadata for a non-empty PP; assert import rejected, `shielded_stable` stays false.

### F-03 (IBD-02) — Honest shielded-state export is self-inconsistent (tip-level set vs PP-level MuHash) → honest IBD fails deterministically
- **Severity: Critical (liveness) · Confidence: High · Status: Confirmed (static)**
- **Files:** `consensus/src/pipeline/virtual_processor/processor.rs:344-360` (`export_pruning_point_shielded` reads MuHash snapshot **at PP**, but `nullifier_count` and the streamed set come from the **current global set at tip**); `consensus/src/processes/shielded.rs:476-487`
- **Root cause:** the global nullifier set is append-only/unpruned and reflects the virtual tip, typically a full pruning period ahead of the PP. One shielded spend in `(PP, tip]` ⇒ streamed set's MuHash ≠ PP snapshot ⇒ receiver-side verify fails (`shielded.rs:506-508`) ⇒ IBD errors, peer dropped, next peer fails identically. **Once the pool is in active use, no honest node can serve a passing shielded state.**
- **Violated invariant:** L3. **Classification:** liveness.
- **Remediation:** reconstruct the PP-time set on export by subtracting `nullifier_diffs` of selected-chain blocks in `(PP, tip]`, or persist a nullifier-set snapshot per PP.
- **Regression test:** node A mines spends, advances PP, mines more spends; node B fast-syncs from A; assert import verifies.

### F-04 (VP-02/CORE-01b) — "Pruned ⟹ final" anchor short-circuit resurrects abandoned-then-pruned anchors → silent inflation + prune-boundary divergence
- **Severity: High · Confidence: Confirmed (code) · Status: Confirmed (logic; live repro recommended)**
- **Files:** `consensus/src/pipeline/virtual_processor/processor.rs:610-612` (`let Ok(source_blue_score) = ... else { return true; }`); enablers: append-only, never-reverted anchor index (`processes/shielded.rs:410-414`); pruning deletes ghostdag/reachability of non-kept blocks (`pruning_processor/processor.rs:506,529`); shielded stores never pruned.
- **Root cause:** the comment assumes a pruned anchor source is "by construction canonical and matured". Matured yes; canonical **no** — the index is written for every block while selected and never reverted, so an anchor produced on a **reorged-out, later-pruned** branch resolves as final.
- **Attack (inflation):** attacker gets block C (with their shielded tx, note N′) briefly selected, reorged out, waits for C to prune, then spends N′ against `anchor_C` with a valid Halo2 proof. `get_blue_score(C)` errors → final → spend applied: value created only on a dead branch enters the canonical pool (turnstile-blind, `value_balance ≥ 0`). Repeatable per orphaned block.
- **Second-order:** nodes just before/after the prune boundary disagree accept/drop on the same merging block (same divergence class as F-01).
- **Violated invariants:** S6, S10, monetary soundness. **Classification:** safety / accounting.
- **Remediation:** below the PP the selected chain is immutable and available — resolve canonicality via selected-chain membership at the PP instead of failing open; and/or revert anchor-index entries in the reorg down-walk (same batch as F-01 fix).
- **Regression test:** mint in C, reorg C out, advance PP past C; assert `is_shielded_anchor_final(anchor_C) == false`.

### F-05 (VP-03/SH-02/CORE-01a) — IBD seed lacks the pre-PP anchor index → full nodes accept old-anchor spends that fast-synced nodes drop → permanent split between node classes
- **Severity: High · Confidence: Confirmed (3 independent reviews) · Status: Confirmed**
- **Files:** `processes/shielded.rs:526-554` (seed writes **only** the PP's own anchor, `:539`); `processor.rs:597-599` (unknown anchor → not final → drop)
- **Root cause:** no maximum anchor age exists (only the 600-block minimum maturity). A spend anchored to a pre-PP canonical root: full node → index hit → pruned source → (F-04 short-circuit) final → **accept**; IBD-seeded node → `anchor_block.get(A)=None` → **drop**. Divergent applied sets → divergent state roots → #24 rejection → fresh nodes permanently fork off.
- **Trigger:** anyone spending a note older than the PP (attacker with an old note + custom wallet, or an ordinary wallet with an old anchor). A full-node miner includes it (valid for them); every fast-synced node forks.
- **Violated invariants:** S5, S6, S8, D1. **Classification:** safety / determinism.
- **Remediation (protocol decision):** either stream the anchor→block index with the IBD import, or enforce a consensus **maximum** anchor age < pruning depth (uniform on all nodes).
- **Regression test:** seed a node, validate a child block with a pre-PP-anchored spend; assert identical outcome to a full node.

### F-06 (AUX-01) — Block level derives from the un-hashed, malleable AuxPoW witness → persistent cross-node consensus partition
- **Severity: High · Confidence: High · Status: Confirmed (mechanism fully traced)**
- **Files:** `consensus/pow/src/auxpow.rs:72-86` (`check_pow_gated` returns level from **parent witness** pow); `consensus/pow/src/lib.rs:104-105`; `consensus/core/src/header.rs:153-165` + `hashing/header.rs:7-30` (aux excluded from block hash); `header_processor/processor.rs:300,383` (level stored, first-witness-wins); `post_pow_validation.rs:56-70` + `parents_builder.rs:42,60-95` (level feeds `check_indirect_parents`); `processor.rs:306-309` (sticky `StatusInvalid`)
- **Root cause:** `H_fc` excludes `aux_pow` by design, but acceptance-time block level is computed from the witness's parent PoW bit-length. Two valid witnesses for the same block (cost ≈ grinding ~2 parent blocks; ~free during the launch window) yield different levels with ~50% probability per variant. `headers_store` is append-only; each node keeps the first witness seen → network splits into level-L1/L2 groups. Children are validated against parents filtered by **stored level** → one group rejects the child in `post_pow_validation` → **permanently `StatusInvalid`** (sticky, not self-healing). Divergent pruning proofs too.
- **Violated invariants:** S2, D1, S9. **Classification:** safety / determinism.
- **Remediation:** make level witness-independent (e.g. derive from native pow or from the target for aux-accepted blocks), or commit `hash(aux_pow)` so one witness is canonical.
- **Regression test:** one aux block, two valid witnesses with different parent-pow bit-lengths; assert same computed level and witness-independent `check_indirect_parents`.

### F-07 (AUX-02) — Remote zero-work panic via unvalidated `CompressedParents` in the borsh-decoded aux parent header
- **Severity: High · Confidence: High (panic certain; blast radius = pipeline stall vs process crash not runtime-verified) · Status: Confirmed (code)**
- **Files:** `protocol/p2p/src/convert/header.rs:118` (raw `borsh::from_slice::<AuxPow>` — no `TryFrom` invariant checks on `parent_header.parents_by_level`); `consensus/core/src/header.rs:18`; panic site `utils/src/iter.rs:42` (`expect("cumulative counts must be strictly increasing")`) via `parent_pow → State::new → hashing::header::hash → expanded_iter`
- **Attack:** craft aux bytes whose parent header has `parents_by_level = [(0,[h])]` (or non-increasing runs); binding check passes without PoW (attacker sets `hash_merkle_root` to fold their crafted coinbase committing the victim block's `H_fc`); `parent_pow` panics **before** any work check. Reachable via block relay and IBD proof/trusted flows.
- **Violated invariant:** liveness (remote crash/stall). **Classification:** availability.
- **Remediation:** validate the aux parent's `parents_by_level` at the p2p edge (strict-increase check or `CompressedParents::try_from` round-trip); make `expand_rle` total as defense in depth.
- **Regression test:** feed malformed aux parents; assert `ConversionError`, not panic.

### F-08 (IBD-03/CORE-02/SH-04) — Peer-controlled `nullifier_count` drives pre-allocation → remote process abort; unbounded in-memory accumulation; slow-trickle sync starvation
- **Severity: High · Confidence: High · Status: Confirmed**
- **File:** `consensus/src/consensus/mod.rs:1745-1748` — `Vec::with_capacity(metadata.nullifier_count as usize)` then unbounded `extend` per chunk.
- TiB-range count → allocation failure → `handle_alloc_error` **aborts** (uncatchable). Below the abort band: multi-GB hold + per-chunk-only timeouts → IBD slot pinned indefinitely. O(N) MuHash runs before any cheap rejection; serving side defeats its own streaming design by materializing the full set.
- **Classification:** availability / resource exhaustion.
- **Remediation:** never pre-allocate from untrusted counts; hard-cap `nullifier_count` (absolute constant or derived bound); fail when streamed > declared; total-transfer deadline.
- **Regression test:** `nullifier_count = 1<<40` then silence → prompt rejection, no crash, retry other peer.

### F-09 (IBD-04) — Poisoned shielded seed permanently wedges the node: no detection, no re-seed, no recovery
- **Severity: High · Confidence: High · Status: Confirmed**
- **Files:** `flow.rs:163-169` (flag set once, never re-checked); `processor.rs:713-717`; `pruning_meta.rs:85-98`
- Seed poison → `shielded_stable=true` → first honest PP child → `BadCoinbaseTransaction` → disqualifications accumulate forever. Nothing attributes failure to the local seed, clears the flag, bans the peer, or warns the operator. Restart doesn't help (DB-persisted).
- **Remediation:** track `BadCoinbaseTransaction` disqualify density below the current PP; above threshold, reset `shielded_stable`, clear shielded stores, and log loudly. (Moot once F-02's header binding exists.)
- **Classification:** liveness / recovery.

---

## 2. LIKELY VULNERABILITIES / DORMANT CRITICALS (bridge seam — inert only while `BRIDGE_ENABLED = false` / peg-in unwired)

### F-10 (PEG-01) — `KaspaBurnProof::claim()` proves inclusion, not destruction → any Kaspa tx (incl. every merged-mining coinbase) is a valid "burn"
- **Severity: Critical if wired · Status: Confirmed dormant (zero non-test callers — dormancy claim verified)**
- `consensus/pow/src/pegin.rs:99-107`: checks only `outputs[0].value != 0` and payload ≥ 32 B; never pins an unspendable burn script, no magic/domain tag (contrast AuxPoW's ZKMM). Every ZKas parent coinbase (miner-controlled payload ≥ 68 B) would auto-qualify → mint ≈ Kaspa subsidy per block. Wiring the seam as the docstrings describe opens unauthorized mint on day one. Related gaps: no Kaspa network/genesis binding, no confirmation-depth rule, no consumed-burn replay set (PEG-02/03, High/Medium latent).

### F-11 (SC-01) — Burn declaration (Kaspa recipient + burn/fee split) not committed by the shielded sighash → miner-malleable peg-out theft
- **Severity: Critical if bridge enabled · Status: Confirmed dormant**
- `shielded-core/src/verify.rs:92-114`: sighash commits the burn *flag bit* and total `value_balance`, never the `burn: Option<(u64,[u8;32])>`. A miner can rewrite the recipient to themselves; all signatures still verify. Fix must hash the declaration into the sighash.

### F-12 (SC-02) — Accepted burn re-mints the full `value_balance` to the miner → two-chain inflation of exactly the burned amount
- **Severity: Critical if bridge enabled · Confidence: Likely · Status: Likely dormant**
- Miner fee accrual uses undivided `value_balance` (`utxo_validation.rs:198`); the pool ledger alone splits burn vs fee (`state.rs:308-310`). ZKas supply unchanged **and** burn receipt claimable on Kaspa ⇒ combined supply +v per peg-out.

### F-13 (SC-03) — "Flip `BRIDGE_ENABLED` to re-enable" is false: burn flag bit is un-verifiable
- **Severity: Medium (upgrade-safety) · Status: Confirmed dormant**
- `verify.rs:157-161` rejects flag bit 2 (`BUNDLE_FLAG_BURN = 0b100`) as non-canonical both directly and via orchard `Flags::from_byte`. Flipping only the const makes peg-outs silently unspendable; re-activation is a real consensus change. Commit message/docs must be corrected.

### F-14 (PEG-04/SC-04/SH-07) — Turnstile totals `cumulative_burns`/`cumulative_pegged_in` are not persisted and not in the state root → activation trap (chain stall / weakened turnstile)
- **Severity: Medium (High if bridge enabled) · Status: Confirmed dormant**
- `model/stores/shielded.rs:226-231` (`SupplyTotals` = coinbase+fees only); `processes/shielded.rs:293-296` (`from_totals` zeroes burns/pegged_in on every reload); `shielded_state_root` omits them. Wiring peg-in/burns without a store+root migration → accounting divergence → `PoolUnderflow` → every later block invalid.

---

## 3. MEDIUM

| ID | Title | Key location | Status |
|---|---|---|---|
| F-15 (IBD-05) | `clear_pruning_shielded_stores` clears only the flag; re-import **unions** stale nullifiers → quiet fund-freeze + fresh-vs-catchup divergence | `consensus/mod.rs:1755-1760`, `flow.rs:210,227` | Confirmed |
| F-16 (IBD-06) | Serving side: O(N) count + full materialization per request, no rate limit; `assert!(sent==count)` panics under concurrent set mutation | `v10/request_pruning_point_shielded_state.rs:52-138`, `processor.rs:351,356` | Confirmed |
| F-17 (SH-03) | `expect("frontier corrupt")` on attacker-controlled IBD metadata (panic contained by `spawn_blocking` JoinHandle per flow.rs:831 → IBD abort, not process crash) | `processes/shielded.rs:141,538` | Confirmed (panic), blast radius Needs verification → Medium |
| F-18 (MP-01) | Spends of already-spent-on-chain notes admitted, relayed, and template-included (nullifier set never consulted before block acceptance) — recurring zero-fee block-space + Halo2-verification griefing | `utxo_validation.rs:674-722`, `processor.rs:1580-1609` | Confirmed |
| F-19 (MP-02) | Anchor existence/finality never checked on mempool path; droppable txs persist (high-priority forever; revalidation doesn't check anchors) | `processor.rs:593-622`, `manager.rs:653-799` | Confirmed |
| F-20 (MP-05) | No shielded-verification cache: admission + every template build + every block validation re-run full Halo2 per tx | `utxo_validation.rs:712-719`, `processor.rs:1580-1609` | Confirmed (magnitude needs benchmarks) |
| F-21 (ECON-01) | `modify_block_template` rewrites the **dev-fee output** ("last output = red reward" is false on ZKas) → every cache-repointed template with red reward is disqualified; **fix exists in working tree but UNCOMMITTED** | `mining/src/block_template/builder.rs:103-106` (committed) | Confirmed — must land before reset binary |
| F-22 (SH-08) | On `shielded_coinbase:false` networks any fee-bearing shielded tx disqualifies the merging chain block (turnstile underflow; template path can't see it) | `utxo_validation.rs:404-411`, `processor.rs:713-718` | Likely (mainnet unaffected) |
| F-23 (PRM-01) | Launch difficulty pin: trivial PoW (~2^17 hashes) for blue ≤ 5000, throttled ramp to 25000 — **live now**; no block in the window is final while open; cheap-block flood surface | `params.rs:854-855`, `difficulty.rs:192-216` | Confirmed (design tradeoff; assess as launch risk) |
| F-24 (SH-05) | Two-phase reorg commit: per-block nullifier inserts committed during walk, reverts at end — crash window leaves disk desync healed only implicitly by re-resolution | `processor.rs:644,732,759` | Needs verification (recovery trigger) |

---

## 4. LOW

- **F-25 (AUX-03)** Aux-witness stripping in relay delays aux-only blocks (recoverable; no `StatusInvalid`). `pre_ghostdag_validation.rs:102-118`.
- **F-26 (AUX-04)** `Uint256::from_compact_target_bits` debug-build panic via attacker aux parent `bits`. `math/src/lib.rs:64-79`.
- **F-27 (SC-06)** Anchor index last-writer-wins + never re-written on rejoin: stale source block can spuriously mark a live anchor non-final (liveness only). `shielded.rs` persist / `processor.rs:597-622`. Needs reorg test.
- **F-28 (SC-07)** `burn > value_balance` invalidates the whole block instead of dropping the tx (dormant; template-poisoning class if bridge on). `state.rs:295-299`.
- **F-29 (SH-06)** Shielded stores never pruned: unbounded disk growth (frontier/MuHash/supply snapshots per block, forever). No delete callers exist.
- **F-30 (MP-03)** Shielded RBF: replacement always rejected (stuck notes until expiry); inclusion-based eviction mismatches the drop model. `replace_by_fee.rs:22-27`.
- **F-31 (ECON-02)** Uncommitted builder fix residual edge: cached miner spk == dev recipient → reverse scan rewrites dev output (self-DoS). Working tree `builder.rs`.
- **F-32 (PRM-02)** Permanent post-ramp difficulty floor at genesis target (soft-wedge if hashrate ever < ~131 kH/s). `difficulty.rs:201-206`.
- **F-33 (PRM-03)** Testnet: inherited Kaspa activation scores, Kaspa subsidy constants, Kaspa DNS seeders on a fresh ZKas genesis. `params.rs:865-923`.
- **F-34 (PRM-04)** Devnet: `toccata_activation: never()` vs its shielded-test purpose; stale ramp comment. `params.rs:1007,1040`.
- **F-35 (PEG-05)** Merkle witness index bits beyond branch length silently dropped — index carries no semantic weight; do not build coinbase-exclusion rules on position. `crypto/merkle/src/lib.rs:155-167`.
- **F-36 (CORE-05)** Trusted IBD blocks bypass the `BadShieldedCoinbasePayout` body check (needs ≥majority-hashrate sync chain). `body_validation_in_context.rs:80-93`.
- **F-37 (IBD-07)** `seed_pruning_point_shielded` never checks `pp == current PP` (latent; no live path found). `processor.rs:366-385`.

---

## 5. INFORMATIONAL

- **ECON-03 (#9 open item):** genesis payload declares subsidy 100,000,000 sompi vs schedule 6,000,000,000 → latent `WrongSubsidy(6e9,1e8)` if genesis is ever body-validated; never paid (genesis in `mergeset_non_daa`), never validated in production. Fix at final re-cut + test. `genesis.rs:101`.
- **ECON-04b:** stale comments (tail "0.6/0.3 FC" vs actual 6/0.6 ZKAS/s; "6 FC/block" vs 60). Code correct.
- **ECON-06:** #24 commitment enforced for **chain** blocks only; mergeset blocks' commitments unchecked (harmless — re-checked if they ever become chain blocks). Amend invariant S5 wording.
- **ECON-07:** zero-fee shielded txs are consensus-valid (fee ≥ 0; feerate is policy-only) — intended.
- **ECON-08:** shielded storage mass is exactly 0 by design (A4); permanent-state growth priced only via compute mass (~≤14 MB/yr) — economics sign-off, not a bug.
- **SC-05/PRM-05:** transparent-layer hash personals are byte-identical upstream Kaspa values (`TransactionID`, `BlockHash`, …) — S9's "personals are zkas_*" is only true for the shielded sighash. PoW personals must stay Kaspa's (merged mining). Re-scope S9.
- **PRM-06:** legacy `firecash` HRP/network-name decode aliases (user-level confusion only; handshake rejects wrong network).
- **PRM-07:** whether turnstile ingests the genesis payload subsidy field (cross-check ECON-03; both conclude genesis is never ingested).
- **AUX-05:** parent kHeavyHash computed twice per aux accept (2× cost amplification). `auxpow.rs:80-81`.
- **AUX-06:** launch-window analysis (no work discount; DAA not gameable via native/aux alternation); `pow_hashers.rs` +449 lines are a std-gated FishHashPlus port used only by KAT tests (dead code, stale comment).
- **PEG-06/07:** merkle leaf/branch domain separation sound; turnstile arithmetic clean (u128 checked); replay_key choice sound.
- **SC-08:** IBD `FrontierState{size:0}` ignores junk fields (harmless; strictness pass suggested).
- **CORE-06/07/08:** shielded tx version gating, address/script classes, DB registry prefixes — all clean.

---

## 6. FALSE POSITIVES INVESTIGATED AND REJECTED (condensed; full list in UNCONFIRMED_ISSUES.md)

- F-01-fees (dropped-spend fee re-mint) — correctly closed via `partition_applied` + reward correction (`utxo_validation.rs:263-296`) and pool-delta check.
- Block-1 halt via genesis reward to 1-byte script — genesis enters `mergeset_non_daa`, never rewarded (`window.rs:155-158`).
- Dev-fee validation — full expected-coinbase hash comparison; cannot be omitted/redirected (ECON-05).
- Mass parity consensus/mempool — same `MassCalculator`, same constant; `action_count_from_bytes` undercharge only for already-invalid txs.
- Bundle decode safety — bounds-checked reader, trailing bytes rejected, 512-cap before allocation.
- Halo2 verification completeness — all actions, binding sig, all spend-auth sigs; batch path unused by consensus.
- "Parent must be Kaspa mainnet / fresh" (AuxPoW) — work is measured against ZKas's own target; no discount possible.
- One-parent-two-blocks / stolen-PoW aux — ZKMM uniqueness + H_fc binding.
- Permanent blacklist via stripped aux witness — isolation-stage PoW failure never writes `StatusInvalid`.
- F-10 pruning-proof aux levels — all paths gated; no native `calc_block_level` remains.
- MuHash `expect` — unreachable given add-only usage.
- Selected-parent shielded replay double-apply — applied exactly once by selected child; later replays conflict-drop deterministically.
- Empty-tree-anchor spends — no real note can prove membership.
- Red-block shielded-fee pool-delta halt — algebra verified for all four mergeset cases (VP FP-1).
- Mempool "conflicting template → over-mint/invalid block" rationale — already closed by fee deduction.

---

## 7. GENERAL HARDENING RECOMMENDATIONS

1. **Make every consensus decision a pure function of replicated data.** F-01/F-04/F-05/F-06 are one bug class: validation outcomes depending on local cache visibility, local pruning progress, local IBD completeness, or local first-seen witness. Adopt as a review checklist rule for any new predicate.
2. Commit a shielded state root into the block header (mirrors `utxo_commitment`) — kills F-02/F-09 and most of the IBD trust surface in one stroke.
3. Protocol decision on anchor age: maximum anchor age < pruning depth (uniform), or full anchor-index transfer in IBD. Resolves F-04/F-05 jointly.
4. Edge validation for nested borsh structures (aux parent header) before any use; make `expand_rle` total.
5. Never pre-allocate from peer-declared counts; stream-verify with early size caps; add total-transfer deadlines to IBD streams.
6. Do NOT re-enable the bridge by flipping `BRIDGE_ENABLED`; F-10..F-14 must be fixed in the same coordinated upgrade, with e2e burn/peg-in tests.
7. Commit the working-tree `builder.rs` fix (F-21) + regression tests before cutting the reset binary; resolve #9 (ECON-03) at the final genesis re-cut.
8. Cache shielded verification verdicts by txid; add admission-time finalized-nullifier and anchor-known checks (fixes F-18/F-19/F-20 cheaply).
9. Launch-window advisory (F-23): no economic finality before blue score ≫ 25,000; consider inbound header-rate soft limit during the pin.
10. Fix stale numeric comments (ECON-04b) and the misleading "flip the switch" (F-13) / "behaviour-preserving" (F-01's commit 603afce) claims — they actively misdirected prior sessions.
