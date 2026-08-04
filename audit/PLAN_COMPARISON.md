# Verdict: 1aug.md + SCALABILITY_V2_REVIEW vs my audit/scalability plan

Date basis: my docs @ `fd86c69`+patches (2026-07-25); theirs @ `deac95c`+ (2026-08-02).
(Note: "F-02" collides — *my* F-02 = IBD import binding; *their* F-02 = coinbase rho
collision. Below, F-02 means theirs where marked.)

## 1. Is 1aug.md good or shit?

**Good — genuinely strong engineering record, better than my scalability docs in the
dimension that matters most: everything load-bearing is measured on the live chain.**
Proving = 0.8 s/spend flat; live traffic = 0.9 actions/block; coinbase = 77% of tree
growth; dev fee = exactly 1.00 note/block = 32.8% of all note creation; `aux_pow` =
73.1% of every header; nullifier set = 0.78% of the DB; shielded stores = 6.6%;
snapshots = 1.63 KB per *chain* block ≈ 16.4 GB/yr. Its §12 rejected-ideas list and
§14 process lessons are at a level my docs didn't reach. Its main defect: the
headline root cause was found in production, not before launch — which is the one
thing my audit existed to prevent, and where I have to own a miss (§2.1).

## 2. The honest scorecard

### 2.1 Where I was wrong — the one that matters: SH-09 / the anchor landmines

My audit *saw* the identical-anchor overwrite (`anchor_block` last-write-wins, never
reverted) and dismissed it as "combinatorically near-impossible", reasoning that
coinbase notes derive from unique coinbase txids. **That uniqueness assumption is
false**: two siblings sharing a mergeset *and* a miner build byte-identical coinbase
transactions (same selected parent ⇒ same payload, same mergeset ⇒ same outputs) ⇒
identical txid ⇒ identical note seeds ⇒ identical tree root. Production proved it:
360 landmines, 2 live wedges, fresh nodes unable to sync at DAA 371,851 — exactly the
failure class my audit was commissioned to find.

Two aggravating facts I have to state plainly:
- My own F-04 fix (fail-closed anchor finality) is what converted the latent
  overwrite into a live chain-halting wedge. Fail-closed was still *correct*
  (fail-open was the inflation vector), but the fix was incomplete without the
  collision generator being killed — and I had the generator in my hands and rated it
  informational.
- Their reset plan had the same item ("F-02 coinbase rho collision") rated Low and
  out of the bundle. Both of us under-prioritized the *interaction*; only they can
  claim the excuse of not having done a dedicated audit.

Their repair is correct and well-layered: pins for history (with the crucial
bulk-pin trap documented), multi-producer index for determinism (dormant, gated,
120/120), and the F-02 seed fix (mix block hash into the note seed) to kill the
generator at a fork. §11.3's reasoning for *holding* F-02 out of upgrade 1
(mergesets are disjoint and contain the selected parent ⇒ distinct chain blocks can
never share a coinbase tx ⇒ multi-producer already closes the soundness gap) is
correct and subtle.

### 2.2 Where the review killed my proposals — fairly

| My item | Verdict | Why they're right |
|---|---|---|
| **N1 aux multi-commitment** | unsound | k blocks per 1 parent PoW = k× chain weight per unit hash — work counterfeiting. I analyzed slot scarcity and missed that blue work is additive. Their salvage (k-scaled target) is the sound version. |
| **N2 denominated minting** | inverted | 3,000 ZKAS from 57-ZKAS notes = 53 spends; split 50+7 → 60 spends. Downward denominations *increase* spend count for payouts > 1 note, and coinbase is 77% of tree growth — N2 worsens the dominant cost. Arithmetic, not opinion. |
| **K1 wallet cap 39→78** | wrong as written | Verified myself: `check_transaction_standard.rs:59,80` — shielded cap = min(compute, transient, storage) = 500k and *transient* mass is compared against it → 39+ actions is non-standard, never relayed. Their fix (per-dimension comparison) is the correct policy change. |
| **K2 parallel proving** | stale + wrong magnitude | Verified: `PROOF_THREADS_EACH = 2` shipped, measured **1.21×** (63.9 s → 52.9 s), not my 2–4×. The sublinearity was the headroom and it's harvested. |
| **N4 path RPC** | privacy blocker | Frontier snapshots can't serve paths (my V2 already conceded), the full-tree cost I quoted was 1000× high (73 MB live, ~4 GB/yr) — but `GetShieldedMerklePath(position, …)` tells the node *which leaf you're spending*. I waved that away as trust-neutral; on a privacy chain, for remote wallets, it's the property itself. SubtreeCache already took 82× of the pain. |
| **N5 epoch checkpoints** | redundant | My own F-02 import binding (shipped) already *is* "verify a checkpoint"; more frequent checkpoints are marginal. |
| storage figure | 2.7× high | snapshots are per *chain* block, not per block (mergeset ≈ 3). |
| T2.3 memo cut | rightly demoted | changing the action format fragments the anonymity set — a permanent cost for 14%. On a privacy chain that's a bad trade; I under-weighted it. |

