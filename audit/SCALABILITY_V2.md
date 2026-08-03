# Scalability V2 — critical re-analysis, corrections, and better ideas

This document **corrects** SCALABILITY.md and SEND_PATH_ANALYSIS.md where a second,
deeper pass found them wrong or oversold, and adds the stronger ideas that fell out of
that correction.

---

## 1. Errata — what I got wrong

**E1. "X3: one proof per tx" — already exists, I was wrong.**
Orchard's `Builder::build` + `create_proof` produces ONE Halo2 proof per *bundle*
covering all its actions (`wallet.rs:481`). Proof size is `2720 + 2272·n` (fixed +
per-action). So transaction-level aggregation is done; the only open aggregation
frontier is **block-level** (X2). Remove X3; X2 stands.

**E2. K3 (node-served witnesses) was oversold — asymptotics don't move the way I said.**
Merkle authentication paths fundamentally need updating as the tree grows; moving the
work from wallet to node changes nothing asymptotically *if the node also maintains
per-note incremental witnesses*. The honest version that actually works: the node
stores the **full note-commitment tree** (32 B/leaf, ~2× with internal nodes ≈
64 B/action — the same order as the nullifier set we already keep forever,
memory-mapped), and computes ANY path on demand in O(log T). *That* deletes the
wallet witness subsystem for real. Cost model: linear storage we already accept for
nullifiers; no per-block per-note work anywhere. Revised as idea **N4** below.
(Unchanged precondition: the 148 B scan record must carry the leaf position — still
needs verification.)

