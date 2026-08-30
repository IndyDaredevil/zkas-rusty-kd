# SESSION-STATE — 2026-08-30 (S14 close: P1 shipped, PowerPanel exercised)
### Supersedes SESSION-STATE-2026-08-29.md (115 ln, 6e895a4e…e66e), which is
### archived in place as archive-SESSION-STATE-2026-08-29.md.
### Deep content lives in the tier docs (ledger @ BL-062, SCOPE r5,
### KRON-HARDENING r2, FLEET-DEPLOY r1, NODE-CONTRACT v1.0.6,
### NODE-CUTOVER r1). Cite, don't re-derive.

## OPEN THE NEXT SESSION WITH
Docs tip: see KEY VALUES. Code tip 1b63698 = running exe, unchanged.
`merged-v2.0.1.4` is RETIRED READ-ONLY — do not commit to it.
Then answer: (1) is `NetworkHistorySampler` still firing clean, and does
`network_history` show ~288 rows/day? (2) D1 — `.bak-v2014` retired? (3) is
`est_hashrate_z` still bimodal (see WATCH)? (4) mount synced to this
session's three artifacts?

## LIVE STATE
- **Production bridge**: v2.0.1.5, unchanged since the 08-28 deploy
  (F1484FB5…A3F0 @ 1b63698). No restarts this session.
