# Review Progress

Target: `/root/work/rusty-kaspa` @ `fd86c69` (branch ibd-shielded-import). Scope: consensus only.
Fork base: upstream rusty-kaspa `33903e0` (shallow boundary; diff = 479 files, consensus delta much smaller).
Method: 10 parallel deep-review tracks + parent verification of headline findings. Static only.

## Status: COMPLETE (all assigned consensus-critical code reviewed or explicitly listed)

## Directories reviewed

| Dir | Files | Status |
|---|---|---|
| consensus/pow | auxpow.rs, pegin.rs, lib.rs | reviewed (tracks 1,2) |
| consensus/core | auxpow, mass, config/{params,genesis,network}, tx, header, zkas_state_binding, coinbase, errors, hashing | reviewed (tracks 2,6,8,10) |
| consensus/src/processes | shielded.rs, coinbase.rs, difficulty.rs, pruning_proof/*, sync/mod.rs, window.rs, parents_builder.rs | reviewed (tracks 3,6,5,8) |
| consensus/src/pipeline | virtual_processor/*, header_processor/*, body_processor/*, pruning_processor (touch points) | reviewed (tracks 2,4,6,10) |
| consensus/src/model/stores | shielded.rs, pruning_meta.rs, headers.rs | reviewed (tracks 3,5) |
| consensus/src/consensus | mod.rs, services.rs | reviewed (tracks 5,10) |
| crypto | hashes, merkle, txscript, addresses, muhash | reviewed (tracks 1,2,7,10) |
| shielded-core | bundle, verify, state, turnstile, nullifier, tree, commitment, burn, attestation, witness_chain, coinbase, payment_check | reviewed (track 7) |
| protocol/flows | ibd/{flow,streams}, v10/* | reviewed (track 5) |
| protocol/p2p | proto, convert/header.rs, payload_type, codec limits | reviewed (tracks 2,5,10) |
| mining | mempool/*, block_template/builder.rs | reviewed (track 9) |
| database | access.rs, registry.rs, cache.rs | reviewed (tracks 3,10) |
| math | lib.rs, uint.rs (touch points) | reviewed (track 2) |
| inherited DAG core (ghostdag, reachability, finality, transaction_validator) | spot-checked at fork touch points; full upstream re-audit out of scope | spot-reviewed (tracks 4,7,10) |

## Documentation reviewed

- CONSENSUS-CHANGES.md (claims-to-verify; several verified, F-10 confirmed, #9 characterized, 603afce claim found FALSE → F-01)
- CONSENSUS-INHERITED.md (items #1-4 resolved in code as claimed; storage-mass-zero confirmed by design)
- README.md (architecture only)
- Operator note: some docs stale — confirmed (e.g. "flip the switch" F-13, stale tail numbers ECON-04b)

## Important call paths traced

- block: p2p convert → header isolation (aux PoW gate) → pre/post-pow validation → body isolation/context (subsidy, payout guard) → virtual (ghostdag, utxo+shielded compute, #24 verify) → commit
- reorg: calculate_utxo_state_relatively down/up walks + shielded nullifier batch (F-01 found here)
- IBD: 3 paths → pruning proof validate/apply (aux-gated levels) → UTXO import → shielded export/stream/verify/seed (F-02, F-03, F-05, F-08 found here)
- shielded tx: isolation → utxo-context (bundle verify, fee=value_balance) → partition_applied (anchor finality, nullifier conflicts) → apply/drop → coinbase reconstruction
- coinbase: subsidy schedule → dev-fee skim → mergeset rewards (F-01 fee correction) → expected tx hash compare
- aux: ZKMM binding → merkle fold → parent kHeavyHash recompute → level derive (F-06 found here)

## Confirmed findings

See FINDINGS.md: 4 Critical-class (F-01, F-02, F-03 + dormant F-10/F-11/F-12), 6 High (F-04..F-09), 10 Medium, 13 Low, ~14 Info.

## False positives rejected

40 items — see UNCONFIRMED_ISSUES.md.

## Incomplete-evidence areas

- Live multi-node reproduction of F-01/F-03/F-04/F-05/F-06 (mechanisms code-confirmed; static mandate)
- Panic containment details (F-07, F-17) — thread-pool vs process
- Halo2 verification timings (F-20 magnitude)
- Kaspa-mainnet leaf-hash interop for future peg-in (PEG-06)
- Working tree has UNCOMMITTED changes: mining/src/block_template/builder.rs (the F-21 fix) — audit covered both committed and working-tree versions

## Unreviewed (with reason)

- orchard 0.14.0 / halo2 crate internals — pinned crates.io deps, assumed upstream-audited; not diffed
- risc0 guest + Kaspa covenant side of canonical-R/peg-out — different repository (vprogs-zkas)
- Full re-audit of inherited upstream Kaspa DAG consensus — mature audited code; fork seams all covered
- wallet/, sdk/, gateway/, zkas-api, zkas-relayer, pool, explorer — excluded by operator instruction
