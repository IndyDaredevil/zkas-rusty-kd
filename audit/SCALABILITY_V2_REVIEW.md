# Review of SCALABILITY.md / SEND_PATH_ANALYSIS.md / SCALABILITY_V2.md

Third pass. Every load-bearing claim in the three documents was checked against the
code and against measurements taken from the live chain, rather than re-reasoned. This
file records what survived, what did not, and why — including two places where the
reviewer's own earlier notes were wrong.

Basis: working tree at `deac95c`, live mainnet at DAA ~545k, devnet rehearsal chain.

---

## 0. Verdict at a glance

| Item | Verdict | Basis |
|---|---|---|
| K4 verify ≥3× per tx | **CONFIRMED — best idea in the set** | `processor.rs:1948` |
| N6 pre-verify in body processor | **ADOPT** | follows from K4 |
| E4 parent-rate cap at 10 BPS | **CORRECT observation** | `auxpow.rs` |
| E1 one proof per bundle | correct | `expected_proof_len` |
| E3 X1 leaks the payout graph | correct, well caught | coinbase notes are public by construction |
| T1.3 admission checks | **ADOPT** | drop-not-disqualify makes this a silent-failure bug |
| **N1 aux multi-commitment** | **REJECT — unsound** | `auxpow.rs:33-38` |
| **N2 denominated minting** | **REJECT — arithmetic is inverted** | §3 below |
| **K1 wallet cap 39→78** | **REJECT as written — would break relay** | `check_transaction_standard.rs:59,80` |
| K2 parallel chunk proving | **ALREADY SHIPPED**, and 1.21× not 2–4× | `zkas-walletd/lib.rs:913-923, 4138-4152` |
| K3 node-served paths from snapshots | impossible as written | `tree.rs:184` |
| N4 full tree + path RPC | defer — cost overstated 1000×, but leaks spend positions | §6 |
| N5 epoch checkpoints | **already implemented** | `shielded.rs:953` |
| T2.5 snapshot pruning | **SHIPPED 2026-08-02** | `pruning_processor` |

---

## 1. The one finding worth the whole exercise: K4

**Confirmed in code.** `processor.rs:1948` — block-template construction calls

```rust
validate_transaction_in_utxo_context(tx, utxo_view, .., TxValidationFlags::Full, ..)
```

and `Full` reaches `verify_shielded_bundle`
(`tx_validation_in_utxo_context.rs:107`). So **every miner re-verifies every pending
shielded proof on every template build — once per second at 1 BPS** — on top of
admission and block validation.

A txid-keyed verdict cache is sound: the sighash binds the entire transaction, so a
verdict cannot change for a given txid. This is the single highest-value item across all
three documents and it is not yet built.

**N6** (do the first verification in the body processor, in parallel, off the virtual
critical path, and let virtual consume cached verdicts) follows directly and is
architecturally right.

Caveat carried over: if block-level batch verification is added alongside, per-tx
fallback is **mandatory**. `verify.rs:283` returns `ProofInvalid` without identifying
the offending bundle, so "batch fails ⇒ reject block" would convert a droppable
transaction into a rejected block — a direct violation of the drop-not-disqualify
liveness rule adopted after a stale anchor once froze the chain.

Scale note: live traffic is **0.9 shielded actions per block** (720 actions over 800
chain blocks). K4 is correctness-and-headroom work, not present-day relief.

---

## 2. N1 (aux multi-commitment) is unsound — it removes a documented hardening

N1 proposes letting a parent Kaspa coinbase commit a *vector* of `k` ZKas block hashes.
That is precisely the rule `auxpow.rs:33-38` exists to enforce, and the comment states
the attack:

> `MERGE_MINE_MAGIC` must appear in the coinbase payload **exactly once**. This is the
> classic AuxPoW hardening (cf. the Bitcoin merged-mining tag rules): if a miner could
> place two commitments, one parent PoW could be claimed by two conflicting aux blocks.

The consequence is worse than conflicting blocks. Each of the `k` ZKas blocks presents
the same `parent_header` as its proof; each passes its own difficulty check because that
one hash meets the target; each contributes its full work to blue work. **One unit of
hashing becomes `k` blocks' worth of chain weight**, so an attacker with 1/k of honest
hashrate matches honest work.

The distinction the proposal misses: Bitcoin-style merged mining commits a Merkle root
over **multiple different chains**, one block each — sound, because every chain gets one
block per parent PoW. N1 wants `k` blocks on the **same** chain from one PoW, which is
work counterfeiting.

