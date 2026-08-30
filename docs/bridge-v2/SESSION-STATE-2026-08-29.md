# SESSION-STATE — 2026-08-29 (S13 close, post-deploy + docs-rail reconciliation)
### Supersedes SESSION-STATE-2026-08-27.md. Deep content lives in the tier
### docs (ledger @ BL-056, SCOPE r4, FLEET-DEPLOY r1, NODE-CONTRACT v1.0.6,
### NODE-CUTOVER r1) — this doc is live state, gates, and queue.
### Cite, don't re-derive.

## OPEN THE NEXT SESSION WITH
Docs tip: 56ebb54 (merged-v2.0.1.5). Code tip: 1b63698 = running exe.
`merged-v2.0.1.4` is RETIRED READ-ONLY as of a2d8650 — do not commit to it.
Then answer: (1) D1 done — `.bak-v2014` retired? (2) pace gate 02:55 08-28,
still readable in retention or closed as unmeasured? (3) P1 Bolt brief pasted?
(4) mount synced to this session's four artifacts?

## LIVE STATE
- **Production bridge**: v2.0.1.5, deployed 08-28 from SOAKED CANARY BYTES
  (F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0 @
  1b63698). Single process, three listeners on one PID (14940 at deploy).
  All five post-deploy gates PASSED including the parker proof on `:3034` —
  **the "no browser tabs on :3034" interim rule is LIFTED** (BL-045 closed
  on production). Fleet 7/7, w1m returned from canary duty 284/0/0 `zk=ok`.
  Canary at `C:\zkas\canary\` kept as evidence.
- **zkas-node**: v1.0.6 as of 08-28 (H7 CLOSED). Exe 1B49D1FA…DC97D2, zip
  pin 8B63C491…B2D830, installed to `C:\zkas\node-v106\`. Cutover cost ~14
  min zKAS-leg downtime, zero bridge restarts, 27,495/0/0 shares; 32.8 GB
  datadir cold copy in 7:49, 0 failures. gRPC 16810 firewall-scoped to the
  MacBook (192.168.1.173) — the 08-24 LAN opening is closed. File logging ON.
  **No version banner exists; sha-256 is the only identity instrument.**
- **walletd**: still v1.0.5 (`C:\zkas\node\`, untouched by the node cutover).
  First launcher and first sha pin as of 08-29 (BL-055):
  `start-walletd-v1.0.5-r1.ps1` (098BEC5B…), secret now a DPAPI blob at
  `C:\zkas\walletd-secret.dpapi` via `set-walletd-secret-r2.ps1` (A769D354…;
  r1 VOID — reported success on a 1-character capture), `--proof-threads 6`,
  stdout → `C:\zkas\logs\walletd-<stamp>.out.log`. Binary BDCBE067…713C,
  mtime 2026-08-02. **Does NOT auto-start on reboot.**
  Measured cold-start: 269.7s single-threaded subtree cache (1,529,415
  leaves, notes=471) during which it accepts connections and answers
  NOTHING. Scales with leaf count — worse every week.
- **Alerting**: block cards were delivering at ~1-in-10 since 08-22 and
  nobody noticed (BL-051). The 15s→60s scrape change shrank the firing span
  to exactly 90s against Alertmanager `group_wait: 1m30s` — a dead tie.
  Fixed 08-29 with `keep_firing_for: 2m` on RcKasBlockFound,
  RcZkasBlockFound, RcMergedDoubleBlockFound; verified `keepFiringFor=120`
  via `/api/v1/rules`. Repo monitoring mirrors were stale since 08-22,
  synced at 96eff28.
- **Host**: event EIGHT 08-28 09:44, first post-UPS. **Unreadable** — Event
  ID 105 is not logged on this host and no CyberPower/PowerPanel service is
  installed, so BL-044's "one-bit diagnosis" capability does not exist yet.
  Kron-local by signature, unproven by instrument. Recovery cost with the
  operator asleep: ~34 min, gated on logon.
- **Docs rail**: reconciled 08-29. SCOPE r3, FLEET-DEPLOY r1 and
  SESSION-STATE-2026-08-27 had been stranded on the retired `.4` branch for
  two days while `.5` carried SCOPE r2 ("READS PENDING") — BL-056.
  Conduct laws 14, 15, 16 + the 1b amendment are now FILED in project
  instructions (they had been unfiled since 08-27).

## GATES PENDING
1. **D1 — `.bak-v2014` retirement**: eligible since 08-29 ~01:09 EDT
   (BL-031 one-clean-day policy). Run `-WhatIf` preview first. OPEN.
2. **D2 — A1′ seven-day acceptance**: `max_over_time(
   scrape_duration_seconds[7d]) < 5s` on/after **2026-09-04 ~01:09 EDT**.
   Pass → BL-050 close-out, SCOPE retires, v2.0.1.6 opens. Fail → reopen
   A1′ with the seven-day series as the read.
3. **Pace gate 02:55 08-28** (`rc:solves_24h ≥ 33`): set by the 08-27
   handoff, NO result recorded on any rail. Read from Prometheus while in
   retention or close explicitly as unmeasured.
4. **A4 residual**: one eyeball of the HourlyMergedReport card vs live `rc:`
   values. No longer a formality — BL-051 proved the Alertmanager leg had a
   real defect.

## QUEUE (unordered unless noted)
- **P1 Bolt paste — FIRST.** Brief cut and pinned since 08-27
  (P1-BOLT-BRIEF-r1.md, 86d2b546…a210, `~/zkas-lab/`), unpasted. Every day
  loses D_z/D_k curve permanently at 15-day retention.
- **H2 window** (largest open item): KRON-HARDENING §2–§6, Store/OS flush,
  service migration of the Session-1 processes, deliberate bridge relaunch.
  Riders: spare 19V brick; **PowerPanel install** (without it the UPS is not
  an instrument); launchers are load-bearing config and must be carried, not
  replaced. Stake: ~34 min asleep vs ~5 min awake.
- **H8 walletd v1.0.6 cutover** (elevated by the 269.7s measurement): check
  the treasury page's `notes` field use BEFORE cutover (`/api/wallet/balance`
  needs `?notes=1` in v1.0.6); cold-copy the wallet dir (checkpoints go to v8,
  forward-only); `--no-custodial` is NOT for us. Also `run-walletd.cmd`
  house-convention launcher (r2 debt).
- **H6 windows_exporter + ZkasLegDegraded**: still the only uninstrumented
  hypothesis for the original BL-032 wedge. Runbook drafted 08-26.
- **Upstream batch to firecash** (SCOPE §7): wedge report, consolidate
  mislabel, dmg signature, config-key silent drop, no version banner,
  argv secret. File together, not piecemeal.
- **Runbook r2s**: NODE-CUTOVER (two defects — `-AsByteStream` is PS 5.1
  incompatible; log-path glob grabs RocksDB WAL files); FLEET-DEPLOY (file
  lock ordering; cmd `&&` into PowerShell).
- **BRIDGE-SPEC §2** single-process clarification — still not written.
- **Fork rebase** `IndyDaredevil/zkas-rusty` onto v1.0.6: dry run clean
  (tree ed9acea4, zero conflicts). Schedule.
- **Cold storage sweep**: ~2M ZKAS, deferred pending an air-gapped machine
  for key generation. Architecture agreed, unexecuted.
- **Archival stewardship**: Kron holds a true-archival node with history from
  genesis — one of a closed, non-regrowable set. Formal approach undefined.
- **v2.0.1.6 seeds**: A6 job-delivery latency; A1-hygiene bounded retention.

## KEY VALUES
- Docs tip 56ebb54 · code tip 1b63698 · `.4` retired at a2d8650
- Ledger seal BL-056 · 1242 ln · 711980fd…b73c · commit ac4a603
- SCOPE r4 · 315 ln · cf6abfaf…0388 · commit 56ebb54
- FLEET-DEPLOY r1 · 115 ln · 5df03ad7…a4e4e (executed 08-28)
- SESSION-STATE-2026-08-27 · 99 ln · a7bdd148…4b02d (superseded by this doc)
- Production exe F1484FB5…A3F0 · node exe 1B49D1FA…DC97D2 · walletd
  BDCBE067…713C
- Treasury balance at 08-29 verification: 5,946,525,827,141 sompi
  (59,465.25827141 ZKAS), notes=472, scanned_blocks=2,953,223
- Every MacBook git op starts `cd ~/zkas/zkas-rusty-kd &&`; the clone rests
  on `merged-v2.0.1.5`
- Every script on Kron runs `-ExecutionPolicy Bypass` against an AllSigned
  policy — enforced-but-routed-around; KRON-HARDENING should commit or drop
(END)
