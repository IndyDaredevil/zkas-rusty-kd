# SCOPE — Bridge v2.0.1.5 + Pipeline Stream P + Host Stream H
### Content r3 · revised 2026-08-27 (same day as r2) · Status: A1'+A8 ON
### BRANCH merged-v2.0.1.5 @ 1b63698; canary PASSED banner+parker gates,
### soaking since 14:21 ET. All reads COMPLETE; A2/A3/A4 closed by reads.
### r1 (2026-08-22) superseded in-place per the naming law: same filename,
### header carries the content revision. r2 absorbs KICKOFF-v2.0.1.5-r5 §2–§5
### (the 08-22..26 investigation layer that superseded r1's Stream A) plus the
### S11/S12 session results (H1/H1b executed, ledger resealed @ BL-043,
### BL-044 banked, UPS installed, seventh power event). The KICKOFF hereby
### retires to historical session seed; THIS document is the scope of record.
### Convention: Stream A = bridge release (one exe). Stream P = reporter/schema
### (ships independently, WS7 discipline). Stream H = host work.
### References: ENGINEERING-LEDGER sealed @ BL-043 (commit 5179e65); BL-044
### banked in the S11/S12 thread, appends with this document's commit.
### r3 records the read outcomes and implementation state; «READ-GATED»
### markers are retired — every gate has answered. BL-045..049 ledgered.

---

## 0. STANDING RULES FOR THIS RELEASE

- **Aggregate-only telemetry.** No per-share POSTs anywhere in this scope.
  Reporter ships rollups; the webhook's idempotency key is not a license to
  firehose. Fine-grained collection requires a demonstrated need first.
- **Pipeline is the toolchain** (BL-021): anchored-replacement PowerShell
  patches → `merged-v2.0.1.5` branch → bridge-check CI as compile gate →
  branch-targeted release → win64 zip → canary `:5775` (w1c) → production.
  BL-030's banner guard is in `deploy.yaml`; version derives from
  `CARGO_PKG_VERSION` + `BRIDGE_BUILD` — bump `BRIDGE_BUILD` to 5, nothing else.
- **Investigation before code, per item.** Every surviving A-item begins with
  a source or query read that can shrink or delete the code change. The scope
  contracts, never expands, at that gate.
- **Production is sacred** (meta-principle 8): every bridge change canaries on
  `:5775` before fleet exposure. Rollback = relaunch prior exe via
  `run-rc-merged.cmd` (parked copy retained per BL-031 retention policy).
- **Single-process reality** (BL-039): both stratum "instances" are listeners
  in ONE OS process (`stratum-bridge`, one PID owning :5755/:5765/:3034) —
  one kill target in every deploy sequence, one crash domain across both
  fleets. BRIDGE-SPEC §2 clarification rides this release's doc pass.

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
   never measured (outlived every ceiling; the 55s timeout is the standing
   best chance).
4. **Morning stalls SOLVED** (BL-035): Store retry-grind on ScreenSketch;
   killed 08-22; zero morning dips in 4 days — prediction confirmed.
5. **Night dips: composite reading WEAKENED by experiment** (BL-036). H1
   summon (08-27): three event-log-verified RDP churn cycles over a live 2s
   probe — floor unbroken; churn REFUTED as sufficient trigger. Defender
   exonerated for the 01:31/03:33 08-26 pair (31 min offset; nightly
   metronome cannot produce episodic effects). Those two dips remain
   UNATTRIBUTED; the discriminating variable is likely bridge/node-side
   (rpc_ms coupling, survivorship caveat), not host-side. Consequence: A1′
   has NO reproducible trigger harness — the source read proceeds without
   one, and suspect ranking tilts toward what churn cannot explain.
6. **The Stream-A headline:** why does host pressure turn a 230ms render into
   a ≥25s freeze rather than a slow render? Suspect classes: blocking
   write/flush on the render path, sync RPC reachable from the handler,
   runtime-pool starvation (blue-confirm loop 30×2s sync on the async pool).
   The single-process finding (§0) raises the stakes: whatever starves the
   render shares a crash/starvation domain with both fleets' stratum.