**Salvage:** require the parent to meet a target scaled by `k`. Then `k` blocks cost `k`×
the work — identical to mining them separately — and the parent-slot scarcity of E4 is
still solved. The throughput implication disappears; the slot fix survives. Any future
write-up of N1 must include the scaled target, or it is a consensus-security change
described as "small".

E4 itself stands and is a genuinely sharp observation: at 10 BPS ZKas would need a
commitment in essentially every Kaspa block, with no slack for parent reorgs. The honest
mitigations are native mining filling the gaps, or the k-scaled target above.

---

## 3. N2 (denominated coinbase minting) is inverted, and it worsens the dominant cost

**The arithmetic.** A pool paying 3,000 ZKAS from 57-ZKAS coinbase notes needs
**53 spends**. Split each mint into 50+7 and the largest available denomination is now
*smaller*, so the same payout needs **60 spends of the 50s**. Denominating downward can
only increase the spend count for any payout larger than a single note.

For payouts smaller than a note it is neutral, not better: an Orchard bundle carries
`max(spends, outputs)` actions, so trading a change output for an extra spend saves
nothing. Many-payee batches are output-bound either way.

**And it moves the measured dominant cost the wrong way.** Over 800 live chain blocks:

```
coinbase notes           2,438      (3.05 per chain block)
transaction actions        720
=> the coinbase is 77% of all note-commitment tree growth
```

N2 takes 3.05 notes/block to 6–12. It multiplies the largest measured contributor to
permanent state by 2–4× in order to solve a spend-count problem that it does not solve.

The claim "strictly better than auto-consolidation" is exactly inverted. The lever is
**fewer, larger** notes — which is what consolidation does, and what dev-fee accrual
does (measured: the dev fee is exactly 1.00 note per block, **32.8% of all note
creation**; accrual takes it to ~0.03% at a 1,000-block interval).

---

## 4. K1 (wallet cap 39 → 78) would break relay

The claim is that the wallet self-caps at 39 actions while "the mempool admits 79-action
shielded txs TODAY". It does not.

`check_transaction_standard.rs:59` sets the shielded standardness cap to

```rust
limits.compute.min(limits.transient).min(limits.storage)   // = min(500k, 1M, 500k) = 500,000
```

and line 80 compares **transient mass** against that cap:

```
n = 38 → 122,765 B → transient 491,060  ✓ standard
n = 39 → 125,921 B → transient 503,684  ✗ RejectTransientMass
```

So the wallet's 38 is not a stale constant — it matches relay policy exactly. Raising it
to 78 produces transactions that **no peer relays**, which is a worse failure than the
inefficiency it targets.

The underlying observation is not dead, but the fix is in a different place: the policy
compares *transient* mass against a cap derived from `min()` across **all** dimensions,
conflating axes that exist to price different things. Comparing each dimension against
its own limit would make 78 actions standard (compute 327k < 500k, transient
996k < 1M). That is a **relay-policy change requiring coordinated rollout** across
nodes, not "one constant, hours, zero consensus risk".

---

## 5. K2 is already shipped — and both documents, and this reviewer's own notes, had the number wrong

`zkas-walletd/lib.rs:4138-4152` already proves chunks in concurrent groups, with
`PROOF_THREADS_EACH = 2` and a free-memory guard. The measurement is in the source:

> A single Halo2 proof's parallel efficiency is sublinear — measured on 4 cores at
> 38 spends: 91.7 s at 1 thread, 50.1 s at 2 (91.5 %), 37.6 s at 3 (81 %), 29.7 s at 4
> (77 %). … measured 2×38 spends, 63.9 s sequentially vs 52.9 s concurrently —
> **1.21×**

Three corrections at once:

- SEND_PATH_ANALYSIS claims 2× for a two-tx payout and 3–4× on 8 cores. Measured: **1.21×**.
- It describes the harness as unwired ("wire the existing pattern into the send path").
  It is wired.
- The reviewer's own engineering record (`1aug.md` §7.1) states "parallel chunk proving
  is DEAD — one proof already uses 3.13 of 4 cores, there is no idle CPU." Also wrong:
  the sublinearity *is* the headroom, and it was already harvested for 1.21×.

Everyone was arguing from a plausible model instead of the benchmark sitting in the file.

---

## 6. K3 / N4 — impossible as written; the corrected version is cheap but leaks

**K3 is not implementable.** It proposes serving authentication paths from the per-block
`tree_store` snapshots. Those snapshots are *frontiers*:

```rust
pub struct FrontierState { size: u64, leaf: Option<[u8;32]>, ommers: Vec<[u8;32]> }   // tree.rs:184
```

— the right-most path only. An authentication path for an arbitrary historical position
cannot be derived from it. "The node already has everything needed" is false. V2's E2
corrects this honestly and pivots to N4 (store the full tree).