### 2.3 Where I was right (confirmed by them)

- **K4 verify cache — "best idea in the set", still unbuilt.** Every miner re-verifies
  every pending shielded proof every second (`processor.rs:1948`).
- **N6 pre-verification in the body processor — adopt, unbuilt.**
- **T1.3 admission nullifier/anchor checks — adopt, unbuilt.** Under
  drop-not-disqualify this is a silent-failure bug, not a nicety.
- **E4 parent-rate cap — "genuinely sharp observation"**, now a recorded precondition
  for any BPS raise (with the k-scaled-target caveat).
- **E3 X1 privacy leak — "correct, well caught"**; X1 is dead on a privacy chain.
- **E1 one proof per bundle — correct.**
- The security findings themselves: max anchor age 27,000, fail-closed, the import
  binding, store clearing — all live in their tree and held up. The batch-verify
  per-tx-fallback caveat they stress (batch failure must not reject the block —
  drop-not-disqualify) was in my T1.2; they're right to stress it.
- A4/storage-mass: their §12 rejection of per-action storage mass matches my ECON-08
  (actions are already priced by size, and size tracks footprint; revisit only if
  proofs shrink).

### 2.4 What 1aug has that my plan never reached

- Dev-fee accrual (−32.8% of note creation) with three genuinely hard-won design
  decisions: separate store (bincode widening would brick every DB), payout on
  interval *crossing* (DAG blocks step over boundaries), and the accrual-aware
  turnstile invariant found **only by mining across the boundary** — 287 green unit
  tests while the chain halted at activation.
- The two-upgrade split keyed on *wallet-fleet coordination cost* — the right
  criterion, better than my tiering.
- Branch-id/sighash analysis (replay across a split is a deanonymisation vector, not
  an accounting bug) — a privacy-chain fork consideration absent from my docs.
- §12 rejections with reasons: Penumbra TCT (impossible cheaply), epoch anchors
  (superseded by age-pruning), general miner accrual (public payroll — rejected on
  *principle*), smaller anchor window (sharpens the spend-timing fingerprint — a
  privacy parameter I treated as purely operational), aux_pow header split (ZSTD
  captured most of it), proof stripping (an archival node must be able to prove what
  it serves).
- Process lessons §14 — every one of them earned.

## 3. The merged program (what actually remains)

**Non-fork, build now:**
1. **K4 txid-keyed verification cache** (their #1 too).
2. **N6 parallel pre-verification in the body processor**, virtual consumes cached
   verdicts; batch verification optional, per-tx fallback mandatory.
3. **T1.3 admission checks** (finalized nullifier + known-anchor) — silent-failure bug.
4. Relay-policy per-dimension mass comparison (their §4 salvage of K1) — coordinated
   rollout, not a constant.
5. Anchor age-pruning (open in theirs, ~1.6 GB/yr, safe by construction).
6. `v1.0.5` pins release — **blocking** (fresh nodes can't sync without it).

**Upgrade 1 (node/miner only, no wallet ships):** multi-producer anchors (retires the
pins) + dev-fee accrual. Both built, rehearsed, dormant; the only outstanding item is
the activation DAA and the §9.3 process (spec-first `ZKAS-NU1`, 5-week notice, p2p
gating, template-refusal guard for stale nodes, aggregate-only telemetry).

**Upgrade 2 (wallet fleet ships once):** F-02 coinbase seed + branch id in sighash.
Gate all five derivation sites identically first; acceptance = a from-zero IBD across
the boundary + a wallet scanning both sides with the same balance (§9.6 — the only
honest fork test).

**Preconditions recorded, not scheduled:** E4 before any BPS raise (with k-scaled
target if multi-commitment is ever revived); compact/Graphene relay before any
block-size raise (p50 body is 411 B — no present-day win); X2 block-level proof
aggregation = the research endgame (their PLAN Phase 2 agrees).

**Dead, do not re-propose:** N1 (unscaled), N2, K1-as-written, N4-as-RPC, N5, X1,
T2.3, general miner accrual, smaller anchor window, epoch anchors, Penumbra TCT.

## 4. Method verdict on myself

The review's §11 is aimed at me and it's mostly fair: "strong at enumerating the
design space, honest in self-correction, consistently wrong on constants." My audit
mandate was static-only (no build/run), which explains the measurement gaps — but
N1's work-counterfeiting and N2's arithmetic were *reasoning* errors, not measurement
gaps, and SH-09 was a wrong uniqueness assumption stated with confidence. The pattern
to adopt from them permanently: **no claim about a constant without running the
arithmetic to a number; no "X is dead/alive" without the benchmark; and any
uniqueness assumption about hashes gets an explicit collision construction attempt.**
Their chain is measurably better than my documents left it — and three of my four
surviving ideas (K4, N6, T1.3) are precisely the ones their measurements confirm.