7. **Host context now instrumented-adjacent** (BL-040/BL-044): six-event
   uncommanded power series closed as MIXED etiology — 8/15–16 convicted
   Kron-local (19V brick/DC-barrel class; rigs rode through), 8/27 outage
   premises-wide; UPS ×3 installed 08-27, converting every future event into
   a one-bit discriminator. 6008 heartbeat lag calibrated ~35 min against a
   known death time. Relevant to Stream A only as environment: night-dip
   attribution cannot lean on power events (none coincide).

## 2. STREAM A — BRIDGE v2.0.1.5

### A1′ · HEADLINE — ROOT CAUSE FOUND & FIX IMPLEMENTED (BL-045)
**Read outcome:** `serve_http_loop` was serial, spawn-less and timeout-less;
any idle/half-open client (canonically the operator's own dashboard browser)
parked the whole server at read(). True render floor 5ms. Fix (on branch):
spawn-per-connection + 5s read timeout. Deterministic acceptance PASSED on
canary 1b63698: silent-parker stimulus that pins v2.0.1.4 at a measured
8,048ms yields 213–232ms × 8 on v2.0.1.5, parker evicted by FIN at 5s.
**Remaining acceptance:** soak clean + zero timeout-kills across one week
of production post-deploy.
<historical r2 text follows for the record>
**Read first:** trace the render path for anything that can block ≥ seconds
(locks shared with RPC work, sync I/O, pool starvation — the blue-confirm
loop is the named pool-starvation suspect). Anchors: BRIDGE-SPEC §5 (metrics
contract) + §6 (logging contract) as as-built reference; discrepancies vs the
tree are ledger entries per that spec's ground-truth clause. No harness
exists (H1 negative) — the read is unassisted by reproduction.
**Fix shape:** decided by the read; structural end-state is a lock-free /
snapshot render nothing the bridge does can starve.
**Acceptance:** zero timeout-kills across one week of production. (r1's
"declared host-pressure windows" clause is retired — H1 showed we cannot
declare such windows on demand; the week must stand on its own.)

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
<historical r2 text follows for the record>
Explain the ~6/hr smooth mint (what runs hourly per worker?), then retire
idle series with BL-025-aware design: grace ≥10m; full-labelset key (wallet
label is a second churn vector); re-verify all five block rules +
RcReporterStarved against the new coexistence behavior BEFORE deploy
(retired series are staleness markers — BL-024's lookback lesson).
**Acceptance (canary):** churn w1c; series count returns to baseline within
grace; zero phantom cards; scrape duration flat across churn.

### A2 · `full_clear` semantics — CLOSED by read, outcome (c): zero code
**Read outcome:** full_clear prints meets_network_target (pow ≤ KASPA
network target) on a line that only prints inside clears_zkas — it
literally means "double". The ~91% rate is physics: P(double|zKAS block) =
d_z/d_k ≈ 87% at current regime, matching both the 566/646 log rate and the
six-day table (284/348). The "~25 real doubles" baseline was an
incompatible early-era cohort. Field correct, correctly named; ledger note
only.
Unchanged from r1: trace the write site; outcome (a) rename / (b) fix with
unit test / (c) ledger note only. ~91% of DOUBLE lines carry it against ~25
real doubles. Reporter does not consume it today (verify with grep before
claiming, per the negative-claims rule).
**Acceptance:** next real double logs a value consistent with the pinned
definition; definition written into the scope-close ledger entry.

### A3 · Hashrate estimator — CLOSED by read: documented, zero code
**Read outcome:** both legs call the node's
estimate_network_hashes_per_second RPC with a 1000-block observed window
anchored at tip; the bridge computes nothing. Ratio wobble is 1000-block
luck (σ≈3.2%); observations 2.11 (+5.5%) and 1.956 (−2.2%) both inside 2σ.
BL-028's minor-open tail closes.
Unchanged from r1: confirm which window the estimator uses (observed vs
target blockrate); smallest change wins — cosmetic; a wrong "fix" touching
the 30s stats loop is worse than the drift. Closes BL-028's minor-open tail
either way.

### A4 · K/Z/D "24h" card — CLOSED: rules faithful; EXITS Stream A
**Discriminator run 08-27 (restart-rich window = ideal stressor):** rule ≡
raw exactly on all three pairs (32/35/27; solves 40), and raw reconciles
with log-truth within reset-compensation noise (±1 across three restarts —
tolerable). Not the stats loop, not rule evaluation. Residual: one eyeball
of the HourlyMergedReport card vs live values; any remaining discrepancy is
the Alertmanager template leg (config-side).
Unchanged from r1: run the recording-rule expr and raw `increase(...[24h])`
side by side after ≥24h bridge uptime. Monitoring at fault → exits Stream A
(config fix, artifact-verified via /api/v1/rules). Bridge stats loop at
fault → in-scope code fix.
**Clock:** the 08-27 02:05 outage reset the window — bridge StartTime
02:55:25 08-27; discriminator valid from ~02:55 08-28. (Third arming: 08-26
16:00 restart re-armed it once already; any future restart re-arms again.)

### A5 · Submit latency + time-to-blue — MIGRATED to Stream P (near-zero
### bridge code)
**Read outcome:** submit processing latency already exists (merged_obs #4,
µs, surfaced as sub=avg/max), and FOUND → ACCEPTED → confirmed-BLUE are
already timestamped info lines — the reporter can compute the deltas from
lines it already tails. Only submit-SENT lives at debug. Collect
reporter-side first; a structured bridge line only if per-block submit-send
granularity proves necessary.
Unchanged from r1: grep for existing timestamps first (may shrink to a log
FORMAT change); one structured line per block, four deltas (ms), keyed by
H_zk; Prometheus histograms only if free — the log line is the mandatory
artifact. No alert rules this release; collect first.

### A6 · Stale-share / job latency / disconnect counter — SHRUNK to job
### latency only; DEFERRED to v2.0.1.6
**Read outcome:** ks_invalid_share_counter (per-worker, reason-labeled) and
ks_worker_disconnect_counter both already exist — the W9 watch item is
queryable today. Genuine remainder: job-delivery latency gauge; its
dispatch site is not a single obvious location (notification_hub → mining
state → per-client sends), so it rides the next release rather than this
one half-read.
r1 items unchanged (inventory existing share-outcome metrics first; counters
as metrics; job latency as rolling gauge/summary, not per-job logs; kill
criterion: stale ≈0% fleet-wide for a week closes it as "measured healthy").
**Added (BL-044):** per-worker disconnect/reconnect counter — the W9
spontaneous self-reboot (~8/25) was invisible to all current alerting
(~2-min stratum gap, under every threshold). Cheap counter now; threshold
only if W9 repeats.

### A8 · Balance-WARN circuit breaker — IMPLEMENTED (on branch)
Site: client_handler.rs periodic fetch. Breaker: 10 consecutive failures →
one INFO, scheduling stops for process lifetime; any success resets. Live
volume confirmation: the dead WARN was ~120–130/h of the entire WARN
channel (501/4.2h, 557/4.3h measured 08-27) — the breaker restores WARN
visibility wholesale. Canary console shows the single INFO in-soak.

### A7 · Release mechanics + close-out
- Branch `merged-v2.0.1.5` LIVE @ 1b63698 (eda7090 = A1'+A8+build-bump;
  1b63698 = BL-047 CI cache fix). Canary binary
  F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0.
- `BRIDGE_BUILD` = 5 (done). Banner guard verifies v-tags at release time
  (BL-030) — and per BL-047 the guard is source-side only: the BANNER GATE
  on the running artifact precedes any soak, as executed for canary-1b63698.
- Canary `:5775`/w1c: minimum 12h soak; acceptance = all item-level criteria
  + clean share flow + zero FORENSIC IMPLAUSIBLE.
- Deploy: park/copy/hash-verify/kill/launch; four-way identity check. Single
  process (§0): one kill, one launch, both instances ride together.
- Docs pass rides the release: BRIDGE-SPEC §2 single-process clarification;
  ledger entries (A-item outcomes); retire v2.0.1.4 `.bak-*` set after one
  clean production day (BL-031 policy).

## 3. STREAM P — PIPELINE (unchanged from r1; not started)

P1 network_history (D_z/D_k 5-min sampler) — **SHIP FIRST; the curve is
being lost continuously at 15-day retention.** P3 worker_events → P2
worker_stats (metric-name verification first — from `/metrics`, not memory)
→ P4 latency consumer (gated on A5 in production). Full item specs: r1 §P1–P4
text is carried verbatim in git history and remains authoritative for
schemas/footprints; constraints refreshed here:
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

- **H1 summon experiment: DONE 08-27, negative** (BL-036) — churn refuted as
  sufficient trigger; no harness for A1′.
- **H1b posture: DONE 08-27** (BL-041/BL-039) — NLA on; WAN boundary audited
  (two deliberate P2P forwards 16111/16811 recorded as policy; IP
  Passthrough off; IPv6 firewall clean; SG116E characterized). r5's
  prefer-sign-out recommendation RETRACTED: six production processes live in
  Session 1; sign-out is a whole-stack kill until migration (H2).
- **H2 maintenance window — PARTIALLY DONE.** UPS ×3 installed 08-27 (early,
  forced by the seventh event). Remainder, one window: KRON-HARDENING §2–§6
  application (artifact gates per that doc) + Store/OS update flush +
  service/scheduled-task migration of the six Session-1 processes
  (ZkasReporter pattern; kills the 3 AM manual-relaunch cost measured in
  BL-044) + deliberate bridge relaunch. Procurement rider: spare 19V
  brick/DC barrel (BL-040/044 convicted class — the fault the UPS cannot
  cover). Post-window: KRON-HARDENING §8 gates.
- **H3** Hardening acceptance: KRON-HARDENING §9, 7 quiet days from
  application.
- **H4** Clock discipline: **CLOSED HEALTHY 08-27** (BL-046) — steady-state
  steps ±1–23ms per ~30min, boot-era correction +318ms after a 45-min dark
  RTC. Second-exact joins validated; BL-042 closed.
- **H5** Revisit global-vs-per-job scrape config (reporter back to 15s) once
  the bridge render is trusted again (post-A1′).
- **H6** BL-032 instrumentation & escalation package: windows_exporter +
  RAM% / commit-ratio / per-process-RSS / exporter-down rules (runbook
  drafted 08-26) + page-tier `ZkasLegDegraded` on sustained template age
  (detection existed at T+30s; the 3.8h gap was escalation). Config-side;
  artifact-verified via /api/v1/rules per BL-020.
- **H7** Node-side BL-032 fixes: zkas-node v1.0.6 with file logging ON (the
  wedge had no node-side witness); `--ram-scale` on kaspad; gRPC firewall
  rules scoped to the MacBook IP (the 08-24 LAN opening left unauthenticated
  RPC reachable by seven unaudited-firmware rigs). Upstream wedge report to
  firecash rides this item (draft exists, 08-26 session).

## 5. SEQUENCING

```
queue-5 READS SITTING (one sitting, no code):
  A1' source read + A2/A3/A5/A6/A8 reads · A4 query gate (valid ~02:55 08-28)
        ▼
  r2 line-edits from read outcomes (this doc, content r3 only if shape changes)
        ▼
  A-code session(s): only what survived ──► canary 12h ──► deploy ──►
  post-deploy measurements ──► ledger close-out

P1 ── ships independently, ANYTIME (recommended before the reads)
P3 ── independently, anytime · P2 ── after metric-name check · P4 ── after A5
H2 remainder / H6 / H7 ── operator-scheduled, independent of Stream A
```

The honest expectation stands: v2.0.1.5 is anywhere from a two-item release
(A5+A6 instrumentation) to a seven-item release, and the reads decide —
not this document.

## 6. OPEN QUESTIONS AT r2

1. ANSWERED yes (A5): found/accepted/blue all timestamped at info.
2. ANSWERED yes (A6): stale AND disconnect counters both pre-exist.
3. What runs hourly per worker to mint ~6 series/hr? (A1-hygiene read.)
4. What attributes the 01:31/03:33 dips? (Likely falls out of the A1′ read's
   suspect ranking; otherwise stays open — no harness exists.)
5. Where does the KAS-leg difficulty gauge live for P1? (Verify from
   `/metrics`, not memory.)
6. Upstream watch: `firecash/zkas-pool` d79bf68 merged-mining YAML — re-diff
   before porting anything; not in scope unless the diff shows something we
   want.
