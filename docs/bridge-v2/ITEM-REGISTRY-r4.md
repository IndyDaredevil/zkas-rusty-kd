# ITEM REGISTRY — the operation's project-management list
### Content r4 · created 2026-09-03, revised 2026-09-04 (r4: TG cards live, dashboard sitting closed, P2 resolved — BL-099) · Successor to the registry role of SCOPE-v2.0.1.5 (retired at D2)

**Why this doc exists:** SCOPE carried the item definitions and retired with
its version (D2 pass, 09-03) — leaving codes referenced everywhere and
defined nowhere living. Definitions live HERE now, in a doc that does not
retire when a version ships. Maintenance rule: update when an item opens,
changes shape, or closes; closed items stay listed one revision, then move
to the CLOSED section; codes are never reused. Ledger entries remain the
evidence; this is the map.

---

## D — DATED DECISION GATES

- **D1 · .bak-v2014 retirement — CLOSED 09-03.** Five artifacts (4 source
  + rollback exe) deleted against sha'd identity record after D2 made the
  rollback path moot. Coupling rule proven: D-items that depend on another
  gate ride ITS sitting, never standalone. (BL: pending S20 append.)
- **D2 · enlarged-scrape verdict — CLOSED 09-03, PASSED.** Gate:
  max scrape_duration over the post-deploy window < 5s. Result: 0.0491s /
  8,031 samples (135h exact-scoped, incl. event #9 restart); called early
  at 5.65/7d by operator decision. Consequences executed: BL-050 closed,
  SCOPE retired, v2.0.1.6 opened.

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
  unchanged: event #9 = host 34s, production 24 min. Owner: next
  multi-hour Kron window.
- **H6 · memory alerting — RESHAPED to rules-only.** No config change;
  the rule is `windows_memory_swap_pages_written_total` sustained climb.
  RAM% rules are wrong by construction (79% = configured equilibrium,
  BL-081). Executes at the RULES SITTING.
- **H7 · gRPC scoping — CLOSED both legs.** zkas 16810 (08-31) + kaspad
  16110 (found executed 09-02, rule 'Kaspa gRPC MacBook only') both
  MacBook-only. Successor concern: see Rules Sitting item R-4.
- **H8 · v1.0.8 cutover — CLOSED at BL-098** (executed overnight 09-03,
  BL-093; five acceptance gates passed 09-03 evening: beat2 p50 198.9s ·
  ERRSTREAM quiet · poll-failures flat, BL-089 hypothesis confirmed ·
  3 non-LAN peers on 16811 · give-ups 0 steady-state, 2 outage-window
  rows annexed to the give-up reconciliation, now 30 rows). Unblocked and
  live: the reconciliation · P6 · R-1's missing_history semantics.

## P — PIPELINE STREAM

- **P1 · network-history curve (D_z/D_k) — OPERATIONAL.** 5-min sampler →
  Supabase. Known gaps annotated in-ledger (S15 forensics window; event #9
  ~5 buckets). Consumers must not read gaps as anomalies.
- **P2 · dashboard delivered-hashrate — RESOLVED at BL-099.** Nameplate
  14.2 stays BY DESIGN (BL-016(b) capture-efficiency KPI, now labeled);
  a new constant would violate BL-028; Netlify-remote rules out LAN
  Prometheus reads. Delivered figure ships when the reporter posts the
  gauge to Supabase — r6 candidate (with the difficulty feed). Dashboard
  sitting otherwise CLOSED 5/5 w/ 28.4-vs-28.9 cross-check; future card
  noted: "Expected (network)" beside pace = drought instrument in UI.
- **P5 · block-detail expansion — OPEN, gated.** Gate: confirm
  post-e49ce61 layout on the running node. Mergeset-persist rider folded
  in.
- **P6 · treasury push — OPEN, unblocked by H8.** Two paths: walletd
  polling (BL-052 zeroed-object discipline + v1.0.7 `missing_history`
  refusal handling) OR the wallet-app view-key/watch-only route (v1.0.29).
  Design decision before build.
- **P7 · chain-block status — OPEN.** First step is ONE question: is
  chain/blue status persisted or computed-on-demand? Answer decides the
  red-rate observability design.
- **P8 · restart logging — OPEN.** Step 1: pin the actual exporter
  start-time metric name on this build (obvious candidate returned NO
  DATA). Then a recording rule retires the dashboard's manual field.

## R — RULES SITTING (one deliberate sitting, batched)

- **R-1** walletd distress alert: fire on poll-failures INCREASING or
  age-while-pending — never naive age; `missing_history`-aware post-H8.
- **R-2** ZkasLegDegraded (BL-032's original ask; collection exists).
- **R-3** H6's swap rule (above).
- **R-4** Firewall rationalization: eight program-scoped Any/Any allows
  (kaspad.exe/kaspad/zkas-node) undermine port scoping — profile-aware
  audit, explicit rules made authoritative (KRON-HARDENING §6.8).
- **R-5** RcReporterDown fire drill — never run.
- **R-6** Button r3: add walletd version pin (v1.0.5→1.0.8 passed the
  liveness-only check silently).

## T — TRIGGER-DRIVEN (no action until the trigger)

- **T-1** Pi witness node (DELIVERED→build): NUT for all 3 UPSs → drill
  (3 pulls, 3 witnessed alerts) → charter items one at a time (off-host
  deadman leg, rig-canary exporter, WAN probe, backup landing).
  KRON-HARDENING §6.5.3 is the spec.
- **T-2** Third 1500VA (arrival→physical window): KS7-per-unit end-state,
  w8m to battery, canary role retires, gateway cable drop rides.
- **T-3** ERRSTREAM soak: next console flash → one Select-String names it.
  (Two live catches banked: TG 400 UTF-8 at BL-099; walletd refusals at
  BL-087 — the instrument earns its keep.)
- **T-6** Reporter r5 first-card witness: next block → card ~T+5s,
  self-edits at BEAT2. Open until seen; then TG cards are DONE.
- **T-4** Brick experiment (running since 09-02 14:12): falsifier =
  another 41/6008 + zero PowerPanel rows → convicts barrel/board.
- **T-5** Upstream watch: firecash seed-side tip-drop note on #6 (ours
  already confirmed).

## DOCS SWEEP (batch)

NODE-CONTRACT-v1.0.8 (new, post-H8) · NODE-CUTOVER r2 (fix -AsByteStream,
BL-084(4)) · STARTUP-ORDER r2 (/api/wallet/warm; service names post-H2) ·
WINDOWS-EXPORTER runbook (conversation-only; two fixed defects undocketed) ·
FLEET-DEPLOY r2 · BRIDGE-SPEC §2 · covenant/silverscript note (C1–C3
intersections; #218 numbers gate C3) · grafana.exe record-or-retire ·
wallet-app release check → session-open habit · fork rebase · cold-storage
sweep (case strengthened twice this week).

## UPSTREAM REMAINDER

Finality-ladder docs PR (params.rs F-04/F-05 cites) · surplus/third-party
miner docs note (txids to assemble) · anchor-semantics note (#6 Q3,
pre-offered; cite BL-086's sink/virtualParentHashes discriminator).

## v2.0.1.6 (OPEN as of D2)

`tip_hashes.first()→None` both legs (maintainer-endorsed) · mimalloc
purge_decommits A/B (evidence: 14.8 GB private vs 3.9 resident) · A6
job-delivery latency · A1-hygiene bounded retention · reporter r4
candidate: feed webhook's `difficulty` at BEAT1.

## CLOSED THIS REVISION

D1 · D2 · H7 · H8-execution (acceptance items remain under H8 above) ·
A3 (stranded tip: discovered→filed #6→fixed 9a464d51→v1.0.8→healed on
Kron 09-03, ~66h) · UPS-rebalance rider (executed by hand 09-02, map in
KRON-HARDENING §6.5).
