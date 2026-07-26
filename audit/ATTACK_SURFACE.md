# Attack Surface (zkas-rusty consensus)

Attacker models: (a) unprivileged network peer, (b) miner with hashrate share, (c) malicious
IBD sync source, (d) Kaspa-side actor (controls Kaspa blocks/burns), (e) shielded user.

## Attacker-controlled inputs

1. **Block headers with AuxPoW** — parent Kaspa coinbase, merkle branches, aux header bytes.
   Risks: wrong-parent acceptance, magic/target bypass, hash-coverage gaps (aux fields not
   committed), level/blue-work gaming.
2. **Peg-in claims** — Kaspa burn proofs (merkle witness + PoW). Risks: forged/replayed
   claims, wrong-Kaspa-network burns, witness malleability, mint exceeding burn.
   (Status: claimed dormant — verify wiring.)
3. **Shielded bundles in `tx.payload`** — actions, proofs, nullifiers, anchors, ciphertexts.
   Risks: double-spend (nullifier across mergeset/reorg), non-final anchor handling,
   verification DoS (512 actions × Halo2), malformed decode panics, value-balance forgery
   (bound by circuit; orchard pinned 0.14.0).
4. **Coinbase txs** — subsidy/dev-fee amounts, `shielded_commitment` field.
   Risks: over-mint, dev-fee redirect, wrong commitment accepted.
5. **IBD shielded-state stream** (p2p msgs 64–67) — frontier, supply totals, nullifier set.
   Risks: malicious syncer seeds wrong state; inconsistent MuHash accepted; wedge of syncee;
   resource exhaustion from giant nullifier stream.
6. **Pruning proofs / headers-with-pruning** — aux-aware levels in proofs.
7. **Standard transparent txs** — inherited sighash/script/mass paths (fork-modified script
   classes, shielded address types).
8. **Reorg triggers** — attacker with hashpower forcing reorgs across shielded state
   mutations; crash-consistency windows.
9. **Difficulty/timestamp fields** — DAA manipulation, timestamp bounds under aux blocks.
10. **Network/activation identity** — network id `zkas-*`, activation DAA scores inherited
    from Kaspa (toccata now always; crescendo etc. — check for dead-but-armed forks).

## High-value targets (risk-ordered)

1. Peg-in mint path (unauthorized minting) — `consensus/pow/src/pegin.rs`, turnstile.
2. Shielded nullifier/double-spend logic in mergeset + reorg — `processes/shielded.rs`,
   `virtual_processor/*`.
3. Coinbase commitment (#24) validation gaps — `processes/coinbase.rs`,
   `body_validation_in_context.rs`.
4. AuxPoW verification — wrong-chain/weak-target acceptance.
5. IBD shielded-state import — sync-time state injection.
6. Supply accounting (subsidy schedule, dev fee, dropped-spend fees, turnstile).
7. Determinism of shielded state root (MuHash, frontier) across nodes.
8. Crash/recovery consistency of shielded stores (WriteBatch atomicity).
9. Verification-cost DoS (Halo2 per action, mass calibration).
10. Genesis/params consistency across the 4 networks.
