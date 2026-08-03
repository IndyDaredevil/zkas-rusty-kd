# Send-Path Deep Analysis — where the time and bytes actually go

Basis: code-verified at `fd86c69` + patch tree. Not estimates from docs — every
constant below is read from source (cited). Empirical benches exist in-tree
(`shielded-core/src/walletdb.rs`, `#[ignore]` benches measuring proving wall-time
vs spend count and concurrent-vs-sequential proving) — run them to pin the two
remaining empirical constants (prove s/action, verify ms/action).

---

## 1. Anatomy of one shielded send (all constants from code)

Wire size: `2837 + 3156·n` bytes for n actions (`bundle.rs:115`; 884 B ActionWire +
2272 B proof per action). Mass: transient = bytes×4, compute = bytes + 1000·n
(`mass/mod.rs`). Live limits: transient 1,000,000 / compute 500,000 / block
(`params.rs:870-871`, Toccata=always).

| Stage | What happens | Cost formula | Typical (38-spend payout tx) |
|---|---|---|---|
| 1. Plan | `plan_payment` greedy value-descending chunks (`payment.rs:147`) | O(notes) | ms |
| 2. Witness | auth path per spent note at a matured anchor (≥600 blocks deep) | warm: O(log T)/note; cold: O(leaves × live witnesses) Sinsemilla hashes (`zkas-walletd/lib.rs:745-776`) | warm: ms; cold for a 5k-note pool: **the hidden killer, see §4.K3** |
| 3. Prove | ONE orchard `create_proof` per bundle covering all actions (`wallet.rs:481`) | ~linear in actions; single-proof rayon efficiency 100/91.5/78% at 1/2/4 threads (bench comment) | **seconds-to-tens-of-seconds — the dominant send latency** |
| 4. Sign | spend-auth + binding sigs over sighash | negligible | ms |
| 5. Relay | full 123 KB tx gossiped | 123 KB × peers | ~1 s |
| 6. Admission | mass checks, then FULL Halo2 verify (`utxo_validation.rs:712`) | ~ms/action | ~0.1–0.4 s |
| 7. Template | re-verify on EVERY template build (~1 BPS ⇒ every second, every miner) (`processor.rs:1580-1609`) | ~ms/action, repeated | repeated 0.1–0.4 s/s while pending |
| 8. Inclusion | next block (~1 s) — IF not silently dropped (mempool never checked the anchor/nullifier vs chain) | — | ~1 s best case |
| 9. Recipient | scans every action network-wide (compact 148 B records), trial-decrypts | O(actions network-wide) per wallet | continuous |
| 10. Spendable | recipient must wait anchor maturity: 600 blocks | 600 × 1 s = **~10 min at 1 BPS** | fixed by consensus |

Fees: `bytes × 200 sompi` (`payment.rs:101-109`: bytes×4/2×RELAY_FEE_PER_KG/1000).
123 KB tx ≈ 0.025 ZKAS. Not a bottleneck.

---

## 2. Worked example: pool paying out 3,000 ZKAS (notes ≈ 60 ZKAS)

Today, honestly computed:

- 50 notes → chunks of ≤39 actions (wallet cap, see §4.K1) → **2 txs**, ~125 KB each.
- Proving: 2 bundles × 38 actions, **sequential** (`zkas-walletd/lib.rs:4168` proves
  chunk-by-chunk in one closure). At even a conservative ~0.5 s/action effective:
  ~40 s of proving. At measured-Orchard rates more like 1–3 min.
- Node-side: each tx verified ≥3× (admission + every template build + block).
- Recipient waits ~10 min before the received notes are spendable (600-block anchor).
- Behind the scenes, the pool's walletd advances ~5,000 note witnesses by every new
  block's leaves — O(leaves × owned-notes) hashes per block, forever (§4.K3).

Everything except "10 min to spendable" is engineering debt, not protocol.

---

## 3. Storage accounting per action (what we store, forever)

| Where | Bytes/action | Prunable? |
|---|---|---|
| block body (tx payload) | 3,156 | yes — after finality, if implemented (T2.6) |
| global nullifier set | 32 | **never** (double-spend prevention) |
| note commitment tree | 32 (amortized; frontier is O(log T)) | frontier only |
| per-block snapshots (frontier ~1 KB, MuHash 384 B, supply) | ~1.4 KB **per block** regardless of content | not today — SH-06; needed only within finality window |
| scan archive | 148 | never (wallet protocol) |
| wallet DB | note + witness metadata | per owned note |

The unbounded items are the nullifier set (irreducible, 32 B/action) and — the actual
problem — per-block snapshots at ~1.4 KB × every block × forever (~44 GB/yr at 1 BPS,
440 GB/yr at 10 BPS — must be pruned to the finality window, see SCALABILITY T2.5).

---

## 4. The five killers, with the math and the fix

