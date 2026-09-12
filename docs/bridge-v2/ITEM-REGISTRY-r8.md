# ITEM REGISTRY — the operation's project-management list
### Content r8 · created 2026-09-03, revised 2026-09-11 (r8: P8, P11, T-7, R-6 CLOSED; T-1 split; H9 and A-stream opened; catches up to ledger tip BL-121) · r7 was 2026-09-06 · Successor to the registry role of SCOPE-v2.0.1.5 (retired at D2)

**Why this doc exists:** SCOPE carried the item definitions and retired with
its version (D2 pass, 09-03) — leaving codes referenced everywhere and
defined nowhere living. Definitions live HERE now, in a doc that does not
retire when a version ships. Maintenance rule: update when an item opens,
changes shape, or closes; closed items stay listed one revision, then move
to the CLOSED section; codes are never reused. Ledger entries remain the
evidence; this is the map.

---

## D — DATED DECISION GATES

- (none open; D1/D2 in CLOSED — PRIOR REVISIONS)

## H — HOST STREAM

- **H2 · auto-start + runtime-visibility program — OPEN, front of queue,
  RESHAPED at BL-097** (the August StartKron orchestrator: SYSTEM-context
  task, never completed a boot 4/4, killed 08-17 — post-mortem findings
  F1–F5 govern this item; "run as services" is retired language). Staged:
  **(a) Visibility contract per process** — log file + Prometheus metric +
  Button line + viewer command for all seven (kaspad nologfiles=false and
  a versioned kaspad launcher are the open pieces); consoles become
  optional. **(b) Boot-start v2** — inmyh principal (never SYSTEM),
  network-readiness pre-gate, v1's validated internals (tiers, port
  gates, idempotency, env-baking, logging) wrapped try/finally, **with
  its own deadman** (end-of-run healthcheck ping — a truncated run pages
  in minutes, not days); deadman+sampler principal fixes ride here;
  proven across a deliberate reboot before it counts. **(c) Supervision**
  — separate later decision, not conflated (F3). Measured argument
  unchanged: event #9 = host 34s, production 24 min. Added 09-11: the
  reporter's console was closed by hand (operator statement), the process
  died, nothing paged — only the Button found it (9151=False, ~1h later).
  A visibility contract would have. Owner: next multi-hour Kron window.
  Precondition minted 09-11: the Button (`check-kron.ps1`) runs from an
  ELEVATED prompt — walletd is high-integrity, and a non-elevated checker
  reads its path as null and reports a false IMPOSTOR (n=1, two runs).
- **H6 · memory alerting — RESHAPED to rules-only.** No config change;
  the rule is `windows_memory_swap_pages_written_total` sustained climb.
  RAM% rules are wrong by construction (79% = configured equilibrium,
  BL-081). Executes at the RULES SITTING.
- **H9 · reporter r9 — start gauge + DPAPI wallet token — OPEN, one
  restart.** Two riders on one cut: (a) the reporter is outside
  windows_exporter's include regex, so P8's restart logging cannot see it
  — r9 exports its own start-time gauge (BL-119); (b) the wallet token is
  plaintext at reporter line 75 and `GET /api/wallet/reveal` returns
  `seed_hex` to its bearer over loopback (BL-116 hardening finding; stands
  at BL-120) — token moves to a DPAPI blob per the set-*-secret pattern.
  Upstream ask (reveal gated by wallet secret or disableable;
  `--no-custodial` may already gate it, n=0) is a separate question.
  Live reporter = r8 (542 ln, e01206ae…, BOM, BL-118).

## P — PIPELINE STREAM

