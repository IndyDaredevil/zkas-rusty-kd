# NODE-CUTOVER v1.0.6 — r1 · 2026-08-28
### Scope: NODE BINARY ONLY. walletd stays v1.0.5 (wire-compatible; its cutover is
### its own window — NODE-CONTRACT-v1.0.6 §6). Bridge untouched; it auto-reconnects
### (BL-032 blip taxonomy). Fork rebase is separate and later.
### Pre-work already done 2026-08-28: running binary pinned 0E2B5B43D844AF6D824279
### D1F4E46F7E0198DB579F43471D304D6896F57A24AF @ C:\zkas\node-v105\zkas-node.exe;
### ShieldedHistoryChunk ABSENT (pre-e49ce61 build, old-layout archive); config
### read back — pin file NOT armed.
### Decisions baked: rpclisten stays 0.0.0.0:16810, firewall-scoped to MacBook IP
### (H7); perf-metrics on; shielded-history=on via LAUNCHER (TOML drops the key —
### NODE-CONTRACT §2 config-key trap); nologfiles=false unchanged.
### Every command names machine + shell. Cross-step $variables require the SAME
### PowerShell window; each such dependency is stated at the step.
### Kron PowerShell steps assume an ELEVATED prompt (firewall + process control).

## 0 · GO / NO-GO
- Node currently healthy (templates flowing, zk=ok on the bridge status line).
- No open incident in flight on Kron.
- Expected alert noise DURING the window: TG "template age >30s" cards while the
  node is down/restarting — expected, not actionable. Bridge stays up.
- Downtime budget: minutes (stop → copy runs with node DOWN → relaunch).
  The datadir copy is the long pole — see step 7's silence profile.

## 1 · MacBook IP capture (MacBook · zsh)
```zsh
ipconfig getifaddr en0
```
Expected: one IPv4 (e.g. 192.168.x.y). Record it; step 11 prompts for it.
Rider (H7): give the MacBook a DHCP reservation at the router so this scoping
doesn't rot when the lease moves.

