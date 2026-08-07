# SESSION STATE — 2026-08-07 (FINAL, session close)
### Merged-mining bridge v2 · zkas-rusty-kd · Operator: Michael (IndyDaredevil)
### Supersedes SESSION-STATE-2026-08-06.md (this file replaces it; prior version in git history)

---

## 0. ONE-PARAGRAPH STATE

The v2.1 release candidate runs the **full fleet** (7 rigs, ~14.9 TH/s, instances
:5755/:5765) mining KAS with merged zKAS live (`zk=ok`, 100% of templates
committed). 24h performance at session close: **29 KAS, 16 zKAS, 15 doubles,
30 solves**. WS1 and WS2 are complete; WS3 is substantially built; the spec is
current at **Draft 4** and now lives in the repo. Open threads at close: PR #3
unmerged, the zKAS network-hashrate figure is a stale hardcoded pin that makes
zKAS Luck read ~1.7× low, and WS4's detach path remains unbuilt (the one
knowingly-carried production risk).

---

## 1. REPOSITORY STATE

- `main` — carries PR #1 + PR #2 (NotificationHub/WS2, address-fixture fix,
  clippy fixes, Windows test unblocking).
- `merged-ws1-port` @ **a3de8f5** — PR #3 OPEN, non-draft, `mergeable_state:
  unstable` (= the required `bridge` check is GREEN; "unstable" only reflects
  the inherited upstream noise floor on non-required checks).
- Branch protection on `main`: PR required; required status-check context is
  the **`bridge` JOB** inside `bridge-check.yaml` (a context is a job name, not
  a workflow name — this cost two wrong configurations to learn).
- **Fork is 15 commits behind `firecash/zkas-rusty:main`.** Evaluated this
  session — see §5. Do NOT use the GitHub "Sync fork" button: it writes
  straight to `main` and bypasses the PR gate, and its red **"Discard 17
  commits"** neighbour would erase all merged work.

### Documents now versioned in-repo at `docs/bridge-v2/`
- `merged-bridge-v2-spec.md` — **Draft 4** (see §2).
- `SESSION-STATE-<date>.md` — this file; prior ones archived as `archive-*`.
- `README.md` — reading order + conventions (one current state doc; spec =
  contract, state doc = position; drafts numbered, never silently renumbered).
- `rc-merged-example.yaml` — sanitized RC config with port/listener rationale.
- Repo root: `start-rc-bridge.cmd` (launcher; bakes env + `--node-mode
  external` + config path). Michael's equivalent `run-rc-merged.cmd` is
  untracked — **dedupe these two.**

---

## 2. SPEC — DRAFT 4 (2026-08-06, commit a3de8f5)

Draft 3 predated the build and had become **wrong**, not merely stale. Draft 4
corrections, all consequential:

- **Invariant 3 corrected.** Draft 3: "block gate = merged_fc_target, never
  parent bits" — true only in the zKAS-primary reference. Now: the gate is the
  **per-job union** of both targets; a plain job gates on parent bits and is
  byte-identically RKStratum. Implemented literally, the old text would have
  broken the KAS leg.
- **Invariant 4 corrected.** Draft 3: "parent submit precedes zKAS claim." As
  built, the zKAS arm spawns first and the two are independent. Restated as the
  real property: KAS submission is never gated by, ordered after, or
  conditioned on the claim's outcome.
- **Invariant 8 added:** accounting law `solves = kas + zkas − doubles`; a
  double is ONE solve with TWO chain-hashes (parent hash on KAS, `H_fc` on
  zKAS). No counter, table, export or webhook may show a double as two solves
  or with fewer than both hashes.
- **"Structural immunity" claim RETRACTED** — in-workspace builds are immune to
  the *lockfile-drift* class only; the cdylib/risc0 link class lives inside the
  workspace and needed three separate fixes (§4).
- **Promoted from commit messages into the spec:** attach lifecycle (§3.1),
  commitment wire format and *why* it is ASCII hex (§3.2), the non-blocking
  template budget and the once-per-template stash rule (§3.3), the WS3-DA
  display semantics, the WS3 telemetry contract (status line is the production
  instrument; debug is bring-up only), WS5 config facts, §7 launcher
  requirements.
- **Status marks** per workstream; **WS4 re-scoped** to what it still owes;
  **§8 records the cutover deviation** (full fleet moved to the RC ahead of WS4
  and any soak) as an accepted risk with its residual named; §10 re-priced
  against actuals.

---

## 3. LIVE RESULTS AND THE OPEN MEASUREMENT PROBLEM

**24h card at close:** ZKAS 16 · KAS 29 · Doubles 15 · Solves 30 · Rig 14.86
TH/s · KAS Net 311.13 PH/s · KAS Luck 74% · zKAS Net 18.06 PH/s · zKAS Luck 24%.
Accounting law verified in the card: 29 + 16 − 15 = 30. ✓

**The zKAS network figure is WRONG and it matters.** The monitoring pins zKAS
Net at **18.06 PH/s** (a hardcoded fallback, "re-pin monthly"); the operator
reports the real figure runs **~30–32 PH/s**. Consequences:

- zKAS expected blocks/day at 14.86 TH/s: **41.4** (at 31 PH/s), not 71.1.
- **zKAS Luck ≈ 39%, not the 24% displayed** — understated by ~1.7×.
- **The two chains' per-block difficulties are nearly EQUAL.** Calibrating from
  the KAS leg (`H ≈ difficulty × BPS × 2` → 1.56e16 × 10 × 2 = 312 PH/s, matching
  the observed 311), zKAS at 31 PH/s and 1 BPS implies **zKAS difficulty ≈
  1.55e16 — within ~1% of Kaspa's.** Different network hashrate, different block
  rate, same per-block target.
- **This retires the "1.5–1.8× difficulty ratio" figure carried in earlier docs**
  and explains the observed full-clear rate: at ratio ≈1.0, nearly every zKAS
  block is also a KAS block. 15 doubles against 16 zKAS (~94%) is exactly what
  theory predicts — NOT an anomaly.
- **A flagged "missing zKAS-only singles" anomaly is hereby RETRACTED.** It was
  an artifact of the bad denominator: at the pinned 18.06 PH/s the model
  predicted ~11 zKAS-only singles that should not exist at the true ratio. The
  dual gate looks healthy.
- Yield projections built on the old ratio need redoing: zKAS is roughly a
  *1:1* additional block stream with KAS, not 1.5–1.8×.

**Fix direction (next session):** derive, don't pin. The bridge already holds a
gRPC client to the zkas node; `get_block_dag_info().difficulty` plus the
calibration above yields a live gauge, killing both the monthly re-pin chore and
the silent-staleness failure mode. Interim: bump the pin to ~31 PH/s and date it.

**Monitoring files NOT YET REVIEWED** — `alert_rules.yml` and `prometheus.yml`
live only on the Kron and were unavailable this session. Two things to check
when they surface: (a) whether the `×10` factor applied to the zKAS gauge (per
prior notes; KAS is native with no ×10) is a legitimate unit correction or a
copied Kaspa-BPS assumption — a spurious ×10 would read zKAS luck 10× low;
(b) that the "24hr" card's denominator handles a fleet whose hashrate changed
mid-window. **Commit these files to `docs/bridge-v2/monitoring/`.**

---

## 4. BUILD / RUNTIME FACTS (hard-won; do not re-derive)

- **Kaspa node gRPC = :16110.** `:17110` is wRPC-Borsh (what legacy RKStratum
  used — the remembered "efficiency": Borsh over protobuf, hence its
  `tcp_no_delay` pairing). Symptom key: wrong-protocol port = accept-then-abort
  (BrokenPipe/ConnectionAborted); dead port = ConnectionRefused.
- `tcp_no_delay` is **not** in the v2 schema and is **silently ignored** if
  present (no `deny_unknown_fields`); tonic enables TCP_NODELAY by default.
- **`--node-mode external` is required** on Windows (default `Inprocess` hits
  the stub and exits). Baked into the launcher.
- **Merged env vars are per-PowerShell-window** and were lost three times
  across restarts — each loss silently produced a KAS-only window and cost zKAS
  earnings. Restarting by anything other than the launcher is now a spec
  violation; the two `MERGED MINING ENABLED` banner lines are the check.
- **Bug #1 (Windows risc0/cdylib link) has THREE vectors, all closed:**
  (a) `kaspad → wrpc-server` cdylib → dependency gated `cfg(not(windows))`,
  in-process-node stubbed, node-embedding tests gated;
  (b) `consensus-core → shielded-core → risc0` (ungateable) → `cfg(windows)`
  `no_mangle` host stub for `sys_alloc_aligned`;
  (c) **example binaries** don't auto-link the parent lib → `use
  kaspa_stratum_bridge as _;`. **Rule: every new artifact kind (example, bench,
  bin) gets a Windows test build.**
- Release builds always survived via LTO dead-strip — why `-win` tags built
  while `cargo test` on Windows never could.
- Rebrand-sed casualties documented: **four** (FCMM lockfile pin, legacy address
  fixtures, the `NetworkIdError` message, the network-name parser). This is the
  fork's signature bug class.

---

## 5. UPSTREAM SYNC EVALUATION (fork is 15 behind)

Reviewed this session; **decision: do not sync yet.**

- **Eleven commits are walletd performance work** (witness climb off the send
  path, adaptive page sizing, idle-wallet eviction, subtree-cache semaphore,
  scan-CPU profiling). Aimed at real pain (walletd scan lag ↔ treasury
  confirmation ambiguity) but land in `zkas-walletd`, and the operator runs the
  **official v1.0.5 binary**, not a fork build. No benefit until that changes.
- **`9a7dd342` "zkas-api: merged-mining attribution view"** — upstream's
  per-IP Kaspa payout attribution joined from an off-node ZKMM indexer, plus a
  `zkmm_indexer` example. **Read for design intel before upstreaming any of our
  attribution work**: theirs is pool-shaped (per-IP, off-node), ours is
  solo-shaped (worker-name labels, in-bridge).
- `e49ce610` shielded-history p2p backfill (archival note completeness);
  `2e7b3ddb` marks the anchor pin file obsolete (matches what we knew).
- **Why waiting is right:** `bridge/` is untouched by all 15, but
  `consensus/core/src/api/mod.rs` (+51), `consensus/core/src/lib.rs`,
  `zkas_state_binding.rs` and `consensus/pow/src/auxpow.rs` all changed, and
  `475aeb4c` explicitly carries shielded+covenant API changes through call
  sites. Syncing means rebuilding the RC against a shifted consensus tree while
  the fleet mines on it — the "post-NU1 consensus drift" risk in the spec's own
  risk table — for zero current benefit.
- **Order when we do sync:** (1) merge PR #3 first so its base stays stable;
  (2) sync on a dedicated branch (`sync-upstream-<date>`), never the fork-sync
  button; (3) `cargo test -p kaspa-stratum-bridge`; (4) only then fast-forward
  `main`, rebuild, restart via launcher.

---

## 6. OPEN ITEMS (priority order)

1. **WS4(a) detach-on-loss** — the one knowingly-carried production risk: a
   zKAS node death mid-session leaves the leg attached, every template quietly
   misses its budget, and the bridge earns KAS normally while silently earning
   no zKAS. Only the `zk=` status field reveals it.
2. **zKAS network hashrate: derive from the node**, retire the pin (§3). Then
   recompute the yield model on the ≈1.0 difficulty ratio.
3. **Review `alert_rules.yml` + `prometheus.yml`** (the `×10` question, the
   24h-window denominator) and commit them to `docs/bridge-v2/monitoring/`.
4. **Merge PR #3**; then `deploy.yaml` needs `--bin stratum-bridge` + zip
   packaging (web-UI edit — the PAT deliberately lacks Workflows scope); then
   tag **v2.1.0-rc1-win**.
5. **Near-miss ratio anomaly** — console vs dashboard disagreed (5.41e-1% vs
   1.22e+1% same session) and ~19% readings are implausible under c.12's own
   magnitude math. c.14 forensics are armed. **V3 is not claimable until the
   instrument agrees with itself.**
6. **Status-line `blk=` anomaly** — 1,103,374 at 05:10 vs 1,479,661 at 03:36 on
   the same synced node. Display/units artifact; mining unaffected.
7. **Token** expires 08-12; revoke + remint (the Read-only dropdown trap caught
   both prior mints — the confirmation modal must read "Read and write").
8. **Upstream PRs**: `cb632f7` (legacy address fixtures) and `d0232e1` (kaspa-
   network-name prefix) — self-contained, and both fix upstream's own CI.
9. **Punch list:** prom balance fetch gate-off (mining wallet not utxo-synced
   by design); flip the `Inprocess` default or document it; restore `-D
   warnings` on the bridge-check clippy step (the crate is clippy-clean now);
   server-side `BlockInfo` optional `kas_hash`/`zkas_hash`; delete the merged
   `ws2-notification-hub` branch; dedupe the two launcher .cmd files.
10. **Gates remaining:** V3 (blocked, item 5), V4, V5-on-our-bridge (needs a
    live dual settled by our code — the RC's KAS blocks were KAS-solo, and the
    15 doubles now on the card are the first evidence to verify against the
    treasury), V6 (blocked on WS4), V7 (≥48h A/B).

---

## 7. PROCESS RULES (binding)

- **Local-first:** every command destined for CI runs on the Windows machine
  first (cargo 1.95 — the only machine that can compile). Never hand-patch
  `Cargo.lock`.
- **Build after every merge/rebase, before pushing.** Two build breaks this
  arc came from trees no compiler had seen.
- **Conflict resolution means EDITING THE FILE** — deleting the
  `<<<<<<< / ======= / >>>>>>>` markers and choosing content. `git add` during a
  rebase stages whatever is there, markers included. After any rebase:
  `grep "^<<<<<<<"` before building.
- GitHub required-check contexts are JOB names; workflow files go through the
  web UI.
- **Verify beats infer.** Ports, wire formats, rebuild-time heuristics, network
  hashrate — every inference that lost this arc lost to a one-command check.
- Verbatim-ported files stay verbatim; new assertions live in `tests/` beside
  them.
- **Debug logging is bring-up only**; state-change INFO plus the status line is
  the production instrument. If a gate can't be expressed in the telemetry, the
  gap is in the telemetry.
- **Test contracts before assertions.** Two tests this arc encoded wrong
  contracts (hub `Closed` semantics; raw-bytes commitment) and both cost a CI
  round.
- **A wrong denominator invents bugs.** The zKAS "missing singles" anomaly was
  chased on a stale pinned constant. Validate the inputs of a model before
  investigating its residuals.

---

## 8. INVENTORY

- **Host:** ACEMAGICIAN Kron K1 (Ryzen 7 5825U, 32GB, 1TB NVMe).
- **Fleet:** w1,w2,w5,w6 = KS0 Ultra; w7,w8,w9 = KS7 Lite. ~14.2–14.9 TH/s
  total. Merged-side worker suffix "m".
- **Nodes:** kaspad v2.0.1 (gRPC 16110, wRPC 17110, p2p 16111); zkas v1.0.5
  (gRPC 16810, p2p 16811) + walletd :8501.
- **Payouts:** KAS → per-worker `kaspa:qznq82nszz…` via stratum authorize
  (load-bearing: it *is* the coinbase address). ZKAS → single treasury
  `zkas:px7ggt9l6kh45k2nffc63mpclvz92mln4z6cvt2dcnewxa7c8950dgtl8lhyk3nqvdqyw8qc5r3fxrn`
  (the rig username is an identity label only on this chain).
- **Ports:** 5555–5563 RKStratum (failover), 5655–5663 reference merged bridge,
  **5755/5765 the RC**, dashboards 3030/3033/3034, prom 2114–2122/2214–2222.
- **Close state:** RC running the full fleet with merged live; RKStratum
  available as each rig's pool #2.