- **P1 · network-history curve (D_z/D_k) — OPERATIONAL, sampler r2.**
  5-min sampler → Supabase; r2 (BL-114, 7f586bf0…) adds raw near-miss
  counters per chain (near_miss_kas/zkas/workers; negative delta = bridge
  restart, skipped). Known gaps annotated in-ledger (S15 forensics window;
  event #9 ~5 buckets). Consumers must not read gaps as anomalies.
- **P2 · dashboard delivered-hashrate — RESOLVED at BL-099.** Nameplate
  14.2 stays BY DESIGN (BL-016(b) capture-efficiency KPI, now labeled);
  a new constant would violate BL-028; Netlify-remote rules out LAN
  Prometheus reads. Delivered figure: no fleet gauge exists on
  /metrics (BL-114) — delivered hashrate stays a Prometheus rule; a
  Supabase copy would need the reporter to post it (queued behind H9). Dashboard
  sitting CLOSED 5/5 w/ 28.4-vs-28.9 cross-check; future card noted:
  "Expected (network)" beside pace = drought instrument in UI.
- **P11 · dashboard remainder — CLOSED at BL-113/BL-117.** Expected-vs-
  pace (shared expectedBlocksPerDay(), rows 44/46, matched to 0.1) and
  kron_events (replaces restarts; 17 events migrated; five event types;
  markers on daily/weekly/monthly) shipped rows 44–57. Row 27 screenshot
  closed on synthetic-row captures (BL-117). Dashboard rows 63–81 closed
  the same day (reconciliation labels, health header re-based on the 7d
  gap mean, Poisson panel replaces Restart Backtest).
- **P5 · block-detail expansion — OPEN, gated.** Gate: confirm
  post-e49ce61 layout on the running node. Mergeset-persist rider folded
  in.
- **P6 · treasury push — OPEN, unblocked by H8.** Two paths: walletd
  polling (BL-052 zeroed-object discipline + v1.0.7 `missing_history`
  refusal handling) OR the wallet-app view-key/watch-only route (v1.0.29).
  Design decision before build.
- **P7 · chain-block share — CLOSED at BL-107.** Persisted: KAS via
  `cached_transactions.is_accepted` (payment copy, red = absence); zKAS via
  amount>0. Dashboard rule replicated exactly (payload subsidy decode).
  Findings: Toccata 06-30 halved the share 40.5→17.2% (merge set
  unchanged); the custom bridge lifted it 16.9→22.4% (~4σ); 200 ms poll vs
  WS2 listener not separable (n=155). Optional card: chain share by week
  with the 06-30 and 08-02 markers (one handoff row).
- **P8 · restart logging — CLOSED at BL-119, both halves.** Metric is
  `windows_process_start_time_seconds_timestamp` (the earlier candidate
  lacked the suffix); recording rule `kron:process_start_seconds` = max
  without (process_id, creating_process_id); alerts KronProcessRestarted /
  KronHostBooted (rules hook r2, alert_rules.yml 962 ln, promtool 48).
  Write half: kron-events-webhook (fail-closed secret, dedupe_key
  component:epoch) + KronEventsSampler task every 5 min (r2, 152 ln).
  Proof: POST pair inserted/duplicate 7063477b. Reporter not covered —
  see H9.

## R — RULES SITTING (one deliberate sitting, batched)

- **R-1** walletd distress alert: fire on poll-failures INCREASING or
  age-while-pending — never naive age; `missing_history`-aware post-H8.
- **R-2** ZkasLegDegraded (BL-032's original ask; collection exists).
- **R-3** H6's swap rule (above).
- **R-4** Firewall rationalization: eight program-scoped Any/Any allows
  (kaspad.exe/kaspad/zkas-node) undermine port scoping — profile-aware
  audit, explicit rules made authoritative (KRON-HARDENING §6.8).
- **R-5** RcReporterDown fire drill — never run.
- **R-7** Reporter: widen the TG edit-path WARN to log Telegram's error
  text (duplicate-card diagnosis, BL-105).

## T — TRIGGER-DRIVEN (no action until the trigger)

- **T-1a** Pi 5 4 GB (on hand, KEPT, BL-120): off-host deadman for Kron
  (the one alarm Kron cannot raise about itself), NUT master for all 3
  UPSs → drill (3 pulls, 3 witnessed alerts), trigger pollers (pwsh 7
  linux-arm64, systemd timers, no DPAPI), second supply-check witness.
  NOT a node (Kron's zkas-node peaks ~6 GB). KRON-HARDENING §6.5.3 is
  the spec.
- **T-1b** Kron-2 (second archival node / warm standby): first unit
  RETURNED (24 GB against 32 listed — six-point acceptance held, BL-120).
  Re-order on correct stock; rebuild from runbooks, never disk-image.
- **T-2** Third 1500VA (arrival→physical window): KS7-per-unit end-state,
  w8m to battery, canary role retires, gateway cable drop rides.
- **T-3** ERRSTREAM soak: next console flash → one Select-String names it.
  (Two live catches banked: TG 400 UTF-8 at BL-099; walletd refusals at
  BL-087 — the instrument earns its keep.)
- **T-7 · RESOLVED at BL-118.** The "loss" was a reporter parse gap: the
  bridge logs a double in two forms (`H_fc:` and `Parent:`); r6 parsed
  one. r8 parses both; 45 rows backfilled; since 08-30 = 90.2% (n=542),
  lifetime 86.0%. Zero red/orphan loss on the KAS leg; every false is a
  full_clear:false (difficulty), tracking d_z/d_k as the operator said.
- **T-4** Brick experiment (running since 09-02 14:12): falsifier =
  another 41/6008 + zero PowerPanel rows → convicts barrel/board.

## A — AUDITOR / SUPPLY (new stream, r8)

- **A-1 · shielded archive — PARKED (BL-111, plan revised).** Replay of
  record = a fresh archival node resync on the corrected verifier; per-
  bundle re-verification is the wrong unit. Schema r1 + revoke_anon r2 +
  extractor r4 + first 20 blocks stay in Supabase; backfill NOT on Kron.
  Owed: tell covenants it is on hold (one row). JWT kid 0b8614eb… expires
  2027-09-07 — rotation deadline if ever revived.
- **A-2 · supply-check r8 — LIVE hourly** (BL-108): drift baseline
  bimodal {+9.72, +11.97} ZKAS, quantum 2.2475 = one dev-fee cut; three
  rules (Stale/Fail/Drift). Established limit: cannot see a proof-system
  soundness failure (BL-110) — only a viewing key, a second circuit, or an
  audit epoch could.
- **A-3 · fork proposal (audit epoch) — HELD** on zcash/ironwood#223
  (is `aProgramBase` a soundness object?). Draft r2 90cf2344….
- **A-4 · stage tables** — rule minted BL-120: named at creation, retired
  at close (IV.9 for tables). zkas_double_backfill_stage dropped 09-09
  after the linter found it without RLS; archive on the laptop.

## W — WALLET APP (MacBook, custody + Covenants++ harness)

- **W-1 · zkas-wallet#3 — FILED 2026-09-06, OPEN.** Desktop connects to
  the public node pre-unlock; own-node choice reverts to "Set up"; node
  left stopped; run-mode dialog defaults to Mining. Published body pinned
  1fe41ce1…; comment #1 = plist 1.0.31-1 + post-unlock lsof. Watch for
  maintainer reply; interim procedure: Own node + Shielded history first
  on every launch. Verification rule for this app: badge + asset sha,
  never About/plist.
- **W-2 · release cadence** — on 1.0.32 (d49597a1…). Batch-by-default;
  upgrade on desktop custody/key/money changes or to reproduce for #3.

## DOCS SWEEP (batch)

NODE-CONTRACT-v1.0.8 (new, post-H8) · NODE-CUTOVER r2 (fix -AsByteStream,
BL-084(4)) · STARTUP-ORDER r2 (/api/wallet/warm; §8 Button ELEVATED; walletd
high-integrity; service names post-H2) · check-kron r4 (null-path guard:
PATH-UNREADABLE, never IMPOSTOR, on an unreadable read) ·
WINDOWS-EXPORTER runbook (conversation-only; two fixed defects undocketed) ·
FLEET-DEPLOY r2 · BRIDGE-SPEC §2 · covenant/silverscript note (C1–C3
intersections; #218 numbers gate C3) · grafana.exe record-or-retire ·
wallet-app release check → session-open habit · fork rebase · cold-storage
sweep (case strengthened twice this week).

## UPSTREAM REMAINDER

Status 09-11: zkas-rusty#8 LANDED 737521e (author IndyDaredevil, BL-109);
A2 fail-closed guard pre-approved in principle, GATED on a zakura-orchard
release > 1.1.0 carrying zakura-core/common#362 (merged b4f90dd). Open
docs: finality-ladder docs PR (params.rs F-04/F-05 cites) · surplus/third-party
miner docs note (txids to assemble) · anchor-semantics note (#6 Q3,
pre-offered; cite BL-086's sink/virtualParentHashes discriminator).

## v2.0.1.6 (OPEN as of D2)

`tip_hashes.first()→None` both legs (maintainer-endorsed) · mimalloc
purge_decommits A/B (evidence: 14.8 GB private vs 3.9 resident) · A6
job-delivery latency · A1-hygiene bounded retention · reporter: feed
webhook's `difficulty` at BEAT1 (behind H9).

## CLOSED THIS REVISION (r8)

P8 (BL-119) · P11 (BL-113/117) · T-7 (BL-118, reclassified: parser gap)
· R-6 (BL-106, check-kron consolidated) · P7/P9/P10/T-5/T-6 rotate out
after their one listed revision (r7).

## CLOSED — PRIOR REVISIONS

r7: D1 · D2 · H7 · H8 (executed BL-093; five acceptance gates BL-098:
beat2 p50 198.9s · ERRSTREAM quiet · poll-failures flat · 3 non-LAN peers
on 16811 · give-ups 0; unblocked P6, R-1, the give-up reconciliation)
· A3 (stranded tip: discovered→filed #6→fixed 9a464d51→v1.0.8→healed on
Kron 09-03, ~66h) · UPS-rebalance rider (executed by hand 09-02, map in
KRON-HARDENING §6.5) · P7 (chain-block share, BL-107) · P9/P10 (page cap,
double KPI, BL-106) · T-5 (#6 closed 09-01) · T-6 (first card 09-04).
