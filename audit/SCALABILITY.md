# ZKas Scalability & Throughput — Analysis and Proposals

Scope: consensus + mining + p2p + wallet-protocol interaction. Goal: more useful
transactions per second (esp. big-pool payout traffic), lighter DAG, faster sends.
Basis: code-verified parameters at `fd86c69` + patch working tree.

---

## 1. Where the bottlenecks actually are (measured from the code)

Live limits (mainnet, Toccata=always ⇒ post-Toccata limits active from genesis):

| Resource | Limit | Shielded cost per action | Capacity |
|---|---|---|---|
| transient mass | 1,000,000 | 3156 B × 4 = 12,624 | **~79 actions/block** |
| compute mass | 500,000 | 3156 + 1000 = 4,156 | ~120 actions/block |
| storage mass | 500,000 | 0 (by design) | unbounded |
| block rate | 1 BPS | — | — |

- A shielded tx = `2837 + 3156·n` bytes (n = actions; 884 B ActionWire + 2272 B proof
  each). Typical payout tx (6–38 notes) = 22–123 KB.
- **Throughput ceiling today: ~79 actions ≈ 79 note-spends per block ≈ 26 small txs/s
  at 1 BPS.** The binding dimension is *transient* (bytes × 4).
- Per-tx ceiling: ~79 actions (transient-bound) — the doc's "38" was pre-Toccata.
- **CPU:** every Halo2 proof (2272 B/action, ~ms each) is verified ≥3× per tx network-wide:
  admission, every template build on every miner (~1/s), every block validation. No cache.
- **Pool pain specifically:** pool receives 1 coinbase note per mergeset block (~60 ZKAS,
  thousands of notes), then spends them in payout txs: proving time (seconds/tx,
  sequential in wallet), 600-block anchor maturity (~10 min at 1 BPS), ~79 notes/tx max,
  and the mined-but-dropped trap (mempool never checks on-chain nullifiers/anchors).
- **DAG weight:** per-block shielded snapshots (frontier ~1 KB, MuHash 384 B, supply,
  148 B/action scan records) are kept *forever, unpruned* — this scales linearly with BPS,
  so raising the block rate without addressing it makes SH-06 10× worse.

---

## 2. Tier 1 — no consensus change, deploy anytime (days)

**T1.1 Halo2 verification cache (txid-keyed).** Verdicts are immutable (sighash binds the
full tx). Verify once at admission; template build and block validation hit the cache.
Removes 2 of 3 verifications network-wide. Prerequisite for any throughput raise.
(F-20 from the audit.)

**T1.2 Batch proof verification at block level.** `shielded-core` has a batch verify API
consensus never uses. Verify all bundles in a block in one Halo2 batch (all-or-nothing —
same validity semantics, deterministic). Big constant-factor win on block validation.
Must be combined with T1.1's per-tx cache so one bad tx is identifiable (fall back to
per-tx verify on batch failure — validity unchanged).

**T1.3 Mempool admission checks for on-chain nullifiers + known anchors** (F-18/F-19).
Kills the mined-but-dropped trap — the single biggest *perceived* "slow/broken send" for
pools: today a payout tx can be mined and silently do nothing. Cheap DB point-lookups.

**T1.4 Pool payout hygiene (wallet/pool software only):**
- batch payouts up to the ~79-action tx cap (today they fragment);
- threshold payouts (accumulate per miner, pay at e.g. 500 ZKAS → 8× fewer actions);
- standard denominations (powers-of-ten notes) so recipient-side spends need fewer notes;
- auto-consolidation in wallet-engine (already the documented follow-up).
Combined effect on payout block-space demand: ~5–10× reduction, zero consensus risk.

**T1.5 Compact-block relay.** Blocks currently relay full bodies; at 1–4 MB blocks this
becomes the propagation bottleneck. Relay short-txid compact blocks (peers already hold
the txs in mempool) — Bitcoin BIP152-style. Pure p2p change, no fork, large latency win
for "sends confirm faster".

**T1.6 Wallet proving pipeline.** Proving is per-action parallelizable; walletd should
prove in parallel and pre-anchor to matured roots. UX latency (seconds → sub-second
submission) without touching consensus.

---

## 3. Tier 2 — hard fork (fits the already-planned reset)

**T2.1 Raise the transient block limit (cheapest big win).** The binding constraint is
bytes×4 against 1M. Raising `new_transient_mass_limit` to 4M → ~316 actions/block
(~105 tx/s at 1 BPS). Bandwidth: ~1.2 MB blocks at 1 BPS ≈ 10 Mbps average — fine.
Keep compute at 500k and let compute become the binding dimension (it's the honest
anti-DoS axis) — but ONLY ship with T1.1+T1.2, or validation CPU becomes the bottleneck.
One-line param change; the analysis is the work.

**T2.2 Raise the block rate (1 → 5–10 BPS).** This exact codebase runs Kaspa at 10 BPS in
production; the DAG mechanics are proven. Linear scaling of everything (≈790
actions/block/s at 10 BPS + T2.1). Bonus: anchor maturity 600 blocks shrinks from
~10 min to ~1 min — the single biggest *send-latency* improvement for users. Costs:
(a) 10× more per-block shielded snapshots — MUST ship T2.5 with it; (b) more red/orphan
traffic (merged mining makes red blocks cheap, fine); (c) IBD/storage growth ×10.
Params: `Bps::<10>` — the codebase is already generic over BPS.