**E3. X1 (coinbase-native payouts) has a privacy cost I didn't disclose.**
Coinbase notes are created from *public* data (recipient, value, rho/rseed are
recomputed by every validator — that's how the mint is authorized). Moving pool
payouts into coinbases therefore makes the pool→miner payout graph **public on-chain**
(recipient addresses and amounts), where today payouts are fully shielded txs. X1
still deletes the traffic, but it is a privacy-for-throughput tradeoff, not a free
win. Decision belongs to the operator; **N2 below achieves part of X1's benefit with
zero privacy loss.**

**E4. T2.2 (10 BPS) has a merge-mining cap I didn't analyze.**
One parent Kaspa coinbase commits exactly one ZKas block hash (`ZKMM` + one `H_fc`).
At 10 BPS, ZKas wants 10 aux slots/s — exactly Kaspa mainnet's own 10 BPS: zero slack
for parent reorgs or competing ZKas tips, and native mining must fill any gap. Not a
blocker (the DAG tolerates native+aux mix), but "just raise BPS" silently couples
ZKas's max rate to the parent's. Fix is cheap and is idea **N1**.

**E5. T2.1 (transient 4M) needs a propagation prerequisite I under-weighted.**
4M transient = ~1 MB of tx bytes per block, every second, gossiped as full bodies —
without compact-block relay (T1.5) that *becomes* the new bottleneck and raises
orphan rates. T2.1 without T1.5 is a half-fix.

---

## 2. Better ideas that fell out of the corrections

### N1. Aux multi-commitment: `ZKMM + k·H_fc` per parent coinbase (hard fork, small)
Allow the parent coinbase payload to commit a *vector* of ZKas block hashes
(length-prefixed, k bounded, e.g. ≤ 8; each hash folds into the aux proof with its
index). Effect: the parent chain stops being the rate limiter for ZKas BPS raises
(E4), parent reorgs strand fewer ZKas blocks (k tips can share one parent), and the
merge-mining protocol gains an explicit many-children semantic. This is the enabling
change for any serious BPS future — and it's a contained auxpow.rs + embed/verify
change with a genesis re-cut.

### N2. Denominated coinbase minting — kill fragmentation at the SOURCE (hard fork, small)
The pool-fragmentation root cause: every coinbase mints ONE note of ~subsidy value
(~60 ZKAS), so every downstream payment shatters into many note-spends. Instead, the
coinbase mints the reward as a **fixed denomination ladder** (e.g. binary or
1-2-5 decomposition: 60 ZKAS → notes of 50+10), directly into the coinbase mint logic
(`build_coinbase_mint`/`expected_coinbase_transaction` — fully deterministic, so the
byte-exact coinbase validation property is preserved, unlike X1). Cost: ~2–4
commitments per block instead of 1 (tree grows slightly faster); privacy unaffected
(miner payout amounts were already public). Effect: payments need ~3–5× fewer
note-spends → fewer actions → less proving, smaller txs, less block space. **This is
strictly better than auto-consolidation** (which spends fees and block space to repair
the fragmentation after the fact) and takes most of X1's throughput benefit without
X1's privacy loss (E3). Cheapest throughput-per-line-of-code in this document.

### N3. Graphene-style set reconciliation for block relay (no fork)
At 100 KB+ txs, compact blocks (T1.5) still ship ~1–5% of bytes + ordering info;
Graphene (IBLT + Bloom) ships O(√n·txid) and reconciles against the receiver's
mempool. For 22 KB txs this is the difference between ~10 KB and ~1 KB per block on
the wire. Prerequisite-grade change for any block-size raise (E5). Implement T1.5
first if time-boxed; N3 if doing it properly.

### N4. Full commitment tree as consensus infrastructure + path RPC (no consensus change)
Revised K3 (E2): keep the full Orchard note-commitment tree in the node
(memory-mapped, ~64 B/action all-time), maintained incrementally at persist time
(append-only, crash-consistent with the same WriteBatch). Serve
`GetShieldedMerklePath(position, anchor_block)` from it in O(log T). Wallets then
need: note positions (scan records) + one RPC per spend. Deletes: witness warming,
witness budgets, bounded-witness-set degradation, inline climbs at send time — the
entire O(leaves × owned-notes) cost class, for every wallet, forever. Storage honesty:
at 1 B action-year scale this is ~64 GB — significant but bounded, and it *replaces*
per-wallet witness data that costs more in aggregate. Do a memory-map with a
right-side hot suffix in RAM.

### N5. Epoch shielded checkpoints → trustless fast-forward sync (hard fork, builds on F-02 fix)
The F-02 remediation binds shielded state to a coinbase commitment. Generalize: every
E blocks (e.g. E = finality depth), the epoch-boundary block's commitment becomes a
**checkpoint root**; a new node can fast-forward to the latest checkpoint, import the
state at it (verified against the checkpoint, not trusted from a peer), and validate
only forward. Turns IBD from "trust the syncer + catch lies later" into "verify a
checkpoint". Same machinery as the F-02 fix, amortized.

### N6. Parallel pre-verification pipeline for shielded txs (no fork)
Today Halo2 verification happens inside UTXO-context validation on the virtual path
(serial, on the critical path of block acceptance) — plus template re-verification.
Move FIRST verification to the body processor (which is already off the virtual
critical path and parallel-friendly): verify each block's bundles in rayon-parallel
as bodies arrive, cache verdicts by txid (= K4's cache), and let the virtual path
consume cached verdicts. Block acceptance latency for a 79-action block drops from
"verify 79 actions serially on the virtual thread" to ~pre-verified by the time
virtual needs it. Deterministic — same checks, same order of acceptance decisions,
just computed earlier and concurrently.

### N7. Lean-node profile (formalizes T2.6)
A node mode that stores: headers, ghostdag/reachability, UTXO set, nullifier set,
full commitment tree (N4), scan archive — and NO block bodies below finality. It can
fully validate NEW blocks (it verifies incoming txs) and serve wallets, but cannot
serve historical bodies. This is the realistic "lightweight DAG" endpoint: steady-state
disk grows by ~(32 + 64 + 148) B/action + snapshots-in-window, not ~3.2 KB/action.

---

## 3. Re-ranked program (effort vs. payoff, honestly)

| # | Item | Fork? | Effort | Payoff |
|---|---|---|---|---|
| 1 | K1 wallet cap 39→78 actions (stale constant) | no | hours | 2× pool packing, today |
| 2 | K4 verify cache + block batch verify | no | days | node CPU ~3×; prerequisite for everything |
| 3 | N6 parallel pre-verification in body processor | no | days | block-accept latency off the critical path |
| 4 | T1.3 admission nullifier/anchor checks | no | days | kills mined-but-dropped UX bug |
| 5 | N2 denominated coinbase minting | **fork** | small | kills fragmentation at source; ~3–5× fewer spends/payment |
| 6 | N4 full tree + path RPC | no | 1–2 wks | deletes wallet witness cost class |
| 7 | T1.5/N3 compact/Graphene relay | no | 1–2 wks | enables any block-size raise |
| 8 | T2.1 transient 4M (after 2,3,7) | **fork** | 1 line + analysis | ~4× block capacity |
| 9 | N1 aux multi-commitment (before any BPS raise) | **fork** | small | removes parent-rate cap |
| 10 | T2.2 BPS raise (after 7,8,9 + T2.5 snapshot pruning) | **fork** | params + soak | 10 min→1 min respend latency, 10× throughput |
| 11 | N5 epoch checkpoints | **fork** | medium | trustless fast sync |
| 12 | N7 lean-node profile | no | medium | bounded-disk full validation |
| 13 | X2 block-level proof aggregation | **fork** | research (months) | the 100× endgame |
| 14 | X1 coinbase-native payouts | **fork** | medium | only if the payout-privacy tradeoff (E3) is accepted; N2 first |

**If only three things ship:** 1, 2, 5. They cost almost nothing and address the
three real costs: packing, node CPU, and fragmentation-at-source.

---

## 4. What survived scrutiny unchanged

- T1.1/T1.2 (verify cache + batch) — correct and still top-3.
- T1.3 admission checks — correct; also a UX-correctness fix.
- T1.4 pool hygiene (batching, thresholds, denominations wallet-side) — still valid;
  N2 makes part of it unnecessary.
- T2.3 memo 512→64 — still valid (wire-only, verify against orchard crate), ~+16%.
- T2.5 snapshot pruning — still mandatory for any BPS raise.
- X2 block-level aggregation — still the endgame (now the *only* aggregation item, E1).
- The core math: 12,624 transient-mass/action, ~79 actions/block today, transient-bound.
