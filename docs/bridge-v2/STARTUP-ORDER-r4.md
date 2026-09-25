# STARTUP-ORDER — Kron full-stack manual start · r4 · 2026-09-25 (r2 was 2026-09-11; r1 2026-08-30; r3 cut 09-24, never landed, VOID)
### r4: kaspad v2.1.0 via versioned launcher (BL-126); stratum-bridge v2.0.1.6
### (B1–B5 port of upstream 2a47b24, BL-124). §§1 and 4 revised.
### r2: node + walletd at v1.0.8 (H8, BL-093/094); walletd v1.0.8 status gate;
### §7 gains ZkasSupplyCheck + KronEventsSampler + the two-reporter trap;
### §8 = 8 PASS, ELEVATED. Rollback identities retained as notes.
### The ONLY authoritative startup reference. Supersedes all personal notes —
### the 08-30 impostor incident (BL-065) was caused by a stale notes line;
### any copy of a start command living outside this doc is a defect.
### Verification after ANY start/restart: `check-kron.ps1` (§8) — identity-
### pinned, prints the fix for whatever is missing.
### SESSION LAW (BL-039): all six processes live in RDP Session 1. The
### interactive session is the kill domain — DISCONNECT, never sign out.
### Each launcher gets its OWN console window, LEFT OPEN (closing a console
### kills its process — the single-window version of the same law; the
### reporter died exactly this way on 09-11 and only the Button noticed).
### INTEGRITY LEVEL (BL-093; read 09-11): launch and kill from the SAME
### integrity level. walletd runs ELEVATED — its launcher (§3), any
### Stop-Process against it, and the Button (§8) run from an elevated
### PowerShell. A non-elevated checker reads walletd's path as null and
### reports a FALSE IMPOSTOR (n=1, two runs 09-11).

## 0 · ORDER (dependency-driven)
1+2 kaspad and zkas-node (parallel, no interdependency)
3   walletd — only after zkas-node is HEALTHY (never against a node that
    cannot serve genesis — the upstream wallet-loss class)
4   stratum-bridge — after both nodes (degrades gracefully, but clean order
    is nodes-first)
5   alertmanager — BEFORE prometheus (no notification into a dead receiver)
6   prometheus — last daemon (first scrapes hit live targets)
7   scheduled tasks — VERIFY, never hand-start
Then §8: the button.

## 1 · kaspad v2.1.0 (KAS leg) — Kron · cmd, new window
```cmd
C:\rusty-kaspa-v210\run-kaspad-v210.cmd
```
Identity (BL-126, 09-25): the launcher runs C:\rusty-kaspa-v210\kaspad.exe =
16BD68241C79113858C873EE16C5267809D7B8DF11E878BFDEC9802E2A33A1DA (official
kaspanet/rusty-kaspa v2.1.0 win64 asset, zip FB25743A…8EC0F) with the
unchanged C:\Node-v2\config.toml (appdir ~/.rusty-kaspa-v2, DB version 7 in
both v2.0.1 and v2.1.0 — no resync; synced within ~18 s on cutover). Banner:
`kaspad v2.1.0`. Gate: 16110 + 17110 listening, owned by
C:\rusty-kaspa-v210\kaspad.exe; bridge status line `v=2.1.0 … zk=ok`.
Firewall: program+port rule "kaspad v2.1.0 P2P inbound" (16111, R-4 style)
minted 09-25 before first launch; no first-run prompt appeared (n=1).
Rollback operand (untouched): C:\rusty-kaspa-v2\target\release\kaspad.exe =
084C2B928CB3DCEB2F752728581DE39EF2471640F5F894EF73C2ED1BBBC82743 (v2.0.1);
relaunch it with the same --configfile line if v2.1.0 must come down.
kaspad logs: nologfiles = false → C:\Users\inmyh\AppData\Local\.rusty-kaspa-v2\
kaspa-mainnet\logs (KASPAD-EVAL r1 §3.3 said "no log file" — wrong, corrected
BL-126). `externalip` in its TOML is a dead key on this engine (BL-063
drop-trap class) — do not rely on it.

