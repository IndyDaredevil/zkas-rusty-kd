# Design-Level Consensus Issues (non-adversarial)

Issues that don't need an attacker: design mistakes, incoherent rules, or structural
properties that weaken the consensus model, hurt performance, or degrade usability.
Compiled from the security audit (zkas-rusty @ fd86c69). Severity here = design impact,
not exploitability. **[tradeoff]** = deliberate decision with a real cost; **[mistake]** =
something the design clearly got wrong.

---

## A. Coherence of the consensus model

1. **Validation outcomes depend on local, non-replicated state** [mistake — the core design flaw of this fork].
   Four separate mechanisms decide block/tx validity from data that differs between honest
   nodes: in-walk cache visibility (F-01), local pruning progress (F-04), IBD-seed
   completeness (F-05), first-seen aux witness (F-06). Even with zero attackers, asynchronous
   pruning or mixed archival/pruned node populations diverge. Consensus validity must be a
   pure function of replicated data; this codebase violates that principle in four places.

2. **"Included in a block" ≠ "executed" for shielded txs** [tradeoff, under-communicated].
   A shielded spend can be mined, appear on-chain with a txid, and still be silently
   *dropped* (non-final anchor, nullifier conflict). The recipient sees a txid; no value
   moved; the fee is not paid. This inverts the universal blockchain mental model and pushes
   correctness onto wallets (which must consult the scan archive, not the block). Every other
   chain either executes or rejects.

3. **Two classes of nodes see different consensus-relevant data** [tradeoff → hazard].
   Archival nodes keep everything; pruned nodes prune — but shielded stores are *never*
   pruned and IBD seeds only a subset (anchor index missing). The protocol silently assumes
   all nodes have the same view; it never states which node class is the "real" one.

4. **Genesis block is exempt from its own rules** [mistake, latent].
   The genesis coinbase payload declares subsidy 1 ZKAS (schedule says 60) and a zero
   shielded commitment ≠ the empty-state root. It survives only because nothing ever
   validates genesis. The most important block in the chain contains dead-wrong
   consensus data — a permanent trap for any future validation tool or import path.

5. **The #24 state commitment is enforced only for chain blocks** [tradeoff, inconsistent].
   Mergeset blocks carry an unchecked `shielded_commitment`. Harmless today (re-checked if
   they ever become chain blocks), but the invariant "every block commits to its parent's
   state" is only half-implemented — the commitment is advisory until selection.

6. **Monetary policy is keyed to DAA score, not time** [inherited, worth questioning].
   "Months" (halvings, tail step-downs) are DAA-months. A hashrate shock moves the emission
   calendar — 3-month halvings can arrive early or late in wall-clock terms. For a chain
   whose hashrate is rented from Kaspa merged mining, emission timing is hostage to parent-
   chain conditions.

7. **Transparent transactions still exist alongside the shielded pool** [tradeoff — the
   "transactions not shielded" point]. The chain is privacy-*optional*: ordinary UTXO txs are
   fully visible, the shielded pool is a parallel accounting domain stitched to the
   transparent one by the turnstile. Consequences: (a) metadata leakage for anyone using the
   cheap path; (b) the anonymity set is only as big as actual shielded usage; (c) two supply
   domains means two accounting systems that must be reconciled forever (the entire turnstile
   complexity class exists only because of this split).

---

## B. Economic / accounting design

8. **Permanent shielded state is priced at zero (storage mass)** [tradeoff, mispriced].
   Every action leaves one nullifier + one note commitment that must be kept *forever*; the
   KIP-9 harmonic that prices exactly this for UTXOs structurally returns 0 for shielded txs.
   The only charge is compute mass (a bandwidth/CPU proxy). Permanent disk growth is sold at
   transient prices.

9. **Halo2 verification cost is priced by accident** [mistake-in-waiting].
   Compute mass = bytes × 1 + 1000 g/action. The per-action 1000 g is not calibrated to
   measured verification time; the byte term dominates today only because proofs are big.
   If proofs ever shrink (recursion/aggregation), verification cost stays and the price
   collapses — a designed-in future mispricing.

10. **Zero-fee shielded txs are consensus-valid** [tradeoff]. Fee ≥ 0 is all consensus asks;
    feerate is pure mempool policy. Block space for ~22 KB, milliseconds-to-verify txs is
    free at the protocol level; the network relies entirely on miner policy for pricing.

11. **Dev fee is hardcoded inflexible** [tradeoff]. 5% of subsidy, recipient a compile-time
    constant, deliberately not in OverrideParams. Changing either requires a coordinated
    binary upgrade (effectively a hard fork). Good for trust, bad for governance agility —
    and the constant being wrong is chain-fatal (block 2 halts), so the unit test is
    load-bearing infrastructure.

12. **Perpetual tail emission (0.6 ZKAS/block) with merged-mining security** [tradeoff].
    Security budget rests on Kaspa miners finding it profitable to embed ZKMM payloads; the
    native subsidy alone at tail (~18.9M/yr against unknown market value) may or may not
    matter to them. Liveness is coupled to parent-chain economics that this chain doesn't
    control.

13. **Perpetual inflation feeds a dev fund, not stakers/burners** [note]. 5% of all issuance
    forever, including the tail. At tail the dev fund receives a fixed absolute amount while
    miner rewards shrink — the fee *share* of miner revenue grows over time.

---

