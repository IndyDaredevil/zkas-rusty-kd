# STARTUP-ORDER — Kron full-stack manual start · r1 · 2026-08-30
### The ONLY authoritative startup reference. Supersedes all personal notes —
### the 08-30 impostor incident (BL-065) was caused by a stale notes line;
### any copy of a start command living outside this doc is a defect.
### Verification after ANY start/restart: `check-kron.ps1` (§8) — identity-
### pinned, prints the fix for whatever is missing.
### SESSION LAW (BL-039): all six processes live in RDP Session 1. The
### interactive session is the kill domain — DISCONNECT, never sign out.
### Each launcher gets its OWN console window, LEFT OPEN (closing a console
### kills its process — the single-window version of the same law).

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

## 1 · kaspad (KAS leg) — Kron · cmd, new window
```cmd
C:\rusty-kaspa-v2\target\release\kaspad.exe --configfile "C:\Node-v2\config.toml"
```
Gate: 16110 + 17110 listening. NOTE: only leg without a launcher .cmd yet
(queued; H2 prerequisite). `externalip` in its TOML is a dead key on this
engine (BL-063 drop-trap class) — do not rely on it.

## 2 · zkas-node v1.0.6 — Kron · cmd, new window
```cmd
C:\zkas\node-v106\run-zkas-node.cmd
```
The launcher is LOAD-BEARING (bakes --shielded-history=on; the TOML cannot
carry it — BL-063). NEVER start the exe by hand; the legacy exes that made
that mistake survivable-looking are quarantined in
C:\zkas\archive\legacy-node-dir\ (BL-065). Identity: exe
C:\zkas\node-v106\zkas-node.exe, sha256 1B49D1FA5416130A6CB82A166E5941E778
EE1266E8BD5ACB23EA810B01DC97D2 (no version banner exists — hash IS the
version). Gate: 16810 + 16811 listening; BL-001 health (advancing DAA,
UTXO-validated > 0) before starting walletd.

## 3 · zkas-walletd v1.0.5 — Kron · cmd or PowerShell
```cmd
powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\node\start-walletd-v1.0.5-r1.ps1
```
Versioned launcher (BL-055); name rolls at the walletd cutover. Its command
line carries the DPAPI-decrypted secret path — never paste a walletd cmdline
readback anywhere. Gate: 8501 listening; key-index line in its log; treasury
page renders.

## 4 · stratum-bridge v2.0.1.5 — Kron · cmd, new window (lifecycle dialect)
```cmd
C:\Users\inmyh\zkas-rusty-kd\run-rc-merged.cmd
```
READ THE HEADER EVERY LAUNCH (BL-017/BL-019): the two ENABLED env lines are
the contract. Gate: BOTH "MERGED MINING ENABLED" lines (node + treasury
address echo) · 5755/5765/3034 owned by ONE pid (BL-039) · worker table
fills to 7 within ~1 min · status line `zk=ok`.

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

## 7 · Scheduled tasks — verify only
- ZkasReporter: liveness = :9151 listening + reporter.log moving.
  `task=Running` alone proves nothing (BL-067). Restart ONLY task-tier:
  `Stop-ScheduledTask`/`Start-ScheduledTask -TaskName ZkasReporter`.
- NetworkHistorySampler: 5-min D_z/D_k sampler, :3034-only by design.
  Console blink per trigger is known-cosmetic until the S4U fix (BL-067).

## 8 · THE BUTTON — after any start, restart, or doubt
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\check-kron.ps1
```
Expected steady state: 7 PASS (script r2). Any FAIL prints its own fix.
Limits (honest): proves process identity, not leg health — `zk=ok` on the
bridge status line remains the template-flow instrument; pins inside the
script must be revved IN THE SAME WINDOW as any artifact they pin.

## 9 · FULL-STACK COLD START, one screen
kaspad (§1) + zkas-node (§2) → wait node-healthy → walletd (§3) → bridge
(§4) → alertmanager (§5) → prometheus (§6) → verify tasks (§7) → button
(§8) → glance: bridge `zk=ok`, worker table 7, treasury renders.

## 10 · PROVENANCE
Cut from live-process readback + cutover pins, 2026-08-30, after BL-065.
Revision triggers: any component cutover, launcher mint, H2 service
migration (which retires §§1–6 of this doc in favor of service definitions
and demotes it to the manual-fallback reference).