- **P1 — SHIPPED 2026-08-29, capturing.** Supabase `network_history` +
  `network-history-webhook`, all four acceptance tests plus the fail-closed
  check verified against the DEPLOYED endpoint (not Bolt's report). Kron:
  `C:\zkas\set-nh-secret-r1.ps1` (2CCEFCFB…F0207) wrote
  `C:\zkas\nh-secret.dpapi` (15 chars, cross-checked against an independent
  count); `C:\zkas\network-history-sampler-r1.ps1` (E4869402…30DA68) is
  ONE-SHOT, driven by scheduled task `NetworkHistorySampler` at 5-min
  repetition with both battery flags disarmed. Verified firing at 00:00:03
  and 00:05:03 into buckets no hand-run produced; LastTaskResult 0,
  NumberOfMissedRuns 0. **The D_z/D_k curve is no longer being lost.**
- **UPS / PowerPanel — INSTALLED AND EXERCISED 08-30.** BL-044's assumed
  channel is REFUTED: PowerPanel does not write to the Windows event log at
  all, so Event ID 105 will never appear here. Real store is the PowerPanel
  UI's Event Logs view, backed by `C:\Program Files (x86)\CyberPower
  PowerPanel Personal\assets\PPPE_Db.db` (SQLite; snapshot 0586A64C…F56B3 at
  `C:\zkas\tmp\`). A 12-second plug pull produced four second-precision rows.
  **Every future host-event sweep must include this store.**
  Runtime measured 11 min with 2 KS0 Ultras still on the 1000VA at 97%
  charge; post-rebalance forecast ~33–40 min.
- **zkas-node** v1.0.6, **walletd** still v1.0.5, **alerting** — all unchanged
  from the 08-29 doc; see it in the archive for detail.
- **Docs rail**: ledger appended and committed at `4734675`; mount
  delete-then-add executed and VERIFIED by direct read (both files timestamped
  04:41, duplicate check = 2). SCOPE r4 had been stranded on the mount since
  08-29 — the delete-then-add named in `a2d8650`'s message was never executed —
  and was closed in the same pass.

## GATES PENDING
1. **D1 — `.bak-v2014` retirement**: eligible since 08-29 ~01:09 EDT.
   `-WhatIf` preview first. STILL OPEN, carried from the 08-29 doc.
2. **D2 — A1′ seven-day acceptance**: `max_over_time(
   scrape_duration_seconds[7d]) < 5s` on/after **2026-09-04 ~01:09 EDT**.
   Positive control now EXISTS (see KEY VALUES): pre-fix was 56 of 10,080
   scrapes over 5s, bimodal with zero samples between 5s and 10s.
3. **A4 residual**: one eyeball of the HourlyMergedReport card vs live `rc:`
   values. Carried.
4. **Pace gate — CLOSED MEASURED**, no longer pending. See BL-062.

## WATCH
- **`est_hashrate_z` bimodal ~18×.** Five samples inside 28 minutes read
  2.883e16, 3.014e16, 3.031e16, 3.041e16, then **1.677e15**, with `d_z` flat
  and `hk` steady. Not physical. Oscillates between two regimes rather than
  drifting (1.677e15 matches a 08-27 reading). Sampler reports faithfully;
  the GAUGE is suspect. A3's file, but A3 was scoped on a few-percent drift —
  this is a different phenomenon under the same label. Ratio columns
  unaffected (difficulty-derived). Read the distribution after a few hours of
  samples rather than from five points.
- **`zkas_blocks` anon CRUD policies** — `anon_insert`, `anon_update`,
  `anon_delete` all exist for the `anon` role, which is a public key shipped
  in the dashboard bundle. Operator judgment: KDSM is personal, not a product,
  so this is hygiene not emergency. Left in place deliberately. The residual
  is an accidental unqualified write (own code, a scanner) against the block
  accounting table, with no second observer to notice.

## QUEUE (unordered unless noted)
- **H2 window** — largest open item. Now carries FIVE riders including the
  **UPS load rebalance** (SCOPE §4 H2(e), KRON-HARDENING §6.5). Stake ~34 min
  asleep vs ~5 min awake.
- **H8 walletd v1.0.6 cutover** — gates P6.
- **H6 windows_exporter + ZkasLegDegraded** — still the only uninstrumented
  hypothesis for the original BL-032 wedge.
- **P1 follow-ons**: Prometheus staleness alert on the sampler (it currently
  fails to a log line nobody reads); the dashboard chart (brief non-goal).
- **P6 treasury balance push** — NEW, gated on H8.
- **P7 chain-block rate analysis** — NEW. First step is one query: is chain
  status PERSISTED or computed on demand? That decides whether the before/
  after is possible and whether the P5 rider becomes urgent.
- **Reporter `Serve-Metrics` loop-coupling** (BL-060) — cheap fix, serve
  metrics before beats or between POSTs.
- **Upstream batch to firecash** — unchanged, unsent.
- **Runbook r2s**: NODE-CUTOVER, FLEET-DEPLOY. Unchanged.
- **BRIDGE-SPEC §2** single-process clarification — still not written.
- **Fork rebase**, **cold storage sweep**, **archival stewardship** —
  unchanged.
- **v2.0.1.6 seeds**: A6 job-delivery latency; A1-hygiene bounded retention.

## KNOWN DEFECT ON THE RAIL
Commit `4734675`'s message contains the literal token `<STEP5-SHA>` where the
incoming ledger sha belonged — a law-3 violation (an angle-bracket placeholder
shipped inline in a runnable command instead of the value being retrieved
first). Not amended: the commit is pushed and this repo does not force-push.
**Authoritative value: ledger 1425 ln, sha d4c3807e…348e17.**

## KEY VALUES
- Ledger **BL-062 · 1425 ln · d4c3807eea8dc46f6a037d18b5fd70d3e6ad1f569f43aed7f2a710bba5348e17 · commit 4734675**
- SCOPE **r5 · 375 ln · 864f9956…230199**
- KRON-HARDENING **r2 · 287 ln · 62b78420…1a03e8**
- SESSION-STATE-2026-08-29 · 115 ln · 6e895a4e…e66e (archived by this doc)
- P1 artifacts: brief 86d2b546…a210 · secret script 2CCEFCFB…F0207 ·
  sampler E4869402…30DA68
- Perishable captures, `~/zkas-lab/perishable-2026-08-20_26/` (08-20 00:00 →
  08-27 00:00 EDT, step 60s, 3 targets):
  `scrape_duration_seconds` eb76a51b…c0bf0 · `scrape_samples_scraped`
  9065bc26…15b3df · `up` 7fab0b01…e5be3c.
  Pre-fix bridge max **55.005s @ 08-26 01:31**, 56/10,080 over 5s, **bimodal
  with zero samples between 5s and 10s**.
- Prometheus retention MEASURED: `retention.time 15d`, `retention.size 0B`.
- D_z/D_k ratio range over 225 hours: **0.62–1.106** (BL-028 unpinnability,
  louder). Pace baseline mean 53.6/day, σ ≈ 7.3.
- PowerPanel store: `PPPE_Db.db`, snapshot 0586A64C…F56B3.
- Every MacBook git op starts `cd ~/zkas/zkas-rusty-kd &&`; the clone rests
  on `merged-v2.0.1.5`.
(END)


---

## S15 APPEND — 2026-08-30 (v1.0.6 cutover thread: adoption, impostor night, drought arc, 14:22 reset)
Appended per merge protocol; supersedes nothing above. Deep content: ledger
BL-063–068, NODE-CONTRACT v1.0.6, NODE-CUTOVER r1, STARTUP-ORDER r1.

### OPEN THE NEXT SESSION WITH
0. BL-068 hardware forensics: PowerPanel PPPE_Db.db around 14:22 (power
   event vs silence) · Reliability Monitor · decide thermal/rail
   instrumentation (HWiNFO logging) before the next uncaptured reset.
   Pattern review: the wordless-events pile may be TWO mechanisms
   (instant-reset hardware class vs wedge/memory class).
1. Walletd degradation verdict: latest BEAT2 `dt` readings + one
   `/api/wallet/balance` gap read ×2 (scanned must advance). If stalling
   persists → scan-gap investigation (impostor wrote hours of records;
   v1.0.6 reads mixed archives, but walletd v1.0.5 paging through the
   boundary is the suspect surface). BL-067(3) holds the first datapoint.
2. Session-kill + launcher-truncation forensics: what killed Session 1
   ~08-29/30 (Event Log 6008/41 vs 1074/6006 on that window; if
   uncommanded, it joins the BL-040 series) · run-zkas-node.cmd
   empty-file timestamp already captured.
3. `--verify-shielded-history` run (NODE-CUTOVER §13a) — now doubly
   motivated: proves the archive across BOTH the v1.0.6 boundary and the
   impostor's write era.

### LIVE STATE (deltas from S14 block above)
- zkas-node v1.0.6 IN PRODUCTION via launcher: exe C:\zkas\node-v106\
  zkas-node.exe, sha 1B49D1FA…DC97D2, --shielded-history=on. NO banner
  exists; hash is the version. Rollback set: C:\zkas\backup\
  node-data-pre-v106 (32.817 GB, 0 failed) + wallets-pre-v106 +
  zkas-node-758k.toml (95814C89…AD75A) + v105 exe 0E2B5B43…7A24AF.
- Firewall: gRPC 16810 scoped to MacBook 192.168.1.173 only (old rule
  {e38b9023…} disabled, kept). H7 CLOSED (already in SCOPE r5).
- Legacy exes QUARANTINED: C:\zkas\archive\legacy-node-dir\ (impostor
  66D8296D…B4078E, v0.3.1, stray stratum-bridge.exe et al). C:\zkas\node\
  holds configs + walletd launcher ONLY.
- check-kron.ps1 deployed C:\zkas\ (r1 that night, r2 cut this close-out:
  reporter check = :9151 + log-age 90m; launcher-integrity check added).
  One live catch banked (BL-065).
- STARTUP-ORDER-r1.md is now the sole startup reference; personal notes
  retired (the impostor factory).
- NetworkHistorySampler: DISABLED 08-30 ~08:15 during the drought
  experiment, EXONERATED (BL-066) — re-enable is one command and is OWED
  (every hour off = permanent P1 curve gap):
  `Enable-ScheduledTask -TaskName NetworkHistorySampler`.
- Drought escalated same morning per its own 3σ clause; tail-histogram
  hunt localized the deficit to genuine extreme-tail scarcity, bar
  acquitted (BL-066 amended). Per-rig decomposition armed, day-2.
- 14:22:41 INSTANT RESET, uncaptured, hardware-class (BL-068). ~10 min
  gap; STARTUP-ORDER-r1 first live cold start; reporter proved it
  AUTO-STARTS at boot. Give-up trio at provisional amounts: 7da0660e,
  5740a96b, fb952a95 (exact-amount backfill owed).
- Reporter dark window 08:27–11:01 recovered by replay+dedup (5 blocks
  in 6s); reconciliation alert's first live catches (BL-067 amended).

### QUEUE ADDITIONS / CHANGES
- NODE-CONTRACT r2 DEFERRED deliberately: two line-edits (perf-metrics
  "armed, emission path unverified" — zero counter lines at INFO since
  launch; §7 note that check-kron/step-10 style log reads must target
  zkas-mainnet\logs, never the datadir WAL *.log files). Batch with the
  walletd-cutover r3 to avoid double mount churn. The RECORD of both
  corrections is this append + BL-063/065.
- perf-metrics emission-path source read (before wiring any consumer).
- Launcher mints owed: run-kaspad.cmd + monitoring pair (cwd-baking) —
  H2 prerequisites; H2 itself re-argued by the impostor night.
- Close-out grabs STILL OWED (never run): datadir decomposition (or
  zkas-db-usage.exe), fresh RSS baselines (note restarts 08-30 ~03:30
  reset uptime clocks), rollback-set manifest.
- Prometheus rule sketch: cumulative expected-vs-observed solve drift
  (100:1 law) alerting at ~3σ with flat near-misses.
- Sampler S4U principal fix (script read DONE, :3034-only confirmed) +
  the args-discrepancy oddity (BL-067(2)).
- Upstream report batch grew: wedge (class-level) · config-key drop +
  dead externalip · consolidate-as-coinbase · .dmg signature.
- Fork rebase: dry-run merge CLEAN (tree ed9acea4 vs origin 30b0700 +
  zkas-v1.0.6); re-run locally at execution (laws 4/13d).
- Archival stewardship (NEW): Kron holds true-archival depth from genesis
  — closed, non-regrowable set, network-recovery asset (BL-002
  precedent). Implies off-box periodic snapshot + recurring verify runs +
  size-at-height slope (32.8 GB @ ~2.878M chain, 08-30; firecash fresh-
  sync figures NOT comparable — 4 confounds recorded in-thread).

### KEY VALUES (this thread)
- v1.0.6 zip 8B63C491…B2D830 · exe 1B49D1FA…DC97D2 · launch 08-28
  16:46:20, tip-follow ~16:50, zk=ok 0.1s @ 16:55 · downtime ~14 min,
  27,495/0/0 shares, zero bridge restarts.
- Treasury balance read 08-28: 5,783,683,057,813 sompi (57,836.83057813
  ZKAS), 3 keys/3 wallets.
- Drought window 08-30 00:00–08:27: ~4 events vs ~16 expected; near-miss
  stream flat 155–234/hr for 48h; d= 1.62e16→1.64e16.
- STARTUP-ORDER-r1 + check-kron-r2 shas: pinned at this close-out's cut
  (see commit).
(S15 END)