## C. Performance / scalability

14. **~5 shielded tx/s network ceiling** [structural]. 500k block mass ⇒ ≤5 shielded txs
    (~30 note spends) per block at 1 BPS. Kaspa fits ~1,600 transparent txs in the same
    budget. Any adoption wave hits this wall immediately; raising the per-tx cap (#4b)
    repackaged bytes but did not raise throughput.

15. **Unbounded, unprunable state growth** [structural]. Global nullifier set grows forever
    (by design), the anchor index grows forever, and per-block snapshots (frontier ~1 KB,
    MuHash 384 B, supply, scan records) are kept for *every* block with no pruning path —
    hundreds of GB/yr at full BPS. "Pruned node" no longer means bounded disk.

16. **Halo2 verification repeated at least 3× per tx** [mistake]. Admission, every block-
    template build on every miner (~1/s), and every block validation — no result cache,
    though verdicts are immutable (txid-keyed). The most expensive per-byte operation in the
    system is also the most repeated.

17. **IBD shielded import is O(N) in RAM and unverifiable at receipt** [design gap].
    The full nullifier set is materialized in memory, hashed, and only *later* bound by the
    first child's coinbase commitment. Contrast the UTXO import, which is bound by a
    header-committed multiset hash. The shielded import was built to a weaker pattern than
    the one sitting next to it.

18. **Miner coinbase = one note per mergeset block** [structural fragmentation].
    At pool scale that's ~60 notes/hour/wallet of ~60 ZKAS each; a 3,000 ZKAS payment needs
    ~50 notes ⇒ multi-transaction shattering even after the 38-note cap. Fragmentation is
    produced by the coinbase design itself and then fought with consolidation machinery.

19. **Every wallet must scan every block** [structural]. Shielded receipts are invisible by
    design, so all wallets scan all traffic (helped by the 148 B/action compact archive,
    itself unbounded). Wallet cost scales with *network* activity, not user activity.

20. **Anchor maturity delay: 600 blocks ≈ 10 min before a received note is spendable**
    [tradeoff]. Finality-gated anchors are sound, but this is a hard UX floor on payment
    latency that no amount of block speed below finality depth can fix.

---

## D. Protocol usability / rule incoherence

21. **No RBF for shielded txs** [gap]. Any same-nullifier resubmission is rejected even at
    higher fee. A stuck tx locks its notes until 24 h expiry — or forever if submitted
    high-priority (revalidation never re-checks anchors/nullifiers). Transparent txs have
    RBF; shielded users strictly regressed.

22. **The mempool is blind to on-chain shielded state** [gap]. Admission never checks the
    finalized nullifier set or anchor existence, so "never-appliable" txs relay, get mined,
    and get dropped (issue #2 made worse). The honest version: spend a 9-minute-old note →
    your tx appears in a block and does nothing.

23. **Reorged-out shielded txs are not re-admitted** [inherited gap, worse here]. Wallets
    must detect and resubmit; combined with anchor canonicity flips after deep reorgs,
    resident txs silently become permanently droppable with no eviction trigger.

24. **Bridge re-activation path is booby-trapped** [mistake-in-waiting]. The docs/commit say
    "flip BRIDGE_ENABLED"; in reality the burn flag is un-verifiable (F-13), the sighash
    doesn't commit the burn (F-11), fee accounting double-counts it (F-12), and the turnstile
    doesn't persist it (F-14). Four separate consensus changes are masquerading as one const.

25. **Test networks can't test the consensus features that matter** [mistake]. Testnet keeps
    Kaspa's activation scores (Toccata ~1.5 yr out, Crescendo mid-chain flip) and Kaspa's
    subsidy constants and DNS seeders; devnet has `toccata = never`. The bridge, seq_commit,
    and post-Toccata rules are untestable on the networks meant for testing.

26. **Aux witness not committed in the block hash** [tradeoff → F-06]. Witness-style blocks
    (two serializations, one hash) bought relay flexibility and paid for it with witness-
    dependent consensus state and malleable relay copies. The Bitcoin lesson (witness must
    not feed consensus decisions beyond validity) was not fully applied.

27. **Consensus relies on Kaspa coinbase payload space** [coupling]. ZKMM + 64 hex chars in
    the parent coinbase is the entire binding. Any Kaspa-side policy/consensus change to
    coinbase payloads (size limits, standardness) directly threatens this chain's liveness;
    there is no fallback commitment location.

---

## E. Operational / recovery fragility

28. **No self-heal for bad shielded state** [gap]. A poisoned or stale IBD seed wedges the
    node permanently (F-09); the only cure is a DB wipe. There is no "shielded state looks
    inconsistent with observed chain → resync" detection, although the disqualify-counter
    signal is right there.

29. **Load-bearing unit tests as launch safety** [process risk]. Chain-halting constants
    (dev-fee recipient canonicality, genesis hashes) are guarded only by unit tests that
    must be re-run by hand on a build box per CONSENSUS-CHANGES.md. No CI gate in-repo
    enforces them at the re-cut.

30. **Misleading authoritative comments** [process risk]. "Behaviour-preserving" (603afce —
    wasn't), "flip the switch" (fd86c69 — can't), "0.3 FC tail" (6/0.6 actual), "same as
    mainnet" devnet comment (10× off). In a consensus codebase, comments are specifications;
    four of them were wrong in safety-relevant ways.