## 2 · zkas-node v1.0.8 — Kron · cmd, new window
```cmd
C:\zkas\node-v108\run-zkas-node.cmd
```
The launcher is LOAD-BEARING (bakes --shielded-history=on; the TOML cannot
carry it — BL-063). NEVER start the exe by hand; the legacy exes that made
that mistake survivable-looking are quarantined in
C:\zkas\archive\legacy-node-dir\ (BL-065). Identity: exe
C:\zkas\node-v108\zkas-node.exe, sha256 45687E24E925C4ED777290C58B3A74B683
39C8FA6C84F4E303533FC04652236D (no version banner exists — hash IS the
version; pinned in check-kron). Gate: 16810 + 16811 listening; BL-001
health (advancing DAA, UTXO-validated > 0) before starting walletd.
Rollback identity (note only, NOT a start path — v1.0.6 must never open
v1.0.8 data): node-v106\zkas-node.exe 1B49D1FA5416130A6CB82A166E5941E778
EE1266E8BD5ACB23EA810B01DC97D2; datadir cold copy backup\node-data-pre-v108.
Known gap (H2(a)): the node writes NO log file — nologfiles=false queued.

## 3 · zkas-walletd v1.0.8 — Kron · ELEVATED PowerShell, new window
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\node\start-walletd-v1.0.8-r1.ps1
```
Versioned launcher (BL-055 pattern; v1.0.8-r1 minted at BL-093). Invoke by
this FULL PATH only — a glob over C:\zkas\node matched the launcher and
started a second walletd once (I-37, BL-116). Its command line carries the
DPAPI-decrypted secret path — never paste a walletd cmdline readback
anywhere. Identity: C:\zkas\walletd-v108\zkas-walletd.exe, sha256
B5B1DDA9093D1FB55A92A76D4D7CFE1ECDC67D57BCFCB0701FBFBAC7EF8C932C (pinned in
check-kron). Gate: 8501 listening; then `/api/wallet/status` answers with
`synced` true · `missing_history` false · `warming`/`loading` false ·
`blocks_behind` 0 or single digits; treasury page renders. Reporter shows
`WARN beats DEFERRED` until then — expected, not actionable. v1.0.8 also
carries `/api/wallet/warm` + `warm_wallets` (cutover rider, BL-093);
semantics unread here (n=0) — the status gate above is the contract.
Rollback identity (note only): wallet dir cold copy backup\wallets-pre-v108.

## 4 · stratum-bridge v2.0.1.6 — Kron · cmd, new window (lifecycle dialect)
```cmd
C:\Users\inmyh\zkas-rusty-kd\run-rc-merged.cmd
```
READ THE HEADER EVERY LAUNCH (BL-017/BL-019): the two ENABLED env lines are
the contract. Gate: BOTH "MERGED MINING ENABLED" lines (node + treasury
address echo) · 5755/5765/3034 owned by ONE pid (BL-039) · worker table
fills to 7 within ~1 min · status line `zk=ok`.
Identity (BL-124, 09-24): the launcher runs
C:\Users\inmyh\zkas-rusty-kd\target\release\stratum-bridge.exe =
82CC5917EB90FF6F4A67692056A29C3706698629691E2DD7157EDE1B411171CF (CI asset of
release v2.0.1.6-win, tag eac767d). Banner: `RC merged bridge v2.0.1.6
(engine 2.0.1)`. Rollback operand beside it: `stratum-bridge.exe.bak-v2015` =
F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0 (the soaked
v2.0.1.5 bytes; retires after one clean day, BL-031). Post-start B1 check:
`curl.exe -s -o NUL -m 5 -w "%{http_code}" --path-as-is
http://127.0.0.1:3034/static/C:/Windows/win.ini` must print 404.

## 5 · alertmanager — Kron · cmd (cwd is LOAD-BEARING: alertmanager.yml is
## cwd-relative)
```cmd
cd /d C:\Prometheus\alertmanager
```
```cmd
alertmanager.exe --cluster.listen-address=""
```
Gate: 9093 listening.

