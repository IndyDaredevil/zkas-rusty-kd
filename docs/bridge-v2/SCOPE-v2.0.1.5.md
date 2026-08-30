# SCOPE — Bridge v2.0.1.5 + Pipeline Stream P + Host Stream H
### Content r5 · revised 2026-08-30 · Status: SHIPPED — v2.0.1.5 IN PRODUCTION
### Code tip merged-v2.0.1.5 @ 1b63698 (running exe); docs tip @ ac4a603.
### Stream A is CLOSED except A1′'s seven-day acceptance (D2, 09-04).
### Revision chain: r1 (2026-08-22) → r2 (folded 08-27) → r3 (08-27) → r4.
### r3 was committed at 6ecfb06 on merged-v2.0.1.4 and was reachable ONLY from
### that retired branch for two days while the production branch carried r2
### ("READS PENDING") — reconciled onto merged-v2.0.1.5 at a2d8650, incident
### ledgered BL-056. r4 supersedes r3 in place per the naming law: same
### filename, header carries the content revision.
### r4 records the 08-28 fleet deploy, the v1.0.6 node cutover, and the 08-29
### alerting/walletd work; it converts every «acceptance pending» marker into
### either a closed item or a dated calendar gate.
### r5 records S14 (08-29→30): P1 SHIPPED end to end (ingest + Kron sampler,
### all five gates verified against deployed artifacts), PowerPanel installed
### and EXERCISED, and adds P6 + P7. Ledgered BL-057..BL-062.
### Convention: Stream A = bridge release (one exe). Stream P = reporter/schema
### (ships independently, WS7 discipline). Stream H = host work.
### References: ENGINEERING-LEDGER sealed @ BL-062 (1425 ln,
### d4c3807e…348e17, commit 4734675). FLEET-DEPLOY-v2.0.1.5-r1 (5df03ad7…a4e4e)
### is the executed deploy record. NODE-CONTRACT-v1.0.6 (ce39e49d…2692) and
### NODE-CUTOVER-v1.0.6-r1 (78da1589…a36e) govern the node leg.
### SESSION-STATE-2026-08-27 (a7bdd148…4b02d) is the deploy-eve handoff.

---

## 0. STANDING RULES FOR THIS RELEASE

- **Aggregate-only telemetry.** No per-share POSTs anywhere in this scope.
  Reporter ships rollups; the webhook's idempotency key is not a license to
  firehose. Fine-grained collection requires a demonstrated need first.
- **Pipeline is the toolchain** (BL-021): anchored-replacement PowerShell
  patches → `merged-v2.0.1.5` branch → bridge-check CI as compile gate →
  branch-targeted release → win64 zip → canary `:5775` (w1c) → production.
  BL-030's banner guard is in `deploy.yaml`. `BRIDGE_BUILD` = 5, shipped.
- **Promote soaked bytes, never fresh CI output** (FLEET-DEPLOY r1 policy,
  executed 08-28): the artifact that soaked on the canary is the artifact
  that goes to production. A rebuild at deploy time discards the soak.
- **Investigation before code, per item.** Every A-item began with a source or
  query read that could shrink or delete the code change. Outcome, measured:
  the reads deleted more code than they authorized — A2/A3/A4 closed at zero
  code, A5 migrated to Stream P, A6 and A1-hygiene deferred, and the release
  shipped as A1′+A8 (two items) against a seven-item ceiling.
- **Production is sacred** (meta-principle 8): every bridge change canaries on
  `:5775` before fleet exposure. Rollback = relaunch prior exe via
  `run-rc-merged.cmd` (parked copy retained per BL-031 retention policy).
- **Single-process reality** (BL-039): both stratum "instances" are listeners
  in ONE OS process (`stratum-bridge`, one PID owning :5755/:5765/:3034) —
  one kill target in every deploy sequence, one crash domain across both
  fleets. Confirmed at deploy: one PID (14940) owning all three listeners.
- **Branch retirement is a step, not a state** (BL-056, new at r4): when a
  release branch becomes canonical, the predecessor is frozen read-only and
  verified to hold nothing the successor lacks. `merged-v2.0.1.4` was retired
  at a2d8650 after silently taking three documentation commits post-deploy.

---