**T2.3 Shrink the memo field (512 → 64 B).** enc_ciphertext is 580 B of every 884 B
ActionWire — a Zcash legacy carrying a memo ZKas barely uses. Cutting to 64 B:
action = 3156 → 2708 B (−14%), +16% actions/block under T2.1. The ciphertext is
outside the circuit (AEAD over note plaintext; the note commitment does not commit the
memo), so this is a wire-format change, not a circuit change — verify against the pinned
orchard 0.14 crate before committing to it.

**T2.4 Recalibrate shielded mass honestly.** Today: bytes×1 compute + 1000 g/action +
bytes×4 transient. Replace the accidental byte proxy with a measured per-action
verification price (e.g. 5000–10000 g/action in compute, calibrated by benchmark) and
charge proof bytes at 1× transient (headers/wire bytes stay 4×). Effect: block capacity
stops depending on proof byte size (future-proof against smaller proof systems), and
relay cost stays priced. Hard fork, small code change, big design hygiene.

**T2.5 Prune per-block shielded snapshots below the finality window** (SH-06 fix).
Frontier/MuHash/supply snapshots are only needed within the reorg horizon
(finality depth ≪ pruning depth); the scan archive (148 B/action) is the only long-term
per-block artifact worth keeping. This is the enabler for T2.2 — without it, 10 BPS is
~1.3 TB/yr of snapshots. Pruning of derived data, consensus-safe if the retention ≥
finality window; keep the anchor index + global nullifier set + PP snapshots forever.

**T2.6 Shielded payload pruning after finality (node-local light mode).** The 3 KB/action
tx payloads are dead weight once the scan archive exists; wallets need only the 148 B
compact records. Let pruning nodes drop shielded tx bodies below finality and serve
`GetShieldedBlocks` from the archive (it already works that way). Makes "light node"
real: DAG storage grows by KB per block, not MB.

---

## 4. Tier 3 — the fresh ideas (hard fork + real R&D)

**X1. Coinbase-native batch payouts — eliminate pool payout traffic entirely.**
The dominant tx load on this chain is pools paying miners. Today: block reward → pool
note → pool crafts payout txs → they compete for block space. Instead, let a coinbase
carry **arbitrary additional shielded payout notes** beyond the fixed blue/red/dev
outputs, with the invariant Σ(outputs) ≤ subsidy + fees (value conservation unchanged;
payouts come out of the miner's own reward share — no new mint path, the turnstile and
#24 machinery are untouched). The pool pays all miners *in the coinbases of the blocks it
mines*: payout traffic disappears from block space, mempool, and Halo2 block validation
(coinbase notes need no spend proofs). Changes: `expected_coinbase_transaction` layout
(fixed prefix + free shielded suffix with value cap), pool software. This is the highest
leverage idea available to ZKas specifically — it deletes the problem instead of
enlarging the pipe. Consensus change; moderate complexity; needs careful coinbase-parsing
hardening (the payout suffix must be length-prefixed, canonical, and excluded from mass
accounting consistently).

**X2. Block-level proof aggregation (halo2 accumulation).** One aggregate proof per block
covering all its shielded actions, produced by the miner; nodes verify ONE proof per
block instead of N. Verification cost per block becomes ~constant; compute mass can
drop correspondingly. Feasible with halo2 accumulation schemes (no new trusted setup —
IPA). Big circuit + prover work; changes tx validity structure (a tx is valid if included
in a valid aggregate). Research track, but this is the industry direction (and the fork
already vendors halo2).

**X3. Shrinking txs: recursive/aggregated bundle proofs.** Long-term: one proof per *tx*
regardless of action count → tx ≈ 1–2 KB flat, actions capped by value not bytes. Same
technology as X2, tx-level. Together X2+X3 make the 22 KB tx a 1 KB tx and block
validation ~free — that's the 100× endgame, and it subsumes T2.3/T2.4.

**X4. Nullifier accumulator with membership proofs (post-MuHash).** The global nullifier
set is unbounded by design (SH-06). Long-term, replace raw membership DB with an
append-only accumulator + succinct membership/non-membership proofs (the MuHash is
already computed per block — the missing piece is witness infrastructure). Research.

**X5. Payment channels over notes.** For pool↔miner recurring relationships (the exact
"big pool" pain): open once, update off-chain with note-spend authorizations, settle
periodically. All client-side; needs only a sensible standard, no consensus change.
Cheap to prototype on top of X1.

---

## 5. Recommended sequencing

1. **Now (pre-launch, no fork):** T1.1, T1.2, T1.3, T1.4, T1.6. These fix the pool pain
   as experienced today and are prerequisites for everything else.
2. **In the reset binary (fork):** T2.5 (mandatory), T2.1, T2.4, optionally T2.2 and
   T2.3 (BPS is a launch-marketing decision as much as a technical one; note T2.2
   *requires* T2.5). X1 if the coinbase-layout work lands in time — it's the biggest
   single win and it only gets harder to fork in later.
3. **Research branch:** X2/X3 (one team, months), X4, X5.

Honest numbers at the end of Tier 2 (1 BPS, no BPS raise): ~316 actions/block
(T2.1) ≈ 100+ payout actions/s with pool batching absorbing most of it; with T2.2 at
10 BPS: ~3,000 actions/s — Visa-scale for this chain's actual traffic profile, before
any proof aggregation.
