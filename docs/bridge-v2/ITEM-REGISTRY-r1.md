# ITEM REGISTRY — the operation's project-management list
### Content r1 · created 2026-09-03 · Successor to the registry role of SCOPE-v2.0.1.5 (retired at D2)

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

- **H2 · service-migration window — OPEN, front of queue.** The stack's
  seven manual processes → Windows services / run-whether-logged-on.
  Measured argument (event #9): host recovered in 34s, PRODUCTION in 24
  min, only because the operator came home. Riders: (1) all STARTUP-ORDER
  processes as services; (2) deadman + sampler principals fixed (boot
  trigger, non-interactive — kills the §6.7 blind window); (3) retires WT
  tabs/tint/dormant -WindowStyle flags. Owner: next multi-hour Kron window.
- **H6 · memory alerting — RESHAPED to rules-only.** No config change;
  the rule is `windows_memory_swap_pages_written_total` sustained climb.
  RAM% rules are wrong by construction (79% = configured equilibrium,
  BL-081). Executes at the RULES SITTING.
- **H7 · gRPC scoping — CLOSED both legs.** zkas 16810 (08-31) + kaspad
  16110 (found executed 09-02, rule 'Kaspa gRPC MacBook only') both
  MacBook-only. Successor concern: see Rules Sitting item R-4.
- **H8 · v1.0.8 cutover — EXECUTED overnight 09-03** (node v1.0.6→1.0.8 +
  walletd v1.0.5→1.0.8; parallel session, its NODE-CUTOVER-v1_0_8-r1 on
  the mount). Acceptance: stranded-tip drop CONFIRMED (daa 3,377,490,
  ~13h post-cutover, #6 healed end-to-end); still watching: beat2 p50
  ~200s, poll-failures flatline, give-ups 0 over the first days.
  UNBLOCKED BY H8, now live: 28-row give-up reconciliation (money;
  era-correct subsidy values — a step to 42.70175169 occurred 09-02),
  P6, walletd alert semantics (R-1).

## P — PIPELINE STREAM

- **P1 · network-history curve (D_z/D_k) — OPERATIONAL.** 5-min sampler →
  Supabase. Known gaps annotated in-ledger (S15 forensics window; event #9
  ~5 buckets). Consumers must not read gaps as anomalies.
- **P2 · dashboard hashrate pointer — OPEN, quick win.** "Your Hashrate"
  14.2 constant → `rc:fleet_hashrate_delivered_hps` (measured 15.23 TH/s);
  un-flatters Luck ~7%. NEVER source from the zkas estimator gauge until
  v2.0.1.6's anchor fix ships.
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
