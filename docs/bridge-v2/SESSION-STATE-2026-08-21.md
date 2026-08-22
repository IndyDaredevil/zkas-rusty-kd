# SESSION STATE — 2026-08-21 (FINAL) — Mining Dashboard rail: backfill -> live pipeline -> monitoring
### Supersedes the mid-session 08-21 snapshot (browser-crash cut). Supplements SESSION-STATE-2026-08-20.md.
### Scope: KDSM dashboard rail end to end. Bridge/node/fleet untouched all session (meta-principle 8 held).

---

## 1. WHAT EXISTS NOW (all deployed, all verified)

**The pipeline:** bridge log -> zkas-reporter.ps1 (ZkasReporter scheduled task,
inmyh+logon) -> Supabase edge function v2 (secret-gated) -> zkas_blocks
(unique-hash upsert) -> realtime -> dashboard. Two-beat protocol per block:
~T+5s hash/worker/found_at + provisional amount; ~T+60s exact amount from
walletd history. Prometheus scrapes the reporter (:9151 loopback) with four
alert rules; Telegram counter path remains the independent second observer.

**Final verification chain (01:58 ET):** reporter metrics served
(blocks_total=45 == DB stratum-bridge count, cross-instrument to the row;
pending=0; post_failures=0 under armed secret); target `up`; four rules
present via /api/v1/rules (the only accepted proof); log shows metrics
listener + clean replay. Bare POST without secret -> 401. Dashboard live.

## 2. HEADLINE RESULTS (chronological through the session)

1. **Backfill:** 653 treasury-era rows imported, ALL with true H_zk keys +
   worker attribution (B-variant: 614 archive-log join, 39 live-snapshot
   join; median dt 0.9s/1.1s; 0 forced matches). Verified four independent
   ways incl. dashboard-derived stats (avg 50.758, lifetime interval 30.6m).
2. **Latent bug #1 found+fixed (Bolt):** the original webhook's
   ON CONFLICT(block_hash) upsert could NEVER have worked — Postgres cannot
   use a partial unique index as arbiter without the predicate, which
   supabase-js can't supply. Bolt's migration replaced it with a full unique
   constraint (zkas_blocks_block_hash_key). Every prior "dedup" claim was
   untested; the reporter's first POST would have 500'd.
3. **Latent bug #2 (belief-level):** bridge file logging was believed
   toggled on 08-06; compiled default is TRUE since June 24 — on for the
   operation's entire life, at C:\Users\inmyh\AppData\Local\
   kaspa-stratum-bridge\logs (Windows uses data_local_dir; a cropped grep
   asserted profile-root and cost two turns — read the WHOLE file).
4. **WS7 architecture final:** in-bridge webhook DEAD; Alertmanager
   piggyback REJECTED (no identity in counter deltas; T+37-127s vs sidecar
   ~T+5s); sidecar = accounting rail, alert path = humans + watchdog.
   Two data tiers = two beats on one key, not two transports.