**N4's cost estimate is off by three orders — in its own favour.** Live today:
`noteCount = 1,142,124`. A full tree including internal nodes is ~2× leaves × 32 B ≈
**73 MB**, growing ~4 GB/year at the current 2.1 leaves/block. The document worries about
64 GB and nearly talks itself out of an option that is currently trivial.

**The real blocker is privacy, and it is not mentioned.**
`GetShieldedMerklePath(position, anchor_block)` tells the node **exactly which leaf the
wallet is about to spend**. That hands the node a map of which notes a wallet owns and
when it spends them — precisely the metadata that fuzzy message detection and oblivious
sync exist to eliminate. K3 waves this away as "trust-neutral because walletd is
node-embedded"; on a privacy chain, *not having to trust your own node* is the property,
and remote/light wallets are the direction of travel.

Also relevant: `SubtreeCache` already reduced the witness climb from 29.4 s to 0.36 s at
200k leaves (82×), so the problem N4 solves is far smaller than the documents assume.

---

## 7. N5 — premise is false, and it is already implemented

N5 describes today's IBD as "trust the syncer + catch lies later". `shielded.rs:953`:

```
imported shielded state root does not match the PoW-committed coinbase binding
(import is not anchored to the proof-verified header chain — refusing to seed)
```

The import is already *refused* unless the state root equals the root committed in a
PoW-verified coinbase. That is "verify a checkpoint". What N5 adds is checkpoints more
frequent than the pruning point — and the pruning point is only ~30 hours back, so the
marginal gain is small.

---

## 8. Items already delivered that the documents list as open

- **T2.5 per-block snapshot pruning** — shipped 2026-08-02. Snapshots are dropped where
  the pruner already deletes block data, which is provably safe: they exist to recompute
  a block from its selected parent and to rebuild after a reorg; reorgs cannot cross
  `finality_depth` (43,200) and the pruner walks below `pruning_depth` (108,000). The
  compact scan archive, the global nullifier set, past pruning points and a sparse
  frontier checkpoint every 1,000 DAA are retained.
- **T1.4 auto-consolidation** — shipped.
- **K2 concurrent chunk proving** — shipped (§5).
- **N5** — effectively shipped (§7).

Their storage figure is also 2.7× high: "~1.4 KB per block, ~44 GB/yr at 1 BPS" assumes
one snapshot per *block*. Snapshots are per **chain block**, and at mergeset ≈ 3 that is
0.32/s → measured **1.63 KB/chain block ≈ 16.4 GB/yr**. Right target, unmeasured
constant.

---

## 9. Smaller comments

- **T2.3 (memo 512 → 64 B)** is described as a wire-only change worth ~14%. It changes
  the *action* format, which fragments the anonymity set — the one thing a privacy chain
  must not do casually, and which Zcash's Sprout→Sapling→Orchard→Ironwood history shows
  is permanent. Not worth 14%.
- **T2.4 ("recalibrate shielded mass honestly")** calls transient-mass-on-proof-bytes an
  "accidental byte proxy". Bytes are the honest relay cost, and proofs really must be
  relayed; charging them is not accidental. The proposal is defensible as future-proofing
  against smaller proof systems, but the framing overstates the flaw.
- **T1.5 / N3 (compact / Graphene relay)** are correct as *prerequisites for a capacity
  raise*, not as present-day wins: p50 block body is 411 B.
- **X1** is correctly demoted by E3. Worth restating plainly: it makes the pool→miner
  payout graph public, on a chain whose entire proposition is that it is not.

---

## 10. What to build, in order

1. **Verify cache (K4)** — confirmed real, benefits every miner every second.
2. **N6 pre-verification in the body processor** — gets Halo2 off the virtual critical
   path; pairs with 1.
3. **T1.3 mempool admission checks (nullifier + anchor)** — under drop-not-disqualify a
   stale-anchor or double-spent transaction is mined and then silently does nothing, after
   the sender has paid tens of seconds of proving. This is a silent-failure bug, not just
   a UX nicety.
4. Record **E4** as a precondition on any BPS raise, with the k-scaled-target caveat
   from §2.

Rejected: N1 (unsound), N2 (inverted), K1 as written (breaks relay), T2.3 (anonymity
set). Deferred: N4 (cheap but leaks spend positions), X1 (privacy), X2 (research).

---

## 11. Method note

The pattern across all three documents: strong at enumerating the design space, honest in
self-correction, and consistently wrong on constants — because the constants were read
from source or reasoned about rather than measured or executed. Every rejection above
came from either running the arithmetic to a number or reading a benchmark comment that
was already in the tree.

The reviewer was not exempt: §5 corrects this file's own author on parallel proving.