## 1. INVESTIGATION RECORD (what rewrote r1 — ledgered BL-032..BL-044)

1. **A0 gate CLOSED, early, worse than hypothesized** (BL-033): 14s scrape
   ceiling at ≤37h uptime, 25s ceiling within hours of raising it; 18 scrape
   failures over 2 days. r1's A0 item is retired; its outcome is recorded
   here and in BL-033.
2. **BL-029's causal claim REFUTED** (BL-033, pointer at BL-029): the metrics
   page is ~530 samples serving in ~230ms (direct probe). Series growth is
   real (~6/hr smooth schedule-minted ramp) but cannot drive a 25s render.
   Series retirement demoted from fix to hygiene (A1-hygiene below).
3. **The stall class:** episodic — floor ~230ms, spikes PIN at whatever the
   timeout is (12→14→25s observed) = blocked, not busy. True stall length
   never measured (outlived every ceiling). Resolved by A1′; see §2.
4. **Morning stalls SOLVED** (BL-035): Store retry-grind on ScreenSketch;
   killed 08-22; zero morning dips in 4 days — prediction confirmed.
5. **Night dips: composite reading WEAKENED by experiment** (BL-036). H1
   summon (08-27): three event-log-verified RDP churn cycles over a live 2s
   probe — floor unbroken; churn REFUTED as sufficient trigger. Defender
   exonerated for the 01:31/03:33 08-26 pair (31 min offset; nightly
   metronome cannot produce episodic effects). Those two dips remain
   UNATTRIBUTED — but A1′'s root cause (a parked accept loop) is a
   sufficient mechanism for both, and D2's seven-day window is the test.
6. **The Stream-A headline — ANSWERED** (BL-045): host pressure was never
   the mechanism. See A1′.
7. **Host context now instrumented-adjacent** (BL-040/BL-044/BL-054): the
   uncommanded power series is now EIGHT events, closed as MIXED etiology —
   8/15–16 convicted Kron-local (19V brick/DC-barrel class; rigs rode
   through), 8/27 outage premises-wide. UPS ×3 installed 08-27. **BL-044's
   "every future event is a one-bit diagnosis" claim did not survive first
   contact:** event eight (08-28 09:44) was UNREADABLE — Event ID 105 is not
   logged on this host and no CyberPower/PowerPanel service is installed, so
   nothing records battery transfers. Kron-local by signature, unproven by
   instrument. Installing PowerPanel is an H2 rider. Measured recovery cost
   with the operator asleep: ~34 minutes, gated on logon — the realistic
   figure for H2 service migration.

## 2. STREAM A — BRIDGE v2.0.1.5 · SHIPPED 2026-08-28