## 6 · prometheus — Kron · cmd (cwd is LOAD-BEARING: TSDB lives at .\data —
## wrong cwd = a new EMPTY history, silently)
```cmd
cd /d C:\Prometheus
```
```cmd
prometheus.exe --config.file="C:\Prometheus\prometheus.yml" --web.enable-lifecycle
```
Gate: 9090 up, targets green, rc_merged_bridge scrape_duration at the
~230ms floor. Launcher .cmds for 5+6 queued with kaspad's (H2 prereq).

## 7 · Scheduled tasks — verify only (Kron · PowerShell)
```powershell
Get-ScheduledTask ZkasReporter,NetworkHistorySampler,ZkasSupplyCheck,KronEventsSampler | Select-Object TaskName,State; "9151 listeners: $((Get-NetTCPConnection -LocalPort 9151 -State Listen -ErrorAction SilentlyContinue | Measure-Object).Count)"; "reporter processes: $((Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where-Object { $_.CommandLine -match 'zkas-reporter' } | Measure-Object).Count)"
```
Expected: four tasks listed; `9151 listeners: 1`; `reporter processes: 1`.
- ZkasReporter (r8, BL-118): liveness = :9151 listening + reporter.log
  moving. `task=Running` alone proves nothing (BL-067). Restart ONLY
  task-tier, and ONLY when `reporter processes` reads 0: `Stop-ScheduledTask
  ZkasReporter; Start-ScheduledTask ZkasReporter`. TWO-REPORTER TRAP (I-43,
  BL-118; seen again 09-11): a hand-started copy plus the task instance
  fight for 9151 — log reads "conflicts with an existing registration".
  Stop ALL reporter processes, then start ONE, task-tier.
- NetworkHistorySampler (r2, BL-114): 5-min D_z/D_k + near-miss sampler,
  :3034-only by design. Console blink per trigger is known-cosmetic until
  the S4U fix (BL-067).
- ZkasSupplyCheck (r8, BL-108): hourly getShieldedSupply vs schedule →
  textfile zkas_supply.prom; alerts Stale/Fail/Drift.
- KronEventsSampler (r2, BL-119): 5-min restart-event sampler → kron_events
  webhook; state C:\zkas\kron-events-state.json.

## 8 · THE BUTTON — after any start, restart, or doubt (Kron · ELEVATED PowerShell)
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\check-kron.ps1
```
Expected steady state: `ALL 8 UP - stack correct` (script = r3 content,
8CECBABC…, BL-106: node 45687E24… + walletd B5B1DDA9… pinned by path+sha).
Any FAIL prints its own fix — read the process before obeying it: a
legitimate upgrade MUST fail as IMPOSTOR (BL-094), and a NON-ELEVATED run
fails walletd as IMPOSTOR with an empty `()` path (09-11) — that is the
checker, not walletd. Limits (honest): proves process identity, not leg
health — `zk=ok` on the bridge status line remains the template-flow
instrument; pins inside the script must be revved IN THE SAME WINDOW as any
artifact they pin.

## 9 · FULL-STACK COLD START, one screen
kaspad (§1) + zkas-node (§2) → wait node-healthy → walletd (§3) → bridge
(§4) → alertmanager (§5) → prometheus (§6) → verify tasks (§7) → button
(§8) → glance: bridge `zk=ok`, worker table 7, treasury renders.

## 10 · PROVENANCE
r1 cut from live-process readback + cutover pins, 2026-08-30, after BL-065.
r2 cut 2026-09-11 from the ledger rail (BL-093/094/106/108/114/116/118/119)
and the 09-11 elevated process read (walletd path + Button 8/8); no live
launch was re-executed for r2 — §§1, 4, 5, 6 carry r1's readback unchanged.
r4 cut 2026-09-25 from two live readbacks: the v2.0.1.6 bridge deploy
(FLEET-DEPLOY v2.0.1.6 r1 gates 4a–4e + production-path hash, BL-124; §4)
and the kaspad v2.1.0 cutover (KASPAD-EVAL v2.1.0 r1 §5 steps 1–4 + the
bridge status line, BL-126; §1). §§2, 3, 5, 6 unchanged from r2. r3 (§4
only, 7224419e…) was cut 09-24 and superseded before landing — VOID.
Revision triggers: any component cutover, launcher mint, H2 service
migration (which retires §§1–6 of this doc in favor of service definitions
and demotes it to the manual-fallback reference).
