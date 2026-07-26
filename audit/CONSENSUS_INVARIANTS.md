# Consensus Invariants (zkas-rusty)

## Safety

- S1. No two conflicting blocks are both final (GHOSTDAG finality-depth rule, inherited).
- S2. Every accepted block satisfies PoW: native kHeavyHash *or* a valid AuxPoW against the
  Kaspa parent chain with correct magic (`ZKMM`), target, and merge-mined header binding.
- S3. Total ZKAS supply = Σ coinbase subsidy (schedule) + Σ peg-in mints − Σ peg-out burns.
  No other mint path exists. Dev fee ≤ 5% of subsidy, only to the dev recipient.
- S4. A shielded nullifier is accepted at most once across the entire DAG history
  (double-spend impossible, including across mergeset blocks and reorgs).
- S5. Every block's coinbase `shielded_commitment` equals
  `shielded_state_root(selected_parent)` as recomputed by the validator (#24).
- S6. Shielded anchors referenced by spends must be canonical, matured (final) tree roots
  (#29/#31); non-final anchors → spend dropped, block remains valid.
- S7. Peg-in mint requires a valid Kaspa burn proof: real Kaspa block, sufficient PoW,
  merkle inclusion of the burn tx, correct domain separation; one burn → one mint (no replay).
- S8. Imported shielded state at IBD (frontier, supply totals, nullifier set) is consistent
  with the pruning point's committed roots and is bound by #24 on the first child block.
- S9. Block/tx hashes commit to all consensus-relevant fields (no unsigned malleability);
  sighash personals are `zkas_*` (no cross-chain replay with Kaspa or old firecash).
- S10. Reorg apply/revert leaves UTXO set, nullifier set, tree frontier, and supply totals
  exactly consistent with the new selected chain (atomic, crash-safe).

## Liveness

- L1. A dropped shielded spend (non-final anchor / nullifier conflict) does not disqualify
  the containing block.
- L2. Difficulty adjustment converges under merged mining; no DAA wedge from aux blocks.
- L3. IBD completes from honest syncers; malformed shielded-state streams are rejected
  without poisoning sync (can retry another peer).
- L4. Mempool policy never makes a consensus-valid tx permanently unmineable (cap
  exemptions consistent), and never admits consensus-invalid txs to blocks.
- L5. No unbounded queues/caches from attacker-controlled shielded/p2p input; per-tx
  verification cost bounded by mass accounting (512-action cap, per-action mass).

## Determinism

- D1. State root, nullifier accumulator (MuHash), and tree frontier are order-independent
  functions of the accepted action set; all nodes derive identical roots.
- D2. No system-time, float, hashmap-order, or architecture dependence in state transition.
- D3. Genesis state identical on all nodes (fixed genesis hash per network).

## Accounting

- A1. Turnstile: cumulative_coinbase + cumulative_pegged_in − cumulative_fees − burns ≥ 0
  at every block; shielded value creation bounded by transparent-side accounting.
- A2. Fees: dropped spends do not re-mint fees (F-01); accepted spends' fees accrue to the
  coinbase exactly once.
- A3. Subsidy schedule (60/s start, 3-month halvings, 0.6 tail) and 5% dev fee enforced in
  `body_validation_in_context` for every block including aux blocks and edge heights.
- A4. Mass accounting: shielded per-action mass (1000g/action) identical in consensus and
  mempool; storage-mass zero-for-shielded is a deliberate (documented) asymmetry, not a bug.
- A5. Reorg WriteBatch atomicity: crash mid-reorg cannot persist a partial nullifier delta
  (commit `603afce`); recovery re-derives consistent state.