### K1 — Wallet packing cap is stale by 2× (one-constant fix, today)
`STANDARD_TX_MASS_CAP = 500_000` (`payment.rs:13`) derives a byte budget of 124,744 →
39 actions/tx. But Toccata=always made the live transient limit 1,000,000 →
`1M/4 − 256 = 249,744` bytes → **79 actions/tx** (compute limit allows 120; transient
binds at 79). The mempool admits 79-action shielded txs TODAY (standard cap for
shielded = block mass limits, `check_transaction_standard.rs`). The wallet self-caps
at 39 → pools craft ~2× the txs, pay ~2× the proving and fees.
**Fix:** derive the budget from the live `block_mass_limits` (transient 1M) instead of
the stale constant; `max_spends_per_tx`/`max_actions_per_tx` go 39 → 79. Zero
consensus risk — the network already accepts these txs. Verify against the mempool's
exact per-dimension check and keep a small safety margin (e.g. cap at 76).

### K2 — Chunk proving is sequential; the parallel harness already exists
`zkas-walletd` proves chunks one-by-one; `walletdb.rs:3467-3527` contains the ignored
bench + thread-pool machinery for concurrent chunk proving, with the measured
parallel-efficiency data (sublinear within one proof ⇒ proving N bundles on N small
pools beats one big pool when N > 1). A 2-tx payout proves 2× faster; an 8-chunk
consolidation ~3-4× on 8 cores. **Fix:** wire the existing scoped-thread + small-rayon-pool
pattern into the send path (prove chunk i+1 while chunk i relays — proving fully
hidden behind relay+inclusion for multi-tx sends). No consensus surface at all.

### K3 — Wallet witness maintenance is O(leaves × owned notes) per block — replace with node-served paths
This is the real big-wallet killer. Every block appends ~60 leaves; every owned note's
witness must advance by every leaf (Sinsemilla per leaf per live witness) — the
walletd background loop is budget-throttled precisely because of this
(`zkas-walletd/lib.rs:691-776`), and note-heavy wallets already degrade to bounded
witness sets + inline climbs at send time.
**But the node already has everything needed:** the global tree and **per-block
frontier snapshots** are in consensus stores (`tree_store` per block). An
authentication path for note at position `p` at any stored block's frontier is
computable server-side in O(log T). Wallets only need note positions — which the
compact scan records (148 B) already give them.
**Fix (new RPC, no consensus change):** `GetShieldedMerklePath(position,
anchor_block) -> path` served from `tree_store` snapshots (+ reachability to confirm
the anchor is canonical/mature — the node has all of it). Wallet sends then need NO
incremental witness state at all: query path at send time, prove, submit. This
deletes the entire witness-warming subsystem's cost model: O(leaves × notes) → O(1)
per block, O(log T) per spend. For a pool: background wallet cost goes from
minutes-per-block to zero. Check first: the 148 B scan record must carry leaf position
(verify; if not, add it — the archive is already consensus-persisted).
Privacy note: the wallet reveals its note positions to its OWN node — zkas-walletd is
node-embedded/self-hosted (`f9bad6e`), so this is trust-neutral for the reference
deployment; remote-wallet use should keep the client-side path as an option.

### K4 — Node verifies every proof ≥3× (admission, every template build, block)
O(actions) Halo2 verification repeated per tx per miner per second while pending.
**Fix:** txid-keyed verification cache (verdicts immutable — sighash binds the tx)
shared by admission/template/block paths + block-level batch verification (the batch
API exists in shielded-core, unused by consensus; fall back to per-tx on batch
failure so a bad tx is still identified). 3× → ~1×, plus batch constant-factor.
(= audit F-20 remediation; prerequisite for any block-size increase.)

### K5 — Block space is transient-bound at ~79 actions/block
Capacity today: 1M/12,624 ≈ 79 actions ≈ 26 small txs/s at 1 BPS. Levers in order of
safety: T2.1 transient 1M→4M (→~316 actions/block; needs K4 first or validation CPU
binds), T2.2 BPS 1→10 (linear; halves *and* cuts anchor maturity 10 min → 1 min;
needs snapshot pruning T2.5 or 440 GB/yr), X1 coinbase-native payouts (removes pool
traffic from the equation entirely — see SCALABILITY.md §4).

---

## 5. Net effect for a big pool (3,000 ZKAS payout example)

| Stage | Today | After K1–K4 (+K5 later) |
|---|---|---|
| txs crafted | 2 (cap 39) | 1 (cap 79) |
| proving wall time | ~40–120 s sequential | ~15–40 s, hidden behind relay (K2) |
| wallet background cost | O(leaves×notes)/block | ~zero (K3) |
| mined-but-dropped risk | real (no admission checks) | rejected at admission (T1.3) |
| recipient spendable | ~10 min | ~10 min (1 min at 10 BPS, T2.2) |
| node CPU per tx | 3× verify | ~1× (K4) |

**Priority order:** K1 (one constant, today) → T1.3 admission checks (correctness of
UX) → K2 (flip on existing code) → K4 (cache+batch) → K3 (new RPC, biggest
engineering item here but transformative for pools) → K5 fork items with the reset.
