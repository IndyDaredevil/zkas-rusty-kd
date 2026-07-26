# Unconfirmed Issues & Rejected False Positives

## Needs verification (evidence incomplete)

- **SH-05 / F-24** — two-phase reorg nullifier commit crash window: healing depends on `resolve_virtual` always re-walking from the last committed virtual after a crash. Recovery trigger not traced in init path. Likely self-healing; untested.
- **SH-03 / F-17** — `expect("frontier corrupt")` blast radius: SH reviewer assessed node crash; IBD reviewer found the import runs in `spawn_blocking` with a `JoinHandle` (`flow.rs:831`) which contains panics to an IBD abort. Runtime confirmation needed; either way it must return `Err`.
- **AUX-02 / F-07** — panic containment in header-processor thread pool (worker death vs process abort) determines whether it's stall or crash.
- **MP-05 / F-20** — Halo2 verify timing (ms/action) decides Medium vs Low in practice; needs benchmarks.
- **SC-06 / F-27** — anchor-index staleness after reorg rejoin: needs a reorg-level test to confirm the spurious-drop scenario.
- **PRM-07 / ECON-03 cross-check** — whether any accounting ingests the genesis payload subsidy field (both reviewers concluded genesis is never body-validated/rewarded; not traced into turnstile init).
- **ECON-04a** — `tail_subsidy` divides by post-Crescendo BPS unconditionally; fine on all shipped params, implicit invariant if a future network delays Crescendo.
- **PEG-06 interop** — whether `hashing::tx::hash` (FULL encoding incl. crescendo mass-commit/covenants) byte-matches live Kaspa mainnet merkle leaves (liveness risk for future peg-in only).
- **CORE-01/AUX-01 live exploitation** — mechanisms code-confirmed; end-to-end multi-node reproduction not executed (static-only mandate). Recommended: two-node reorg test (F-01), two-witness level test (F-06), IBD seed + pre-PP anchor spend test (F-05), honest-export IBD test with post-PP spend (F-03).

## False positives investigated and rejected

1. Dropped-spend fee re-mint (F-01 in CONSENSUS-CHANGES) — closed by partition + reward correction + pool-delta check.
2. Block-1 halt via genesis 1-byte reward script — genesis in `mergeset_non_daa`, never rewarded.
3. Genesis WrongSubsidy as a production failure — genesis never body-validated; latent/test-only.
4. Dev-fee omission/redirection/overpayment — full expected-coinbase hash comparison prevents all three.
5. Mass undercharge via `action_count_from_bytes` — None→0 only for txs rejected in isolation anyway.
6. Bundle decode panics / trailing bytes / duplicate fields / OOM — checked reader, all rejected properly.
7. Early-exit proof skipping — single proof covers all actions; exhaustive spend-auth loop; batch path unused by consensus.
8. SkipScriptChecks skipping shielded verification — only where prior full validation is guaranteed.
9. AuxPoW: parent must be Kaspa mainnet/fresh — rejected; work measured vs ZKas target, network-agnostic by design.
10. AuxPoW: one parent PoW for two ZKas blocks / stolen PoW — ZKMM uniqueness + H_fc binding.
11. AuxPoW: cached parent hash trusted — recomputed from header fields.
12. AuxPoW: stripped witness permanently invalidates block — isolation PoW failure never writes StatusInvalid.
13. F-10 residual native calc_block_level in consensus paths — none outside tests.
14. Fork-gating on attacker-claimed daa_score (PoW gate / proofs) — enforced in context; claimed-high only forces the harder path on the attacker.
15. MuHash TryFrom expect — unreachable (add-only).
16. Selected-parent shielded replay double-apply — exactly-once by construction.
17. Empty-tree-anchor spends — only zero-value dummies can prove membership.
18. partition_applied vs compute double conflict-resolution — verified no-op (unit test).
19. Dropped tx recording burn receipt / moving anchor — nothing recorded for drops.
20. Anchor entries from reorged-out branches breaking canonicality — handled by try_is_chain_ancestor_of (but see F-04 for the *pruned* variant, which IS a finding).
21. Red-block shielded-fee pool-delta halt — algebra verified all four mergeset cases.
22. total_fees / block_fee subtraction underflows — same-iteration add/subtract of equal values.
23. Duplicate shielded tx across mergeset blocks as inflation — second occurrence dropped + fee-deducted.
24. Mempool shielded orphans — impossible (no inputs).
25. Reorg re-admission gap — inherited upstream behavior, not a fork regression.
26. Wrong-PP seed via flag persistence — all paths re-import both states at one captured PP.
27. Bincode metadata pre-allocation OOM — bounded by 1 GB p2p frame + serde caps (the real vector is F-08's with_capacity).
28. Aux-witness stripping/swapping in pruning proofs — rejection only; inflation needs real work.
29. ZERO_HASH padding in witness construction — verified against calc_merkle_root incl. sparse subtrees.
30. is_canonical_orchard_payout 43-byte prefix slice — exact-43 guard + expected-coinbase hash equality.
31. Coinbase misclassified as shielded / charged shielded mass — disjoint version checks.
32. Check reordering in header/body processors, parents_builder, window.rs — fields/params only.
33. New error variants constructed-but-never-returned — all have verified return sites.
34. Bech32 dual-HRP checksum confusion — checksum over literal HRP.
35. Intra-tx duplicate nullifiers double-add — all-or-nothing DuplicateWithinTx rejection.
36. Shielded template conflicting spends → over-mint — already closed by fee deduction (mempool doc rationale overstated).
37. Ramp direction inverted (difficulty ceiling) — correctly implements max(daa_target, min_target).
38. blue_score attacker-controlled in difficulty — recomputed from node's own GHOSTDAG data.
39. kaspa-mainnet/16111 literals in echo.rs/bin/server.rs — test/example binaries only.
40. Merge-base/aux level inflation via witness swap in proofs — requires real Kaspa work for same H_fc.
