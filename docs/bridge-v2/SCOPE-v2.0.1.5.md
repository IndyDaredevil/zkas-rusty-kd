# SCOPE — Bridge v2.0.1.5 + Pipeline Stream P
### Drafted 2026-08-22 · Status: SCOPING (no code started)
### Convention: Stream A = bridge release contents (ships as one exe). Stream P = reporter/schema work (ships independently, per WS7 discipline: bridge grows log lines/metrics, sidecar grows consumers).
### References: BL-028, BL-029, BL-030; SESSION-STATE-2026-08-21 §4.7; 08-14 session state §2 (K/Z/D card).

---

## 0. STANDING RULES FOR THIS RELEASE

- **Aggregate-only telemetry.** No per-share POSTs anywhere in this scope.
  Reporter ships rollups; the webhook's idempotency key is not a license to
  firehose. Fine-grained collection requires a demonstrated need first.
- **Pipeline is the toolchain** (BL-021): anchored-replacement PowerShell
  patches → `merged-v2.0.1.5` branch → bridge-check CI as compile gate →
  branch-targeted release → win64 zip → canary `:5775` (w1c) → production.
  BL-030's banner guard is already in `deploy.yaml`; version derives from
  `CARGO_PKG_VERSION` + `BRIDGE_BUILD` — bump `BRIDGE_BUILD` to 5, nothing else.
- **Investigation before code, per item.** Three items (A2, A4, A5) begin with
  a source or query read that can shrink or delete the code change. Do the
  read first; the scope contracts, never expands, at that gate.
- **Production is sacred** (meta-principle 8): every bridge change canaries on
  `:5775` before fleet exposure. Rollback = relaunch prior exe via
  `run-rc-merged.cmd` (parked copy retained per BL-031 retention policy).

---

## STREAM A — BRIDGE v2.0.1.5

### A0 · GATE: BL-029 8h scrape-duration measurement  «blocks A1»
**What:** After v2.0.1.4 has ≥3 days unbroken uptime, run:
```
max_over_time(scrape_duration_seconds{job="rc_merged_bridge"}[8h])
```
and record the value ALONGSIDE THE UPTIME (BL-029's lesson: a duration without
uptime context is not evidence).
**Pass/fail:**
- Climbing toward seconds-scale → A1 is IN scope (mandatory; no timeout
  headroom exists — 14s vs 15s interval is the hard ceiling, BL-024).
- Flat at milliseconds → A1 drops to backlog; BL-029 closes as
  "hypothesis not confirmed at current fleet size," with the number and
  uptime recorded in the closing entry.
**Effort:** one query + one ledger note. Do this before anything else in
Stream A; it is the only item that changes the release's size materially.

### A1 · Series retirement (CONDITIONAL on A0)
**What:** Retire a worker's Prometheus series when its session disconnects,
so `/metrics` render cost stops growing with uptime.
**Design constraints (from the ledger, non-negotiable):**
- BL-025 retained `sum without (ip)` with `increase()` INSIDE the sum
  precisely because retired series coexist with live ones in the same scrape.
  If retirement removes that coexistence, the alert rules' assumptions change
  — re-verify all five block rules + RcReporterStarved against the new
  behavior BEFORE deploy, not after.
- Retirement must not destroy a series mid-`for:` window on an active alert
  (a retired series is a staleness marker, and BL-024 documents exactly what
  staleness markers do to lookback). Grace period ≥ the longest rule window
  (3m) + `for:` — suggest retiring only series idle ≥10m.
- The `wallet` label is a second churn vector (BL-007/BL-025 note) — the
  retirement key must be the full labelset, not just `ip`.
**Acceptance (canary):** connect/disconnect w1c repeatedly; `/metrics` series
count returns to baseline within the grace period; zero phantom cards; scrape
duration flat across the churn.
**Acceptance (production):** BL-029's 8h query re-run after 3 days on
v2.0.1.5 reads flat. This closes BL-029.

### A2 · `full_clear` semantics pin
**What:** ~91% of `[ZKAS] DOUBLE` lines carry `full_clear` (566/646) against
~25 real doubles. Pin the field's meaning against bridge source before it
ships to the dashboard or anywhere else.
**Step 1 (read, no code):** trace the `full_clear` write site in the RC
source; determine what condition actually sets it.
**Step 2 (one of three outcomes, decided by step 1):**
- (a) Field is correct, name is misleading → rename in log line + one-line
  doc; reporter regex updated in P-stream if it ever consumed it (it does not
  today — verify with a grep before claiming, per the negative-claims rule).