## 2 · walletd capture (Kron · PowerShell) — walletd still RUNNING
Parses the wallet dir off the process; echoes ONLY the dir, never the cmdline
(it contains --wallet-secret — never paste that anywhere, incl. this chat).
```powershell
$wp = (Get-NetTCPConnection -LocalPort 8501 -State Listen | Select-Object -First 1).OwningProcess; $wcl = (Get-CimInstance Win32_Process -Filter "ProcessId=$wp").CommandLine; if ($wcl -match '--wallet-dir\s+"?([^"\s]+)"?') { $wdir = $Matches[1]; "walletd PID=$wp"; "wallet-dir=$wdir" } else { "STOP: --wallet-dir not on the walletd command line - read the launch script manually and set `$wdir by hand before step 8" }
```
Expected: `walletd PID=<n>` and `wallet-dir=<path>`. $wp and $wdir persist in
THIS window — steps 6 and 8 use them; do not close it.

## 3 · Asset probe + download + pin (Kron · PowerShell)
Probe first (asset list was API-rate-limited during evaluation; name derived
from upstream deploy.yaml — fail loud here if it differs):
```powershell
curl.exe -sI -L https://github.com/firecash/zkas-rusty/releases/download/zkas-v1.0.6/zkas-zkas-v1.0.6-win64.zip | findstr /I "HTTP/ content-length"
```
Expected: a final `HTTP/... 200` and a content-length in the tens of MB.
404 = STOP: open the release page's asset list and correct the name in steps
3b/4 before proceeding.

3b. Download + pin (no upstream checksum is published — the hash you record HERE
is the artifact identity, BL-002 rule; ~30-60s, curl progress meter):
```powershell
New-Item -ItemType Directory -Force C:\zkas\node-v106 | Out-Null; curl.exe -L -o C:\zkas\node-v106\zkas-zkas-v1.0.6-win64.zip https://github.com/firecash/zkas-rusty/releases/download/zkas-v1.0.6/zkas-zkas-v1.0.6-win64.zip; (Get-FileHash C:\zkas\node-v106\zkas-zkas-v1.0.6-win64.zip -Algorithm SHA256).Hash
```
Expected: one 64-hex hash → RECORD as zip pin.

## 4 · Extract + stage exe (Kron · PowerShell)
```powershell
Expand-Archive C:\zkas\node-v106\zkas-zkas-v1.0.6-win64.zip -DestinationPath C:\zkas\node-v106 -Force; Copy-Item C:\zkas\node-v106\kaspad.exe C:\zkas\node-v106\zkas-node.exe; (Get-FileHash C:\zkas\node-v106\kaspad.exe, C:\zkas\node-v106\zkas-node.exe -Algorithm SHA256).Hash
```
Expected: TWO IDENTICAL hashes → RECORD as the v1.0.6 node identity (there is
no version banner; this hash IS the version — NODE-CONTRACT §2). Mismatch or
missing kaspad.exe = STOP.

## 5 · Write config + launcher (Kron · PowerShell)
Whole-file unit of review. BOM-free write ([IO.File]::WriteAllText +
UTF8Encoding($false) — the [Text.Encoding]::UTF8 BOM law).
```powershell
$toml = @"
# ZKas node v1.0.6 config - carries zkas-node-758k.toml forward. Deltas: perf-metrics on.
# CONFIG-KEY DROP TRAP (args.rs @ zkas-v1.0.6, verified): shielded-history,
# verify-shielded-history, consensus-diag, shielded-anchor-overrides, externalip
# are parsed from this file then SILENTLY DISCARDED (CLI-only fields, BL-017 class).
# Those flags ride the LAUNCHER (run-zkas-node.cmd) exclusively.
# Isolation
appdir = "C:/zkas/node-data"
# Services
utxoindex = true
archival = true                     # MANDATORY (BL-002); an untested --yes would prune
# Peering
outpeers = 16
maxinpeers = 8
addpeer = ["185.147.157.125:16111", "160.187.211.153:16111", "204.10.194.28:17951"]
disable-upnp = true
# Bindings
externalip = "108.95.94.128"        # DEAD KEY on this binary (dropped - see trap above); kept to preserve intent
listen = "0.0.0.0:16811"
rpclisten = "0.0.0.0:16810"         # LAN-open by decision 2026-08-28; firewall-scoped to MacBook IP (H7)
# Housekeeping
nologfiles = false                  # file logging ON (BL-032 witness rail)
perf-metrics = true                 # 2026-08-28: node RSS/DB counters into the log every 10s (memory-slope instrument)
"@; [IO.File]::WriteAllText("C:\zkas\node\zkas-node-v106.toml", $toml, (New-Object System.Text.UTF8Encoding($false)))
$cmd = @"
@echo off
cd /d %~dp0
REM v1.0.6 launcher - the ONLY supported node start path (run-rc-merged.cmd law; H2 service-migration target).
REM --shielded-history=on: the = is REQUIRED (require_equals). The TOML cannot carry this flag (dropped) - it lives HERE.
zkas-node.exe --configfile C:\zkas\node\zkas-node-v106.toml --shielded-history=on
"@; [IO.File]::WriteAllText("C:\zkas\node-v106\run-zkas-node.cmd", $cmd, (New-Object System.Text.UTF8Encoding($false)))
"toml-lines=$((Get-Content C:\zkas\node\zkas-node-v106.toml).Count) perf=$((Select-String -Path C:\zkas\node\zkas-node-v106.toml -Pattern '^perf-metrics = true').Count) archival=$((Select-String -Path C:\zkas\node\zkas-node-v106.toml -Pattern '^archival = true').Count) bom-first-byte=$((Get-Content C:\zkas\node\zkas-node-v106.toml -AsByteStream -TotalCount 1))"
```
Expected: `toml-lines=22 perf=1 archival=1 bom-first-byte=35` (35 = '#', i.e.
no BOM). Any other counts = STOP, re-read the file.
NOTE: zkas-node-758k.toml stays in place UNTOUCHED — it is the v1.0.5 rollback
config (the new launcher never references it; no two-live-copies conflict).

## 6 · Stop walletd (Kron · PowerShell — SAME window as step 2)
Treasury page goes dark from here until step 12 — expected.
```powershell
Stop-Process -Id $wp -Force; Start-Sleep 2; if (Get-Process -Id $wp -ErrorAction SilentlyContinue) { "STOP: walletd still running" } else { "walletd stopped" }
```
Expected: `walletd stopped`.

## 7 · Stop node + cold copies (Kron)
7a. Graceful stop: **Ctrl+C in the node's console window**, wait for the
process to exit. (Fallback if headless: `Stop-Process` on the 16811 owner —
hard kill, same as historical relief restarts.) Then verify (PowerShell):
```powershell
$np = (Get-NetTCPConnection -LocalPort 16811 -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1).OwningProcess; if ($np) { "STOP: node still running pid=$np" } else { "node stopped - 16811 free" }
```
Expected: `node stopped - 16811 free`. TG template-age cards begin ~30s later —
expected noise for the rest of the window.

7b. Datadir cold copy — THE rollback path across the forward-only layout
boundary. SILENCE PROFILE: ~12+ GB local copy, several minutes, one line per
file; success = robocopy exit code 0-3.
```powershell
robocopy C:\zkas\node-data C:\zkas\backup\node-data-pre-v106 /MIR /R:1 /W:1; "robocopy exit=$LASTEXITCODE (0-3 = success)"
```
Expected: summary table, `robocopy exit=` 0-3. Exit >= 8 = STOP.

7c. Config archive copy (one file, instant):
```powershell
Copy-Item C:\zkas\node\zkas-node-758k.toml C:\zkas\backup\zkas-node-758k.toml; (Get-FileHash C:\zkas\backup\zkas-node-758k.toml -Algorithm SHA256).Hash
```
Expected: one hash → record.

## 8 · Wallet dir cold copy (Kron · PowerShell — SAME window as step 2, walletd stopped)
Insurance only (walletd is NOT upgrading; checkpoint-v8 boundary not crossed) —
but it is one command while everything is already down. Seconds to a minute.
```powershell
robocopy $wdir C:\zkas\backup\wallets-pre-v106 /MIR /R:1 /W:1; "robocopy exit=$LASTEXITCODE (0-3 = success)"
```
Expected: exit 0-3. If step 2 hit the STOP branch, set $wdir by hand first.

## 9 · Launch v1.0.6 (Kron · cmd — a NEW console window, keep it open)
```cmd
C:\zkas\node-v106\run-zkas-node.cmd
```
Expected console: startup banner (reports engine `2.0.1` — NOT the zkas
version; the step-4 hash is the version), datadir open on C:/zkas/node-data,
peer connections, then catch-up block processing. A single
`shielded history: ...` line MAY appear once (a fetch attempt answered
empty/skipped — archive already complete); a LONG transfer+verify here would
mean the archive was NOT complete and is being healed: let it run (progress +
ETA every 15s), it serves RPC throughout.
`failed parsing config file` = STOP: config/binary mismatch, re-check step 5.

## 10 · Acceptance gates (Kron · PowerShell)
BL-001 health: advancing DAA + UTXO-validated > 0 — read the log twice, >= 60s
apart, and require the tip numbers to ADVANCE between reads:
```powershell
Get-ChildItem C:\zkas\node-data -Recurse -Filter *.log | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | Get-Content -Tail 15
```
Expected (each run): fresh timestamps; block/DAA figures higher on the second
read; perf-metrics counter lines present (the new instrument, ~every 10s).
Bridge-side confirmation: status line returns to zk=ok, template-age cards
resolve. A found block is a bonus, not a gate.

## 11 · Firewall scoping — H7 (Kron · PowerShell, elevated)
11a. Inventory what 16810 allows today (the 08-24 opening):
```powershell
Get-NetFirewallPortFilter | Where-Object LocalPort -eq 16810 | Get-NetFirewallRule | Format-Table Name, DisplayName, Enabled, Profile, Direction, Action -AutoSize
```
Expected: the existing allow rule(s) listed — record names.

11b. Scope: disable every existing inbound 16810 rule, create ONE scoped to the
MacBook (prompts for the step-1 IP — no placeholder), read back:
```powershell
$mac = Read-Host "MacBook IPv4 from step 1"; Get-NetFirewallPortFilter | Where-Object LocalPort -eq 16810 | Get-NetFirewallRule | Where-Object Direction -eq 'Inbound' | Disable-NetFirewallRule; New-NetFirewallRule -DisplayName "zkas gRPC 16810 - MacBook only (H7)" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 16810 -RemoteAddress $mac -Profile Any | Out-Null; Get-NetFirewallPortFilter | Where-Object LocalPort -eq 16810 | Get-NetFirewallRule | Where-Object Enabled -eq 'True' | Format-Table DisplayName, Profile, Direction, Action -AutoSize
```
Expected readback: EXACTLY ONE enabled rule — "zkas gRPC 16810 - MacBook only
(H7)", all profiles. Zero production risk: the bridge reaches 16810 via
loopback (never traverses the firewall); the rigs speak stratum 5755/5765 only.

## 12 · Restart walletd (Kron)
Relaunch via your usual walletd launch script — the same one that produced the
step-2 process (its path/name is operator-held; the captured command line is
the reconstruction fallback, and it contains the secret — handle accordingly).
Verify (PowerShell):
```powershell
if (Get-NetTCPConnection -LocalPort 8501 -State Listen -ErrorAction SilentlyContinue) { "walletd listening on 8501" } else { "STOP: walletd not listening" }
```
Expected: `walletd listening on 8501`; treasury page balance renders (v1.0.5
walletd behavior unchanged against the v1.0.6 node).

## 13 · Post-cutover, operator-scheduled (not in this window)
13a. ONE deliberate archive verification run: stop the node (step 7a pattern),
then in a Kron cmd window:
```cmd
C:\zkas\node-v106\zkas-node.exe --configfile C:\zkas\node\zkas-node-v106.toml --shielded-history=on --verify-shielded-history
```
SILENCE PROFILE: replay of the whole scan archive before serving — minutes to
tens of minutes at current chain size, progress + ETA logged every 15s.
Read-only; deletes nothing. Expected verdict line: leaves replay exactly to the
PoW-committed frontier. A MISMATCH here is a finding, not a fix — record it and
bring it to the session before acting. Afterwards relaunch via the launcher
(this flag is one-off; it does NOT belong in the launcher).
13b. Deferred, each its own change: walletd v1.0.6 cutover (checklist =
NODE-CONTRACT §6 deltas; treasury-page `notes` check belongs THERE) · fork
rebase onto zkas-v1.0.6 (dry-run merge clean, tree ed9acea4, 08-28 vs origin
30b0700 — re-run against the local tree at execution time, law 4/13d) · H2
service migration (target = run-zkas-node.cmd).

## 14 · ROLLBACK (only path: restore across the layout boundary)
The moment v1.0.6 first writes, the datadir contains records only v1.0.6+ can
read — rollback is RESTORE + old pair, never old-binary-on-new-data:
1. Stop v1.0.6 node (step 7a pattern) + stop walletd (step 6 pattern).
2. Restore: `robocopy C:\zkas\backup\node-data-pre-v106 C:\zkas\node-data /MIR /R:1 /W:1`
   (same silence profile as 7b).
3. Relaunch the OLD pair — v1.0.5 exe + 758k config; the NEW launcher must NOT
   be used (its --shielded-history flag is unknown to the v1.0.5 binary and
   fails the launch):
```cmd
C:\zkas\node-v105\zkas-node.exe --configfile C:\zkas\node\zkas-node-758k.toml
```
4. Identity check: exe hash must read 0E2B5B43D844AF6D824279D1F4E46F7E0198DB57
   9F43471D304D6896F57A24AF. Restart walletd (step 12).

## 15 · RECORD AT CLOSE (ledger/session-state lines)
- zip pin (step 3b) · exe pin (step 4) · launch timestamp · acceptance reads.
- Firewall: rule names disabled (11a) + scoped rule created; MacBook IP used.
- Incident line if anything deviated (law 9).
- NODE-CONTRACT-v1.0.6.md committed + mount delete-then-add executed (law 2f).
