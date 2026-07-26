# ZKas (zkas-rusty) — Consensus Architecture

Audit target: `/root/work/rusty-kaspa`, branch `ibd-shielded-import` @ `fd86c69`.
Fork base: upstream `kaspanet/rusty-kaspa` @ `33903e0` (2026-06-24, shallow-clone boundary).
Fork delta: 479 files, +533k/−1k lines (majority = vendored shielded crypto + SDK; consensus delta is much smaller — see CODE_MAP).

## Base layer (inherited from Kaspa, audited upstream)

- PoW **blockDAG**, GHOSTDAG/PHANTOM ordering (k=18), ~1 BPS mainnet target.
- kHeavyHash PoW; DAA via difficulty-adjustment window; pruning (pruning point, pruning proofs, UTXO-set import at PP).
- UTXO model, transaction mass (compute/storage/transient, KIP-9), sighash with blake2b personals.
- Finality = finality-depth window on the selected chain; virtual selected-parent chain drives UTXO state.

## Fork additions (the audit's center of gravity)

1. **Merged mining (AuxPoW) with Kaspa as parent chain** — active from genesis
   (`merged_mining_activation: ForkActivation::always()`).
   - `consensus/core/src/auxpow.rs`, `consensus/pow/src/auxpow.rs`,
     `crypto/hashes/src/pow_hashers.rs` (kHeavyHash aux variants, 449 new lines).
   - Aux-aware block level in header pipeline + pruning proofs (`calc_block_level_gated`,
     `check_pow_gated`). Merged-mining magic `ZKMM`.
2. **Shielded pool (Zcash Orchard / Halo2 fork)** — shielded txs carry an action bundle in
   `tx.payload`; no transparent inputs/outputs.
   - Consensus side: `consensus/src/processes/shielded.rs` (1046 lines),
     `consensus/src/model/stores/shielded.rs` (632),
     `consensus/core/src/zkas_state_binding.rs` (204).
   - Stateless core lib: `shielded-core/` (bundle, action verify, nullifiers, note-commitment
     tree/frontier anchors, MuHash nullifier accumulator, turnstile/supply ledger, burns,
     attestation / canonical-R witness chain, compact scan records).
   - **#24 state commitment**: coinbase commits `shielded_state_root(selected_parent)`; every
     child block validates it. Anchor finality rule (#29/#31): spends anchored to non-final
     roots are *dropped* (liveness) rather than invalidating the block.
3. **KAS↔ZKAS bridge**
   - Peg-in: keyless consensus-mint from a Kaspa burn proof (`consensus/pow/src/pegin.rs`,
     `crypto/merkle` any-index witness, `shielded-core/turnstile.rs` `cumulative_pegged_in`).
     Commit `fd86c69` says mint "stays dormant (never wired)" — verify.
   - Peg-out: deactivated via `shielded-core/src/burn.rs::BRIDGE_ENABLED = false`; burns hard
     rejected in isolation checks + `ShieldedTx::from_bundle`.
4. **Emission / dev fee**
   - 60 ZKAS/s initial subsidy, halving every 3 months, perpetual tail 0.6 ZKAS/block
     (`TAIL_SUBSIDY_FINAL_PER_SEC_SOMPI = 60_000_000`).
   - 5% of subsidy minted as a shielded coinbase note to a dev-fund recipient
     (`ZKAS_DEV_FEE_*` params).
5. **Toccata = always from genesis** — KIP-21 seq_commit + canonical-R attestation active
   from block 0 (`toccata_activation: ForkActivation::always()`).
6. **IBD shielded-state import** — new p2p messages (fields 64–67) transfer
   (frontier, supply totals, nullifier MuHash + full nullifier set) at the pruning point;
   internal-consistency verification, real binding via #24 commitment on first child.
7. **Compact scan-archive** — block-time applied set persisted (148B/action), served via RPC,
   folded into block-commit WriteBatch; smart pruning of shielded payloads.
8. **Mass pricing** — `SHIELDED_MASS_PER_ACTION = 1000` grams added to compute mass per
   shielded action (consensus + mempool recompute).
9. **Ports/network identity** — mainnet P2P 16811, gRPC 16810; network id `zkas-*`;
   re-cut genesis (2026-07-24 anchor).

## Consensus pipeline (block flow)

header_processor (pow/auxpow check, pre-ghostdag validation) → body_processor
(isolation + in-context body validation: merkle root, subsidy, mass, shielded bundle
verification) → virtual_processor (GHOSTDAG ordering, UTXO state, shielded state machine
apply/revert, coinbase commitment validation, finality) → pruning (PP advance, proofs).

Trust model: PoW + GHOSTDAG (no BFT validator set, no slashing, no epochs in the Tendermint
sense; "epochs" here = DAA windows / fork activations).
