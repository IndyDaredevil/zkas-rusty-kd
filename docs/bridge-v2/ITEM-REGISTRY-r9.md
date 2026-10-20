# ITEM REGISTRY — the operation's project-management list
### Content r9 · created 2026-09-03, revised 2026-10-19 (r9: catches up BL-122→BL-136 — kaspad v2.1.0 and bridge v2.0.1.6 CLOSED, the security-fork program F-1 opened and its un-ledgered window named, the v1.0.11 audit stream A-5/A-6 opened, W-3 opened, H9/R-4 reshaped; base = r8 of 2026-09-11, repo `docs/bridge-v2/ITEM-REGISTRY-r8.md` @ 6b644ad, 212 ln c83d0c87…; the mount still listed r7 — BL-122 — so r9 replaces BOTH) · Successor to the registry role of SCOPE-v2.0.1.5 (retired at D2)

**Why this doc exists:** SCOPE carried the item definitions and retired with
its version (D2 pass, 09-03) — leaving codes referenced everywhere and
defined nowhere living. Definitions live HERE now, in a doc that does not
retire when a version ships. Maintenance rule: update when an item opens,
changes shape, or closes; closed items stay listed one revision, then move
to the CLOSED section; codes are never reused. Ledger entries remain the
evidence; this is the map. **r9 note:** the ledger stopped citing registry
codes after BL-107 (grep over BL-108→136: 0 hits for `[DHPRTW]-NN`); every
status below was read from the entry bodies, and each line names its BL.

---

## GAP — the un-ledgered window (owed first, before any new item)

Between BL-129 (10-10) and BL-130 (10-19) the ledger has NO entry, yet the
mount carries the runbooks for three production events in that window:
NODE-CUTOVER-v1.0.11-r2 (10-11; node + walletd v1.0.8 → zkas-v1.0.11, the
security-fork release — Kron is on it per the operator, I-48, BL-131),
FLEET-DEPLOY-v2.0.1.7-r1 (10-12; bridge v2.0.1.6 → v2.0.1.7, the bound
ZKM1 form + push listener; `.bak-v2016` HELD until the first accepted
post-fork zKAS block), and the fork itself (DAA 6,983,500, upstream est.
10-15; coinbase-tag ban 6,897,100). Whether each runbook's record-at-close
(CUTOVER §16, DEPLOY §5c) was executed, the self-check line verbatim, the
first accepted post-fork block, check-kron r5/r6, `.bak-v2016` status —
none of it is on a ledger rail (grep: `6,983,500` appears twice, both in
BL-130's code reading; `.bak-v2016`, `check-kron r5|r6`, `STARTUP-ORDER r7`:
0 hits). **Owed: one gap append (BL-137 candidate) written from Kron reads
and the operator's statements — not from recollection.** Until it lands,
every "post-fork" line in this registry rests on the mount + operator only.

## D — DATED DECISION GATES

- (none open; D1/D2 in CLOSED — PRIOR REVISIONS)

## F — SECURITY FORK PROGRAM (P0 at BL-129; sub-items from KICKOFF-SECURITY-FORK-8700500-r1 §2)

- **F-1 · fork readiness — EXECUTED per mount, UN-LEDGERED (see GAP).**
  Scores superseded at the tag: fork + dev-fee end = DAA **6,983,500**,
  tag ban 6,897,100 (NODE-CUTOVER-v1.0.11-r2 header; the kickoff's
  8,700,500 / 8,614,100 were pre-tag estimates). Sub-items: D2 fork rebase
  → `merged-fork-v1.0.11` (exists: wt-br1 @ 6b644ad, BL-136); D3 bridge
  bound-aux port → v2.0.1.7 (tag 99611dc, asset 97f61946…, DEPLOY r1 §0);
  D4 NODE-CONTRACT v1.0.11 — **OWED** (no doc on mount or repo); D5
  NODE-CUTOVER-v1.0.11 — r2 on mount, executed per operator; D6
  reporter/dashboard accrual shape → P12 below. Live evidence that the
  bound form passed the node's verifier on mainnet = the first accepted
  post-fork zKAS block (DEPLOY §5b) — **not on any rail here.**

## H — HOST STREAM