### A1′ · HEADLINE — ROOT CAUSE FOUND, FIXED, DEPLOYED (BL-045, BL-050)
**Read outcome:** `serve_http_loop` was serial, spawn-less and timeout-less;
any idle/half-open client (canonically the operator's own dashboard browser)
parked the whole server at read(). True render floor 5ms. Fix: spawn-per-
connection + 5s read timeout, ~10 lines.
**Deterministic acceptance PASSED on canary 1b63698:** silent-parker stimulus
that pins v2.0.1.4 at a measured 8,048ms yields 213–232ms × 8 on v2.0.1.5,
parker evicted by FIN at 5s.
**Production proof 08-28:** parker re-run against `:3034` post-deploy — 8×
flat 214–266ms then server FIN. The interim "no browser tabs on :3034" rule
is LIFTED.
**Remaining acceptance — D2, the only open Stream-A item:**
`max_over_time(scrape_duration_seconds[7d]) < 5s` evaluated on/after
**2026-09-04 ~01:09 EDT**. Pass → BL-050 close-out and this document's
retirement to historical scope. Fail → reopen A1′ with the seven-day series
as the read.

### A1-hygiene · Series lifecycle — MECHANISM SOLVED, work DEFERRED to
### v2.0.1.6
**Read outcome:** the ~6/hr smooth mint is the block-event gauges
(set_block_event_gauge: nonce/bluescore/timestamp/hash as label values —
one immortal series per block event across K/Z/D gauges, ~5.4/hr at era
block rates); the ghost twin is the pre-app-parse miner="" mint — and the
ghosts are LOAD-BEARING (block-accept counters record onto them). Retention
must be bounded-keep-last-N per gauge (the event gauges are the dashboard's
lossless block-history store), never blanket retirement. With the true
floor at 5ms this is scrape-size hygiene only — deferred.

### A2 · `full_clear` semantics — CLOSED by read, outcome (c): zero code
**Read outcome:** full_clear prints meets_network_target (pow ≤ KASPA
network target) on a line that only prints inside clears_zkas — it
literally means "double". The ~91% rate is physics: P(double|zKAS block) =
d_z/d_k ≈ 87% at current regime, matching both the 566/646 log rate and the
six-day table (284/348). The "~25 real doubles" baseline was an
incompatible early-era cohort. Field correct, correctly named.

### A3 · Hashrate estimator — CLOSED by read: documented, zero code
**Read outcome:** both legs call the node's
estimate_network_hashes_per_second RPC with a 1000-block observed window
anchored at tip; the bridge computes nothing. Ratio wobble is 1000-block
luck (σ≈3.2%); observations 2.11 (+5.5%) and 1.956 (−2.2%) both inside 2σ.
BL-028's minor-open tail closes.

### A4 · K/Z/D "24h" card — CLOSED: rules faithful; EXITS Stream A
**Discriminator run 08-27:** rule ≡ raw exactly on all three pairs
(32/35/27; solves 40), and raw reconciles with log-truth within
reset-compensation noise (±1 across three restarts). Not the stats loop, not
rule evaluation.
**Residual, still open:** one eyeball of the HourlyMergedReport card vs live
`rc:` values. Config-side if it disagrees — and note that the 08-29 finding
(BL-051) proves the Alertmanager leg had a real, unrelated defect, so this
eyeball is no longer a formality.

### A5 · Submit latency + time-to-blue — MIGRATED to Stream P
**Read outcome:** submit processing latency already exists (merged_obs #4,
µs, surfaced as sub=avg/max), and FOUND → ACCEPTED → confirmed-BLUE are
already timestamped info lines — the reporter can compute the deltas from
lines it already tails. Only submit-SENT lives at debug. Collect
reporter-side first; a structured bridge line only if per-block submit-send
granularity proves necessary. Now P4.

### A6 · Stale-share / job latency / disconnect counter — SHRUNK to job
### latency only; DEFERRED to v2.0.1.6
**Read outcome:** ks_invalid_share_counter (per-worker, reason-labeled) and
ks_worker_disconnect_counter both already exist — the W9 watch item is
queryable today. Genuine remainder: job-delivery latency gauge; its
dispatch site is not a single obvious location (notification_hub → mining
state → per-client sends), so it rides the next release.
**Added (BL-044):** per-worker disconnect/reconnect counter — the W9
spontaneous self-reboot (~8/25) was invisible to all current alerting.

### A8 · Balance-WARN circuit breaker — DEPLOYED
Site: client_handler.rs periodic fetch. Breaker: 10 consecutive failures →
one INFO, scheduling stops for process lifetime; any success resets. The
dead WARN was ~120–130/h of the entire WARN channel (501/4.2h, 557/4.3h
measured 08-27) — the breaker restores WARN visibility wholesale. Single
INFO confirmed on canary and in production.

### A7 · Release mechanics + close-out — EXECUTED 2026-08-28
- Deployed artifact: `stratum-bridge-1b63698.exe`, SHA256
  F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0 —
  the soaked canary bytes, promoted, not rebuilt.
- Five post-deploy gates PASSED: banner `v2.0.1.5 (engine 2.0.1)`; both
  MERGED MINING ENABLED lines; three listeners on one PID (14940); reporter
  log rotation within one second of launch; parker proof on `:3034`.
- Fleet 7/7 including w1m returned from canary duty (284/0/0, `zk=ok`).
- Two deviations, both recovered: a file lock on step 3a (resolved by
  reordering 3b ahead of the stage copy, per law 13c) and a cmd `&&`
  separator shipped into PowerShell 5.1 (resolved via `Start-Process cmd`).
  Both are FLEET-DEPLOY r2 candidates.
- **D1 — `.bak-v2014` retirement:** eligible on/after 2026-08-29 ~01:09 EDT
  per BL-031's one-clean-day policy. Run with `-WhatIf` preview first.
  STILL OPEN as of this revision.
- **D2 — seven-day A1′ acceptance:** see A1′ above. 2026-09-04 ~01:09 EDT.
- Docs pass: BRIDGE-SPEC §2 single-process clarification NOT YET DONE —
  carried to the next doc window.

## 3. STREAM P — PIPELINE (P1 SHIPPED 2026-08-29; P2/P3/P4 not started)

**P1 network_history — SHIPPED 2026-08-29 (BL-057, BL-059).** The curve is
no longer being lost. Brief P1-BOLT-BRIEF-r1.md (86d2b546…a210) was cut 08-27,
never landed, and was RECOVERED byte-identical from the conversation rail on
08-29 after a wrongly-issued VOID (BL-057). Supabase side: table
`network_history` (9 cols, UNIQUE on `sample_bucket`, RLS on, SELECT-only for
anon/authenticated) + edge function `network-history-webhook`; all four
acceptance tests plus the fail-closed check verified against the DEPLOYED
endpoint. Kron side: `set-nh-secret-r1.ps1` (2CCEFCFB…F0207) writes
`C:\zkas\nh-secret.dpapi`; `network-history-sampler-r1.ps1` (E4869402…30DA68)
is ONE-SHOT, cadence from scheduled task `NetworkHistorySampler` (5-min
repetition, both battery flags disarmed per BL-046). Verified firing
LastTaskResult 0, NumberOfMissedRuns 0.
Follow-ons NOT built: (i) Prometheus staleness alert on the sampler — it
currently fails to a log line nobody reads, the BL-032 detection-vs-escalation
gap; (ii) the dashboard chart consuming the table (explicit brief non-goal).
P3 worker_events → P2 worker_stats (metric-name verification first — from
`/metrics`, not memory) → P4 latency consumer (A5's migrated work).
- **NEW at r4 — P5 commitment enrichment.** zkas-node v1.0.6 adds
  `RpcShieldedCoinbaseOutput.commitment` (gRPC field 3, optional 32B): the
  CONSENSUS-COMPUTED note commitment, requiring no client-side derivation.
  Candidate column for `zkas_blocks`. Also new: request flag `metadataOnly`
  for cursor discovery. Both additive and wire-compatible.
  Gate: the running node must be confirmed post-`e49ce61` layout before the
  field is trusted (NODE-CONTRACT §3.2).
- **NEW at r5 — P6 treasury balance push (Kron → dashboard rail).** One-shot
  PowerShell on the P1 sampler pattern: read `/api/wallet/balance` from walletd
  on loopback, POST sompi-exact values to a new edge function, low cadence.
  Nothing new is exposed — walletd stays loopback-bound and firewall-dropped
  (VERIFIED 08-29 from the MacBook: exit=28, http=000, no connection
  established). Retires the treasury `file:///` page as a ROUTINE surface,
  which is the structural version of the behavioural rule "no browser left
  open on Kron" (BL-053). Honest limit: BL-053 records that killing the parked
  browser did NOT clear the 9.5h wedge, so P6 closes an exposure, not a proven
  root cause.
  **SEQUENCING: build AFTER H8**, not before — v1.0.6 changes this endpoint
  (`notes` needs `?notes=1`, `note_count` appears), so building against v1.0.5
  means touching it again immediately.
  **Design risk, must be handled:** walletd returns a ZEROED OBJECT rather than
  401 on bad auth, so the consumer must distinguish "answered zero" from
  "did not answer" or it writes false zeros into a permanent rail — the
  BL-052 class, applied to a number the operator might act on.
- **NEW at r5 — P7 chain-block rate analysis.** Operator hypothesis: the ratio
  of chain blocks to blue blocks rose after the bridge moved off template
  polling. Mechanism is sound and specific: GHOSTDAG selects the parent with
  highest accumulated blue work, so a fresher template means parents closer to
  the current tips, which wins selected-parent comparisons. In Kaspa the chain
  block earns the base subsidy PLUS all mergeset transaction fees; the other
  merge-set miners get subsidy only. Since subsidy decays on a fixed schedule
  and fees do not, chain-block capture is the long-run incentive.
  Data EXISTS (operator-built, Supabase + on-use expansion): 3,127 rows,
  734 chain / 2,393 blue = 23.5%.
  Open before analysis: (a) the polling→notification commit date pinned from
  git history, not recollection; (b) whether chain status is PERSISTED or
  computed on demand — if the latter, historical status must be bulk-resolved
  and stored before it ages out; (c) mergeset size as a covariate (chain-block
  probability depends on merge-set width, which moved independently of us);
  (d) two-proportion test — at 23.5% base rate a few-point shift is
  detectable, a one-point shift is not.
  Note the dataset is structurally BLIND to blue-vs-red: only paid blocks
  appear. If the same change also reduced reds, that gain is invisible here.
  **Rider on P5:** persist chain/blue status and merge-set size at block time,
  so the next bridge change is measurable prospectively rather than
  archaeologically.
- Bolt briefs: 08-21 constraints-first format + never-reinit-git-history
  addendum; Bolt cannot set edge secrets (operator does); no RLS loosening.
- Reporter updates per banked runbook: stop task → overwrite → start →
  verify RUNNING artifact (reporter.log listener line + `:9151/metrics`).
- Every consumer honors invariant 8 (accounting law: solves = kas + zkas −
  doubles), codified BRIDGE-SPEC §7.
- Enrichment citations: BRIDGE-SPEC §6/§9 (kaspa-parent join window),
  NODE-CONTRACT §3 (aux_pow stripped from RPC).
- First-live-day reconciliation per item (row counts vs source-of-truth
  query), sompi-exact.

## 4. STREAM H — HOST

- **H1 summon experiment: DONE 08-27, negative** (BL-036).
- **H1b posture: DONE 08-27** (BL-041/BL-039).
- **H2 maintenance window — PARTIALLY DONE, now the largest open item.**
  UPS ×3 installed 08-27. Remainder, one window: KRON-HARDENING §2–§6
  application + Store/OS update flush + **service/scheduled-task migration
  of the Session-1 processes** + deliberate bridge relaunch. Riders:
  (a) spare 19V brick/DC barrel (BL-040/044 convicted class);
  (b) **PowerPanel install** — without it the UPS is not a diagnostic
  instrument (BL-054);
  (c) **walletd is a migration prerequisite, now satisfied** — it has a
  versioned launcher as of 08-29 (BL-055) but still does NOT auto-start;
  (d) the launcher scripts are load-bearing config (they bake env vars), so
  migration must carry them, not replace them;
  (e) **UPS load rebalance to BL-048's designed split** — 2 × (KS7 + 2×KS0)
  ≈ 760W/76% on the 1500VA units, the 1000VA carrying Kron + switch + aux
  only, third KS7 surge-only. Designed 08-27, never written into a plan
  document until r5. Measured 08-30: 11 min runtime with 2 KS0s still on the
  1000VA (battery at 97%, understated); post-rebalance forecast ~33–40 min,
  the LOW half of BL-048's 30–60 band. Graceful-shutdown threshold must be set
  AFTER the rebalance and re-measured, never before (BL-058).
  Measured stake: ~34 min recovery when the operator is asleep vs ~5 min
  awake. Post-window: KRON-HARDENING §8 gates.
- **H3** Hardening acceptance: KRON-HARDENING §9, 7 quiet days from
  application.
- **H4** Clock discipline: **CLOSED HEALTHY 08-27** (BL-046).
- **H5** Revisit global-vs-per-job scrape config. **Escalated at r4:** the
  60s global interval is no longer merely a deviation — it is the proximate
  cause of BL-051 (block-card loss). `keep_firing_for: 2m` mitigates the
  symptom; the per-job design remains the fix. Gated on D2.
- **H6** BL-032 instrumentation & escalation package: windows_exporter +
  RAM% / commit-ratio / per-process-RSS / exporter-down rules (runbook
  drafted 08-26) + page-tier `ZkasLegDegraded` on sustained template age.
  **Still open, and now the only uninstrumented hypothesis** for the
  original wedge — the v1.0.6 upgrade removed the node-side blindness but
  memory pressure remains unmeasured.
- **H7 — CLOSED 2026-08-28.** zkas-node v1.0.6 deployed with file logging
  on; gRPC 16810 firewall-scoped to the MacBook IP (192.168.1.173), closing
  the 08-24 LAN opening that left unauthenticated RPC reachable by seven
  unaudited-firmware rigs. Cutover cost: ~14 min zKAS-leg downtime, zero
  bridge restarts, 27,495/0/0 shares, 32.8 GB datadir cold copy in 7:49.
  Residual: the firecash wedge report is drafted but NOT SENT — it batches
  with the other upstream findings (see §7).
- **H8 — NEW: walletd hardening and cutover.** 08-29 closed three defects
  (argv-borne secret → DPAPI; unbounded proof threads → 6; no log → stdout
  redirect) and pinned the binary for the first time
  (BDCBE067…713C). The measurement that matters: **269.7s single-threaded
  subtree-cache build on every restart**, during which walletd accepts
  connections and answers nothing. This scales with leaf count on a 1 BPS
  chain — it gets worse weekly. v1.0.6 walletd claims 656s→~76s on exactly
  this path. Cutover is its own window; breaking change for `notes`
  consumers (`/api/wallet/balance` requires `?notes=1`) must be checked
  against the treasury page FIRST. Wallet checkpoints move to v8,
  forward-only — cold-copy the wallet dir before starting.
  Sub-item: `set-walletd-secret-r2` and a `run-walletd.cmd` house-convention
  launcher are both r2 debt.

## 5. SEQUENCING

```
STREAM A: ───────────────────────── SHIPPED 08-28 ─────────────────────────
   remaining: D1 (.bak retire, eligible now) · D2 (7-day verdict, 09-04)
                                                          │
                                              D2 pass ──► SCOPE retires,
                                                          v2.0.1.6 opens

P1 ── UNBLOCKED, unpasted since 08-27, losing data daily ── SHIP FIRST
P3 ── anytime · P2 ── after metric-name check · P5 ── after node-layout gate
P4 ── anytime (A5 work is reporter-side, no bridge dependency)

H2 window ── largest open item; carries PowerPanel + brick + migration
H5 ── gated on D2 · H6 ── independent, unblocks BL-032 · H8 ── own window
```

v2.0.1.6 seeds (not scope yet): A6 job-delivery latency; A1-hygiene bounded
retention; per-worker disconnect counter.

## 6. OPEN QUESTIONS AT r4

1. ANSWERED yes (A5): found/accepted/blue all timestamped at info.
2. ANSWERED yes (A6): stale AND disconnect counters both pre-exist.
3. ANSWERED (A1-hygiene): block-event gauges, ~5.4/hr, ghosts load-bearing.
4. What attributed the 01:31/03:33 dips? A1′ is a sufficient mechanism but
   was never confirmed against those specific dips. D2's clean week is the
   practical answer; a negative D2 reopens this as a distinct question.
5. Where does the KAS-leg difficulty gauge live for P1? Still unverified —
   read from `/metrics`, not memory.
6. Upstream watch: `firecash/zkas-pool` d79bf68 merged-mining YAML — re-diff
   before porting anything.
7. **NEW: what was the 02:55 08-28 pace gate result?** (`rc:solves_24h ≥ 33`
   = variance, < 33 = escalate to node-side forensics.) The 08-27 handoff
   set this gate; no result is recorded on any rail. Either read it from
   Prometheus while it is in retention, or close it explicitly as unmeasured.
8. **NEW: does the running zkas-node predate `e49ce61`?** Provenance was
   established for the PRE-cutover binary via a `ShieldedHistoryChunk`
   string probe; the v1.0.6 binary's layout generation gates P5.

## 7. UPSTREAM REPORT BATCH (queued, unsent)

Accumulated findings for firecash, to be filed together rather than
piecemeal: the v1.0.5 wRPC wedge report (drafted 08-26); walletd
consolidate-output mislabeled as `kind=coinbase` in history; the wallet
`.dmg` unsealed-signature bug (present since v1.0.6 of the wallet); the
config-key silent-drop trap (unrecognized TOML keys discarded without
warning); no version banner on the node binary; and `--wallet-secret` on
argv as a local-process disclosure with no env alternative documented.