5. **Bolt engagement:** constraints-first brief worked. Compliant on webhook
   (byte-identical deploy), realtime (INSERT+UPDATE + REPLICA IDENTITY FULL
   migration), pending display. Deviations: the constraint fix (justified,
   see #2) and a git-history flatten to one "Start repository" commit
   (content lossless; add "never reinit history" to future briefs). Bolt
   cannot set edge secrets — operator did via dashboard. Secret value shared
   with Bolt by explicit proportionality decision (anon key ships in the
   public bundle anyway; threat model is drive-by POSTs, not Bolt).
6. **Provenance reconciliation:** 659 rows = backfill-csv 614 +
   stratum-bridge 45 (39 replay flips + 4 post-snapshot + live blocks);
   sum decomposition exact to the sompi. Every row accounted.
7. **Monitoring deployed:** prometheus.yml + alert_rules.yml regenerated as
   whole files FROM THE DEPLOYED COPIES (repo mirror proven stale — still
   pre-08-18: birth clauses present, 12s timeout). New group zkas_reporter:
   RcReporterDown (up==0, loop-wedge included by served-in-loop design),
   RcReporterStarved (bridge counter moved unless reporter counter moved;
   staleness deliberately fires it, increase-inside-sum, no birth clause),
   RcReporterPostFailures (401/outage early warning), RcReporterPendingStuck
   (walletd). No Alertmanager changes needed (sustained conditions, no
   BL-026 coupling).

## 3. OPERATIONAL FACTS BANKED

- **Reporter update procedure:** stop task -> overwrite C:\zkas\
  zkas-reporter.ps1 -> start task -> verify RUNNING artifact via reporter.log
  ("metrics listener" line) + :9151/metrics. State file schema stable;
  replay makes any botched update cost a restart, not data. One-time
  urlacl: netsh http add urlacl url=http://127.0.0.1:9151/ user=inmyh (DONE).
- **Restart semantics:** edge fn is serverless (never restarts); reporter
  returns at inmyh logon; replay-on-start makes downtime = latency, not
  loss. KNOWN GAP: replay scans newest log only — reporter downtime
  spanning a BRIDGE restart can miss blocks logged in the previous file
  (needs reporter-down + bridge-restart + block in window). Remedies: 2-file
  replay patch (5 lines, on offer) or post-messy-day count reconciliation.
- **Secret:** value staged in C:\zkas\webhook-secret.txt AND Supabase env;
  reporter reads file ONLY at startup -> any secret change requires a task
  restart (RcReporterPostFailures catches the forgotten case as 401s).
- **File-shuttle discipline (three scalps in one day):** zero-filled zip
  (right size, 100% NUL — size is the check that lies), wrong-machine
  downloads, stale same-name Downloads collisions (8/10 configs nearly
  deployed over the 08-18 rework — the dir-with-dates gate caught it).
  Checksum/date gates stay MANDATORY for anything bigger than a paste.

## 4. OPEN ITEMS (priority order)

1. **Fire drill:** stop ZkasReporter, ~2.5m -> RcReporterDown Telegram card,
   start, resolve. Converts rules installed->witnessed. Five minutes.
2. **RcReporterStarved live test = passive:** next block appears on
   dashboard with no alert. (Same honest-labeling as BL-026's first test.)
3. **287.58 row (still excluded from 659):** answer whether a manual
   /api/wallet/consolidate ran ~10:14 ET Wed 08-20; else look up txid
   01bf9ea94eb6584e...b3b17256 on explorer.zkas.info (URL found in own rule
   comments). Coinbase-paying-6-blues -> +6 rows (665/33,720.65); else
   excluded forever + firecash bug report (history mislabels consolidate
   outputs as kind=coinbase).
4. **Repo hygiene:** sync-monitoring run (mirror now TWO revisions stale);
   commit zkas-reporter.ps1 + runbooks + reconciliation report to
   docs/bridge-v2/ (suggest docs/bridge-v2/reporter/); Bolt deleted the two
   stray CSVs from supabase/migrations (done).
5. **History recovery session (~110 blocks / ~5,800 zKAS, ~15% lifetime):**
   INPUTS COMPLETE — topology corrected 08-22: web wallet and phone app were
   ONE wallet (same seed, two frontends), so a single address covers the
   entire pre-treasury era:
   zkas:p9ywy4tea4sqeu8ql9gacf4saqag2u82p97heyjesal45nw2nq252r9lnhwnparxqfl6xgcc5csc30q
   (matches the ledger's "p9f9d2d" mystery-wallet shorthand, BL-007).
   Wallet now empty — irrelevant: coinbase mints are permanent public chain
   record; the scan reads history, not balance. WS3-FT minimal cut via
   getShieldedBlocks filtered to this address; acceptance = closes the
   5,340.51 sweep gap + the 3 retained blocks (~161.4, since moved to the
   macOS wallet) to the sompi. 32 RC-era events already hold hashes+workers
   (log_events.json — re-derivable from the log archive if lost).
6. **Log retention rule** for RKStratum_*.log (>100 MB/day, no rotation;
   reporter reads newest only, so old-file deletion is safe — by name,
   BL-031 rails).
7. **full_clear semantics** — unresolved (~91% true, [ZKAS] DOUBLE 566/646
   vs ~25 real): pin against bridge source before the field ships anywhere.
8. **2-file replay patch** — on offer, closes the known gap in §3.
9. **Unattributed deleted test row** (1bd4a66e inserted by gate test,
   already gone when Bolt's delete ran 0 rows): reconciliation query came
   back clean, filed as harmless-unattributed. RLS anon-DELETE hardening
   remains the standing flag behind it.
10. **Standing carries from 08-20 doc** unchanged: node v1.0.6 window,
    zkas-node memory slope, BL-029 8h test, kaspad-death alert,
    RcZkasBlockNotConfirmedBlue name verification, rc-v2-smoke rename, UPS.

## 5. LEDGER CANDIDATES (draft at next ledger session)

- Partial unique index + supabase-js ON CONFLICT = structurally dead upsert;
  "dedup by design" was never exercised until a machine consumer arrived.
  Pair with: file logging on-but-unread for the operation's whole life.
  Same lesson, two instances in one day: A PRODUCER WITHOUT A CONSUMER IS
  UNVERIFIED, however long it has "worked."
- Cropped grep != the artifact (app_dirs.rs cfg(windows) branch) — BL-027's
  fragment corollary.
- File-shuttle failure taxonomy: zero-fill (size preserved), wrong-machine,
  stale-name collision. Gates: hash bracket + date-checked dir listing.
- Prometheus-as-accounting-transport rejected on identity loss; alert rail
  and accounting rail as deliberate independent observers (meta-principle 2
  as architecture).
- walletd history: coinbase retroactive (public mints), transfers not;
  Explorer 0-KB on open files (NTFS lazy size); Copy-Item preserves mtime
  (reconfirmed via the 8/18-dated deployed-config copies).
- Wrong-baseline diff near-miss: compared new dashboard repo against the
  BRIDGE repo tree and nearly reported a 1774-file wipe; caught by
  re-checking the baseline before reporting. Name the baseline in every
  diff.