- **H2 · auto-start + runtime-visibility program — OPEN, front of queue,
  RESHAPED at BL-097.** Staged (a) visibility contract per process, (b)
  boot-start v2 (inmyh principal, network pre-gate, own deadman), (c)
  supervision as a separate decision. Measured argument unchanged: event
  #9 = host 34s, production 24 min. Added 09-30 (BL-128): first PLANNED
  reboot = Boot Law rehearsal, logon 04:22:21 → Button 8/8 04:36 (~14
  min); only the reporter came up (logon trigger) — six daemons
  hand-launched, which is the item. Button precondition stands: ELEVATED
  prompt (walletd high-integrity). Live checker = check-kron **r4**
  (BL-128: kaspad + bridge pinned by path+sha; null-path guard); r5/r6
  are referenced by FLEET-DEPLOY-v2.0.1.7 — in the GAP.
- **H6 · memory alerting — RESHAPED to rules-only** (swap-pages climb;
  RAM% rules wrong by construction, BL-081). Executes at the RULES SITTING.
- **H9 · reporter r9 — start gauge + DPAPI wallet token — OPEN.** Live
  reporter still r8 (BL-128 read: E01206AE…; banner prints "r6", label vs
  content). Riders unchanged (BL-119 start gauge; BL-116 plaintext token at
  line 75). **Reshaped by BL-133 W1/W2:** the token is the WHOLE authority
  (`reveal`, `send`, `sign`, `/history` all on one bearer; no read-only
  scope exists), and `--no-custodial` also disables spending — unusable for
  the treasury. So DPAPI is the only local mitigation; the scoped-token ask
  is upstream (W-3).
- **H10 · kaspad v2.1.0 cutover — CLOSED at BL-126** (09-25, versioned
  launcher `C:\rusty-kaspa-v210\kaspad.exe`, banner 01:05:40 EDT;
  KASPAD-EVAL-v2.1.0-r1; STARTUP-ORDER r2→r4). Rollback operands
  `.bak-v2015` (F1484FB5…) + v2.0.1 exe (084C2B92…) HELD — see H11.