- (b) Field is a bug → fix, with a unit test asserting the intended
  condition (BL-005's lesson: turn the drift into CI red).
- (c) Field is correct and correctly named, our reading of it was wrong →
  ledger note only, zero code.
**Acceptance:** the next real double logs a value consistent with the pinned
definition; the definition is written into the scope-close ledger entry.

### A3 · Hashrate estimator drift
**What:** `estimated_hashrate/difficulty` reads 2.11/2.048 vs theoretical 2.0
at 1 BPS (BL-028 minor open) — consistent with the estimator using an
OBSERVED blockrate window rather than the target rate.
**Step 1 (read):** confirm in source which window the estimator uses.
**Step 2:** either switch to target rate (one constant) or document the
observed-rate choice as intentional with the expected drift band. Bias toward
the smallest change — this is cosmetic; a wrong "fix" that touches the 30s
stats loop is worse than the drift.
**Acceptance:** gauge ratio within a stated band, or a written rationale for
leaving it. Closes BL-028's minor-open tail either way.

### A4 · K/Z/D "24h" card bug — discriminator first
**What:** 24h rows displayed cumulative-since-restart values; luck divided
since-restart blocks by 24h expectation (08-14 session state, never
discriminated).
**Step 1 (query, no code):** run the recording-rule expr and raw
`increase(...[24h])` side by side. Two branches:
- Recording rule / Grafana math at fault → this is a MONITORING fix, exits
  Stream A entirely (config change, deployed-artifact-verified via
  /api/v1/rules per BL-020).
- Bridge stats loop at fault → in-scope code fix.
**Acceptance:** after ≥24h uptime, the 24h row ≠ the since-restart row, and
the two differ by exactly the pre-window increments.

### A5 · Instrumentation: submit latency + time-to-blue
**What:** per block, per leg: solve-detected → submit-sent → node-accepted
timestamps, plus blue-confirmation time (the 30×2s loop already polls the
condition; it does not record when it flips).
**Step 1 (read):** grep the RC source for existing timestamps on this path —
some or all may already be logged, in which case this item shrinks to a log
FORMAT change (structured, parseable line) rather than new instrumentation.
**Step 2:** emit ONE structured log line per block carrying all four deltas
(ms), keyed by H_zk — the reporter joins on the hash it already tracks. Also
export as Prometheus histograms ONLY if free; the log line is the mandatory
artifact (the accounting rail consumes logs; Prometheus retention would
discard the longitudinal value this exists for).
**Non-goal:** no alert rules on these in this release. Collect first,
threshold later — we do not know the healthy distribution yet.
**Acceptance (canary):** solve a share on w1c (or wait for a real block) and
verify the line parses; deltas plausible (submit latency ms-scale,
time-to-blue ~60s-scale).

### A6 · Instrumentation: stale-share rate + job delivery latency
**What:** per-worker stale/rejected share counters (if not already exported —
verify first) + node-notification → job-pushed-to-worker latency.
**Step 1 (read):** inventory existing share-outcome metrics in the RC. The
stale counter likely exists; the job-latency measurement likely does not.
**Step 2:** counters as Prometheus metrics (cheap, aggregate by nature);
job latency as a rolling gauge or summary — NOT per-job log lines.
**Kill criterion:** if the first week of data shows stale rate ≈0% fleet-wide,
this closes as "measured healthy, one ledger line" and P-stream never builds
a consumer for it. The measurement is the deliverable, not a dashboard.
**Acceptance (canary):** metrics present and moving on w1c under normal
share flow.

### A7 · Release mechanics + close-out
- Branch `merged-v2.0.1.5` from `merged-v2.0.1.4` @ `336b7a5`.
- `BRIDGE_BUILD` → 5. Banner guard verifies at release time (BL-030).
- Canary `:5775`/w1c: minimum 12h soak (c.15 precedent), acceptance = all
  item-level criteria above + 77/0/0-style clean share flow + zero FORENSIC
  IMPLAUSIBLE.
- Deploy: park/copy/hash-verify/kill/launch; four-way identity check
  (branch = origin = tag = running exe banner).
- Post-deploy: BL-029 8h re-measurement (if A1 shipped); ledger entries
  drafted (A0 gate result, A2 pin, A3 close, A4 branch taken); retire
  v2.0.1.4 `.bak-*` set after one clean production day (BL-031 policy).

---

## STREAM P — PIPELINE (parallel, independent shipping)

### P1 · D_z/D_k history sampler  «no bridge dependency — can ship TODAY»
**What:** reporter (or a second tiny scheduled task) samples
`ks_zkas_network_difficulty_gauge` + KAS-leg gauge from `:9151`-adjacent
Prometheus every 5m, POSTs a rollup row to a new `network_history` table.
BL-028 measured 9%/20min and 30%/day moves — the curve is unrecoverable once
it scrolls out of Prometheus retention.
**Schema:** `network_history(sampled_at, d_z, d_k, ratio, est_hashrate_z,
est_hashrate_k)` — no dedup key needed (append-only time series), but a
unique on `sampled_at` truncated to the sample grid makes replay idempotent
anyway. Cheap insurance, take it.
**Footprint:** 288 rows/day. Negligible.

### P2 · `worker_stats` hourly rollup — effective hashrate
**What:** hourly per-worker row: accepted-share difficulty sum → measured
hashrate; share counts; stale count (once A6 lands, else omit the column).
Closes BL-016(a) properly: Luck vs MEASURED hashrate = real Poisson luck;
vs nameplate = capture KPI — keep both. Settles the w1m ~7% question with an
actual average instead of snapshots.
**Source:** existing bridge metrics via Prometheus query API (the reporter
already knows `curl.exe -sG --data-urlencode` patterns) — verify the exact
metric names against `/metrics` before writing the query, not from memory.
**Schema:** `worker_stats(hour_start, worker, diff_sum, shares_accepted,
shares_stale NULL, hashrate_measured)` — unique(hour_start, worker) as the
idempotent replay key.
**Footprint:** 168 rows/day at 7 workers.

### P3 · Reconnect/authorize events
**What:** reporter parses authorize lines from the log it already tails; one
row per event: `worker_events(occurred_at, worker, wallet_captured, ip)`.
Feeds BL-029's series-growth model, rig stability trending, and a standing
cross-check on the BL-007 wallet-capture trap (a row with an unexpected
wallet is the alarm).
**Footprint:** near-zero in steady state; that is itself the signal.

### P4 · Latency consumer  «depends on A5 shipping»
**What:** extend the reporter to parse A5's structured line and attach the
four deltas to the existing `zkas_blocks` row — as a beat-2 field extension
if timing allows (the line lands before the T+60s beat), else a third beat
on the same key. Prefer extension: two beats on one key was the WS7 design;
don't add a transport tier without need.
**Schema:** four nullable integer-ms columns on `zkas_blocks`. Nullable
because 659 historical rows will never have them.

### P-mechanics
- Schema changes via Bolt with the constraints-first brief format that worked
  on 08-21, PLUS the banked addendum: **never reinit git history**; Bolt
  cannot set edge secrets (operator does); no RLS loosening.
- Reporter update procedure per banked runbook: stop task → overwrite →
  start → verify RUNNING artifact (reporter.log "metrics listener" line +
  `:9151/metrics`).
- Each P item gets its own reconciliation check on first live day (row counts
  vs source-of-truth query), per the sompi-exact discipline.
- P1 and P3 have zero Stream-A dependency; P2 needs only a metric-name
  verification; P4 waits on A5. Ship P1 first as the pattern-prover for
  "new table + rollup beat."

---

## SEQUENCING SUMMARY

```
A0 (query, today-ish, needs ≥3d uptime) ──► A1 in/out decision
A2.1, A3.1, A4.1, A5.1, A6.1 (all READS — one investigation session, no code)
        │
        ▼
A-code session(s): only what survived the reads
        ▼
canary 12h ──► deploy ──► post-deploy measurements ──► ledger close-out

P1 ── ships independently, anytime (recommended first)
P3 ── ships independently, anytime
P2 ── after metric-name verification
P4 ── after A5 is in production
```

**Two sessions of reads before one session of code.** Every A-item except A1
begins with an investigation that can shrink it; A1 begins with a gate that
can delete it. The honest expectation: v2.0.1.5 could be anywhere from a
two-item release (A5+A6 instrumentation only) to a six-item release, and the
reads decide which — not this document.

## OPEN QUESTIONS PARKED AT SCOPE TIME
1. Does the RC already log submit/accept timestamps? (A5.1 answers.)
2. Does a stale-share counter already exist per worker? (A6.1 answers.)
3. Upstream watch: `firecash/zkas-pool` d79bf68 (merged-mining YAML wiring) —
   re-diff before porting anything, standing ledger note. Not in scope unless
   the diff shows something we want.
4. Where does the KAS-leg difficulty gauge live for P1 — same stats loop?
   (Verify metric name from `/metrics`, not memory.)
