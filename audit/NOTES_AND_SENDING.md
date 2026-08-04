# Notes & Sending on the 1 BPS DAG — verified analysis and playbook

Every claim below is either measured (cited to `1aug.md` / in-tree benchmarks) or read
from code (file:line). Nothing carried over unverified from earlier docs.

## 0. Ground truth (verified)

| Fact | Value | Source |
|---|---|---|
| Proving cost | **0.8 s per note spent** (wall, 4-core EPYC); flat 4→38 spends | 1aug §7.1 benchmark |
| Block capacity | 79 actions/block validity; **38 standard** (relay) | `check_transaction_standard.rs:59,80` (transient ≤ 500k) |
| Live traffic | **0.9 actions/block** — capacity is not the problem | 1aug §13.2 |
| Pool pain anatomy | 47,159 treasury notes → 9,006 spends → 237 txs → ~2 h | 1aug §7.1 |
| Tree growth | **77% from coinbase**; dev fee alone = 32.8% of all notes | 1aug §13.2/§10.1 |
| Tree grows per **action** (incl. dummies), not per real note | `state.rs:303` | 1aug §13.2 |
| Send latency today | 3–5 s (SubtreeCache shipped, 82×) | 1aug §7.3 |
| Anchor maturity | 600 blocks ≈ 10 min to *respend*; 0.3% of send cost | 1aug §7.3 |
| Multi-payee batching | **EXISTS**: `ceil(N / max_payees_per_tx())` proofs | `zkas-walletd/lib.rs:4263`, `shielded-core/wallet.rs:598` |
| Prove/sign split (PCZT) | **EXISTS**: proving needs no spending key; `ask` signs after | `wallet.rs:755,896` ("DEVICE (ONLY `ask`; never proves)") |
| Consolidation + 1.21× parallel proving | shipped | 1aug §7.2, review §5 |

## 1. The pool payout problem, solved with what already exists

The 2-hour payout was **9,006 single-recipient proofs**. The fix is not protocol:

**1a. Batch payees — 37 payouts per proof, TODAY.**
`max_payees_per_tx()` = 37 (38 actions output-bound). A 1,000-miner payout run is
`ceil(1000/37) = 27 proofs ≈ 27 × 30 s ≈ 13 min` on a 4-core box — not 1,000 proofs.
Actions per tx = `max(spends, outputs)`, so with a consolidated treasury (1–2 big
notes) each batch tx is 1–2 spends + 37 outputs. **Action: verify the pool software
calls the multi-payee endpoint (`lib.rs:4263`); if it still sends one payee per tx,
this is the single highest-value change available anywhere in the system — ~37×.**

**1b. PCZT proving fleet — proving without keys, TODAY in shielded-core.**
The PCZT flow creates proofs without `ask` and signs afterward on a separate device
(`wallet.rs:755,896`). A pool can therefore fan proving out to N untrusted worker
boxes and sign locally — the 0.8 s/spend constant becomes horizontally scalable
without key exposure. **Action: check whether walletd exposes the PCZT path; if not,
wire it (wallet-side only, no consensus).** Honest limit: witnesses + note data still
leave the signing device (workers learn note values/positions) — fine for a pool's
own workers, not for public delegation.

**1c. Treasury hygiene (shipped):** auto-consolidate in idle time so batch txs need
1–2 spends; proving for consolidation happens in the background, not in the payout
path. More cores on the payout box scale near-linearly for a backlog (32 cores ≈
12 min for a 9k-spend backlog — but with 1a+1b, backlogs shouldn't form).

**1d. Reliability (unbuilt, consensus-adjacent):** T1.3 admission checks — a payout
tx that is mined-but-dropped (stale anchor / already-spent nullifier) costs the pool
30 s of proving and silently pays nobody. Still the top reliability item.

**1e. Shielded RBF (policy):** nullifier-keyed replacement with a feerate threshold
(machinery exists). A stuck payout currently locks its notes until expiry.

## 2. Note-count / tree-growth levers (protocol)

The problem is not spend count — it's that **the coinbase mints 3.05 notes/block
forever** (77% of tree growth). Levers, ranked:

**2a. Dev-fee accrual — built, rehearsed, dormant (−32.8% of note creation).**
Activate with Upgrade 1. Nothing else to design.

**2b. OPT-IN miner accrual — the structural fix for the treasury class (new idea, fork).**
Let a miner mark its payout as "accrue until interval I" (a flag in the payout
script/miner payload; consensus defers the note until the interval boundary, same
machinery as dev-fee accrual §10.2 — separate store, payout-on-crossing, byte-exact
coinbase validation keeps working). A pool receiving 1 note/block (~57 ZKAS) opts
into I=1,000 → one ~57,000 ZKAS note per 1,000 blocks: **treasury note count
collapses ~1000×, and every downstream payout needs ~1000× fewer spends.**
Privacy honesty: the pool's cadence is already visible (it's the dominant miner), and
opt-in means small miners keep per-block notes. This is NOT general miner accrual
(1aug §12 rejected a mandatory consensus payroll keyed by script — correctly); it's
an individual miner's choice about its own already-public payout. Design risk:
accrual-aware turnstile generalization (the §10.3 bug class) — needs the same
rehearsal standard. Worth doing after dev-fee accrual proves the pattern live.

**2c. Same-recipient coinbase folding** — 11% of coinbase notes, privacy-neutral,
rejected in 1aug §12 for breaking output-index↔mergeset attribution. Their call
stands; if it ever comes back, the new `GetShieldedCoinbaseRewards` RPC (which
already handles multi-reward-per-recipient) is the attribution path to migrate first.

**2d. Relay per-dimension mass fix** — 78-action txs become standard (from 38):
halves the tx count for large batches. Coordinated policy rollout (review §4).

## 3. What is NOT worth doing (verified dead ends)

- **Anchor-depth tuning** — 0.3% of send cost (1aug §7.3), and narrowing the window
  sharpens the spend-timing fingerprint (1aug §12). The 10-min respend latency is a
  BPS property, not a depth property; only a BPS raise changes it (out of scope here).
- **Capacity raises** — 0.9 actions/block live. Pure headroom work; revisit when
  traffic is >10× present.
- **Denominated minting** — inverted arithmetic (review §3).
- **Wallet-side tree-growth fixes** — impossible: growth is 77% coinbase and per
  *action* (dummies included); only coinbase-side levers (2a/2b) move it.
- **Mandatory miner accrual** — public payroll (1aug §12, correct).

## 4. Research track (honest uncertainty)

- **GPU MSM proving.** Proving is MSM-dominated; GPU halo2 backends exist in the
  ecosystem (e.g. ICICLE-class libraries), but orchard-0.14 integration is real
  engineering, not a drop-in. If it works: 5–10× on the 0.8 s/spend constant —
  the only thing that makes single large payments *fast*, not just parallel.
- **X2 block-level proof aggregation / Tachyon-style.** The endgame (PLAN Phase 2);
  required if BPS ever rises. Months of circuit work.

## 5. Ordered playbook

1. Pool → multi-payee endpoint (1a). **Do this first; it's ~37×.**
2. T1.3 admission checks (1d).
3. PCZT proving fleet (1b) if payout latency still matters after 1a.
4. Activate dev-fee accrual with Upgrade 1 (2a).
5. Relay per-dimension fix (2d) with the next coordinated node release.
6. Design opt-in miner accrual (2b) for Upgrade 2/3.
7. Shielded RBF (1e) when fee pressure exists.