- **H11 · kaspad restart investigation — PARKED by operator at BL-128.**
  kaspad start moved 7× in 7 d (09-23→09-30), causes unrecorded ("several
  for unknown reasons"); the 09-30 reboot is an 8th WITH a cause. Gate
  instrument corrected: `delta(kron:process_start_seconds[24h]) > 60` per
  process — `changes()` over the floored metric counts NetTime ticks (read
  58, wrong). Rollback operands stay until this closes.
- **H12 · Windows Update manual — CLOSED at BL-128** (NoAutoUpdate=1 ·
  AUOptions=2 · NoAutoRebootWithLoggedOnUsers=1, read back; Defender
  definitions unaffected). Staged KBs took the planned 09-30 restart.
- **H13 · node v1.0.11 cutover — EXECUTED per operator, UN-LEDGERED (GAP).**
  Kron runs zkas-v1.0.11 = 01ac1865 (BL-131 I-48; CI build, rustc 1.99.0,
  BL-135 B1–B3). Launcher `run-zkas-node.cmd`, console, NO loop — a kill is
  an outage until a hand relaunch; `KronProcessRestarted` is an alert, not
  a restarter (BL-135 I-50 retraction). STARTUP-ORDER r7 owed (§3 `/api/
  status` path fix, v1.0.11 identities, supply-check sha owed since r6).

## P — PIPELINE STREAM

- **P1 · network-history curve — OPERATIONAL, sampler r2** (BL-114).
- **P2 · dashboard delivered-hashrate — RESOLVED at BL-099**; Supabase copy
  queued behind H9.
- **P5 · block-detail expansion — OPEN, gated** (post-e49ce61 layout).
- **P6 · treasury push — OPEN.** Design decision before build. Narrowed by
  BL-133 W1: the walletd-polling path means the treasury token on a
  poller; the view-key route avoids it. (BL-135 I-24 class: a public NODE
  is not a public SERVICE — the viewing key stays local either way.)
- **P12 · accrual-shape — OPEN (kickoff D6; NODE-CUTOVER-v1.0.11-r2 §15).**
  From the fork, one coinbase note may cover several blocks (MinerAccrual,
  CH2 §3m); the reporter's "latest walletd coinbase" amount and the
  dashboard's per-block amount stop mapping one note to one block. Needs a
  mailbox row to mining-dash + reporter rule. Not observed live (GAP).

## R — RULES SITTING (one deliberate sitting, batched)

- **R-1** walletd distress alert (poll-failures increasing / age-while-
  pending; `missing_history`-aware).
- **R-2** ZkasLegDegraded.
- **R-3** H6's swap rule.
- **R-4 · firewall — NARROWED at BL-126, READ at BL-135.** kaspad v2.1.0
  got a pre-minted program+port rule (BL-126: "class R-4 retires" for that
  leg). Live read 10-19 (BL-135): inbound 16811 open, 16810 scoped to
  192.168.1.173 (MacBook); inbound=6 / outbound=14 peers. Remaining: the
  Any/Any program allows audit (KRON-HARDENING §6.8). Optional mitigation
  from A-6 — drop the 16811 router forward until v1.0.12 — sized by the
  operator as not worth it at 15–25 blocks/day.
- **R-5** RcReporterDown fire drill — never run.
- **R-7** Reporter: log Telegram's error text on the edit path (BL-105).

## T — TRIGGER-DRIVEN

- **T-1a** Pi 5 (on hand): off-host deadman, NUT master, pollers, second
  supply-check witness. **T-1b** Kron-2 re-order on correct stock.
- **T-2** Third 1500VA. **T-3** ERRSTREAM soak (two catches banked).
- **T-4** Brick experiment — no read since r8 (running since 09-02).

## A — AUDITOR / SUPPLY / SECURITY REVIEW

- **A-1 · shielded archive — PARKED** (BL-111). **A-2 · supply-check r8 —
  LIVE hourly** (BL-108; sha owed to r7 settled at BL-128 from task
  Arguments). **A-3 · audit-epoch proposal — HELD.** **A-4 · stage-table
  rule** (BL-120).
- **A-5 · informal security audit of zkas-v1.0.11 — OPEN, ~62% by the
  stated rubric (BL-136).** Deliverables on mount, all sha-pinned:
  SCOPE-r7 · CH2-r16 (350 ln a053f868…; §3a–§3q; Q1–Q10 with Q4 answered
  from the sighash read and Q10 closed from the tree, BL-135) ·
  DEPS-AUDIT-r1 (13 advisories; h2 the one on a Kron listener, fixed on
  main) · shielded.rs read in full · the tag's test suite green on every
  node package (6 failing targets triaged, BL-135) · CH2-4 measured
  (BL-132) · CH2-3 characterization test green at the tag (BL-136, patch
  4282a37e…, MacBook wt-v1011). **Open:** (1) SEND the message to
  Ronnie/Leo/Odie (variant 2 — "the audit report the commits cite" — plus
  the simpa-dead note and the CH2-3 acceptance-data question); (2) the
  ours-only bridge remainder: notification_hub.rs (newest code on Kron),
  merged_obs.rs, prom.rs handler, 8 non-indexing unwraps; (3) from-genesis
  sync for Q1 on v1.0.12 when tagged; (4) laws r3 (Appendix B stale at
  I-25; the I-26..I-36 id collision, BL-131). Worktrees: MacBook
  `~/audit-2026-10-19/{wt-v1011,wt-br1}` with CH2-4, CH2-3, BR-1 applied,
  nothing committed.
- **A-6 · drift zkas-v1.0.11 → main — OPEN (ZKAS-DRIFT-r3, BL-134/135).**
  main is 82 commits (10 merges) past the tag: a 10-16/17 hardening sweep
  closing ≥9 remote-kill classes K1–K9 (all `process::exit(1)` via the
  panic hook), N7, two RUSTSEC advisories — none released. Kron exposure
  measured, not assumed: inbound=6, K1/K2/K7 live; a kill = the zKAS leg
  down until a hand relaunch, a few blocks at 15–25/day. The ask is a
  v1.0.12 tag; the sweep's commit messages cite a third-party audit whose
  report we have not seen (n=0 on its scope).

## B — BRIDGE (ours: merged-fork-v1.0.11 line)

- **v2.0.1.6 — CLOSED at BL-124/125** (deployed 09-24, B1 path traversal
  confirmed by loopback probe then fixed, B1–B5 shipped; branch line
  consolidated to `merged-v2.0.1.6`, decision A).
- **v2.0.1.7 — DEPLOYED per mount 10-12/13, UN-LEDGERED (GAP).** Bound
  ZKM1 form per template DAA + zKAS push listener; `.bak-v2016` held until
  the first accepted post-fork block.
- **v2.0.1.8 — OPEN queue:** BR-1 r2 patch (BL-136, cd3f40c9…, green on
  MacBook; one-shot claim keyed per chain hash, not per template) · the
  merged_obs test-race fix parked on `merged-fix-double-race` f39d7f0
  (DEPLOY-v2.0.1.7 §0) · carried from r8 unless shipped (unread since):
  `tip_hashes.first()→None` both legs · mimalloc purge_decommits A/B · A6
  job-delivery latency · A1-hygiene retention · webhook `difficulty` at
  BEAT1 (behind H9).

## W — WALLET APP + WALLETD

- **W-1 · zkas-wallet#3 — OPEN** (filed 09-06; BL-116 added a reproduction
  log line; BL-129 10-10: first run hit AddrInUse 16810 from the embedded
  node). Maintainer reply: unread since r8.
- **W-2 · release cadence** — batch-by-default; last read 1.0.32.
- **W-3 · walletd authority model — OPEN, upstream ask (BL-133 W1–W6).**
  One bearer token = reveal + send + sign + read; no scoped read-only
  token; `--no-custodial` disables spending. Kron's reporter polls
  `/history` with the treasury token (BL-116, mechanism confirmed from
  source). Ask: a read-only token or `reveal` behind the wallet secret.
  Local half = H9.

## DOCS SWEEP (batch)

OWED NOW: ledger GAP append · NODE-CONTRACT v1.0.11 (F-1 D4) ·
STARTUP-ORDER r7 · laws r3 (A-5 item 4) · check-kron r5/r6 ledgered.
Carried: NODE-CONTRACT-v1.0.8 · NODE-CUTOVER r2 fixes · WINDOWS-EXPORTER
runbook · FLEET-DEPLOY r2 · BRIDGE-SPEC §2 · covenant/silverscript note ·
grafana.exe record-or-retire · wallet-app release check · cold-storage
sweep.

## UPSTREAM

#8 LANDED 737521e · zakura-core/common#362 MERGED b4f90dd · A2 landed as
9b1c49a with zakura-orchard 2.2.0 (BL-129 lineage) · **#18** (runtime
circuit-identity banner) OPENED 10-10 — status unread since · v1.0.11 =
01ac1865 tagged 10-11 · main's hardening sweep unreleased (A-6) · asks
queued in A-5(1): the audit report, a v1.0.12 tag, the CH2-3 acceptance-
data question, scoped walletd token (W-3). Docs remainder carried
(finality-ladder PR · surplus-miner note · anchor-semantics note).

## CLOSED THIS REVISION (r9)

H10 kaspad v2.1.0 cutover (BL-126) · H12 Windows Update manual (BL-128) ·
bridge v2.0.1.6 + B1 (BL-124/125) · check-kron r4 live (BL-128) ·
STARTUP-ORDER r6 (BL-128, the second-"r2" merge) · CH2-4 measured (BL-132)
· CH2 Q10 closed, Q4 answered (BL-135) · BL-116's "n=0 unverified" closed
by W1 (BL-133) · the "watchdog restart" claim retracted (BL-135 I-50) ·
P8/P11/T-7/R-6 rotate out after their one listed revision (r8).

## CLOSED — PRIOR REVISIONS

r8: P8 (BL-119) · P11 (BL-113/117) · T-7 (BL-118, reclassified: parser
gap; since 08-30 = 90.2%, n=542) · R-6 (BL-106).
r7: D1 · D2 · H7 · H8 (BL-093/098) · A3 stranded tip (#6 → 9a464d51 →
v1.0.8) · UPS-rebalance rider · P7 (BL-107) · P9/P10 (BL-106) · T-5 ·
T-6.
