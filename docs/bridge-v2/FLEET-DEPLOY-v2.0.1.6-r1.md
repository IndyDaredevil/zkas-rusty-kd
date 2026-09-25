# FLEET-DEPLOY v2.0.1.6 — r1 · 2026-09-24
### Policy: direct deploy of the CI release asset, no canary (operator decision,
### S29: "we don't need a canary for this type of change"). Production copies
### the exact CI bytes pinned below; the pin is re-read on Kron before the swap.
### Every command names machine + shell (law III.2).

## 0 · Identity (pinned container-side 2026-09-24 ~13:25Z, sandbox)
- Release `v2.0.1.6-win` on IndyDaredevil/zkas-rusty-kd, tag @ `eac767d`
  (branch ws2-b1-static-traversal, tree 692b633e…). bridge-check 35960021888 ✓.
- Asset `zkas-v2.0.1.6-win-win64.zip`: 47,755,079 B, sha256
  `47e7a68068a4d571a1f7a1239c4924b40488a90927cc407e0c6be896aea91591`.
- Inside, at archive root (six exes + README.txt): `stratum-bridge.exe`
  15,247,360 B, sha256
  `82cc5917eb90ff6f4a67692056a29c3706698629691e2dd7157ede1b411171cf`.
  Strings present in the exe: `Connection: close` ×5, `TCP accept error on web
  dashboard` ×1 (the B3/B2 code is in the build; the banner is format!-built and
  is checked at gate 4a, not by strings).
- Contents: B1 static-path traversal fix (002f635) + B2–B5 and BRIDGE_BUILD 6
  (eac767d). Expected banner: `RC merged bridge v2.0.1.6 (engine 2.0.1)`.

## 1 · Pre-deploy capture (Kron · PowerShell, read-only) — run in ONE window
## and keep it open; $pid34/$exe/$oldhash carry into steps 2–3 and 6.
```powershell
$pid34 = (Get-NetTCPConnection -LocalPort 3034 -State Listen).OwningProcess; $exe = (Get-Process -Id $pid34).Path; $oldhash = (Get-FileHash $exe -Algorithm SHA256).Hash; "prod PID=$pid34"; "prod exe=$exe"; "prod hash=$oldhash"; curl.exe -s -o NUL -m 5 -w "B1 pre-fix probe: %{http_code}`n" --path-as-is "http://127.0.0.1:3034/static/C:/Windows/win.ini"
```
Expected: PID, exe path (the repo-root `target\release` path per FLEET-DEPLOY
v2.0.1.5), hash `F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0`
(the soaked v2.0.1.5 bytes), and `B1 pre-fix probe: 200`. A different hash means
production moved since FLEET-DEPLOY r1 — STOP and paste it.

## 2 · Stage + verify (Kron · PowerShell, same window, idempotent)
```powershell
$ProgressPreference='SilentlyContinue'; $d='C:\zkas\rel-v2016'; New-Item -ItemType Directory -Force $d | Out-Null; $z="$d\zkas-v2.0.1.6-win-win64.zip"; Invoke-WebRequest -Uri 'https://github.com/IndyDaredevil/zkas-rusty-kd/releases/download/v2.0.1.6-win/zkas-v2.0.1.6-win-win64.zip' -OutFile $z; (Get-FileHash $z -Algorithm SHA256).Hash -eq '47E7A68068A4D571A1F7A1239C4924B40488A90927CC407E0C6BE896AEA91591'
```
Expected: `True`. `False` = STOP (BL-002 rule: never proceed on a mismatched pin).

```powershell
$d='C:\zkas\rel-v2016'; Unblock-File "$d\zkas-v2.0.1.6-win-win64.zip"; Expand-Archive -Force "$d\zkas-v2.0.1.6-win-win64.zip" -DestinationPath $d; (Get-FileHash "$d\stratum-bridge.exe" -Algorithm SHA256).Hash -eq '82CC5917EB90FF6F4A67692056A29C3706698629691E2DD7157EDE1B411171CF'
```
Expected: `True`.

## 3 · Swap (Kron · PowerShell, same window) — the KAS leg is down from the
## kill to gate 4c (~1 min last time; not measured this session).
3a. Park the old exe beside itself (reads are allowed on a running exe):
```powershell
Copy-Item $exe "$exe.bak-v2015" -Force; (Get-FileHash "$exe.bak-v2015" -Algorithm SHA256).Hash -eq $oldhash
```
Expected: `True` — the rollback operand is in place.

3b. Kill production (one process = both instances + dashboard):
```powershell
Stop-Process -Id $pid34 -Force; Start-Sleep -Seconds 2; (Get-NetTCPConnection -LocalPort 3034 -State Listen -ErrorAction SilentlyContinue).Count
```
Expected: `0`.

3c. Copy the new bytes onto the production path and re-pin:
```powershell
Copy-Item C:\zkas\rel-v2016\stratum-bridge.exe $exe -Force; (Get-FileHash $exe -Algorithm SHA256).Hash -eq '82CC5917EB90FF6F4A67692056A29C3706698629691E2DD7157EDE1B411171CF'
```
Expected: `True`. `False` = go to §6 immediately.

3d. Relaunch (Kron · cmd, from the repo root that run-rc-merged.cmd lives in —
the directory of $exe minus `\target\release`, same as the v2.0.1.5 deploy):
```bat
run-rc-merged.cmd
```
Expected: the echo header, then log output to the console.

## 4 · GATES, in order (Kron · PowerShell) — no gate, no done
4a. Banner (HARD GATE):
```powershell
Get-Content (Get-ChildItem 'C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs\RKStratum_*.log' | Sort LastWriteTime | Select -Last 1).FullName -First 25 | Select-String "merged bridge v"
```
Expected: `RC merged bridge v2.0.1.6 (engine 2.0.1)`. Anything else = §6.

4b. Merged mode, both instances:
```powershell
Get-Content (Get-ChildItem 'C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs\RKStratum_*.log' | Sort LastWriteTime | Select -Last 1).FullName -First 60 | Select-String "MERGED MINING ENABLED" | Measure-Object | Select -Expand Count
```
Expected: `2`.

4c. Listeners, then watch the worker table fill to 6 in the console (~1 min):
```powershell
Get-NetTCPConnection -LocalPort 5755,5765,3034 -State Listen | Select LocalPort, OwningProcess
```
Expected: three rows, one PID.

4d. B1 closed (the reason for this release):
```powershell
curl.exe -s -o NUL -m 5 -w "B1 post-fix probe: %{http_code}`n" --path-as-is "http://127.0.0.1:3034/static/C:/Windows/win.ini"; curl.exe -s -o NUL -m 5 -w "dashboard js: %{http_code}`n" "http://127.0.0.1:3034/static/js/dashboard.js"
```
Expected: `B1 post-fix probe: 404` and `dashboard js: 200` (the legitimate path
still serves). 200 on the first line = B1 NOT closed = §6 and paste.

4e. B4 — ip label host-only, and B3 — Connection: close (after ≥1 worker is on):
```powershell
$m = curl.exe -s -m 8 -D - "http://127.0.0.1:3034/metrics"; ($m | Select-String -Pattern '^Connection: close' | Measure-Object).Count; ($m | Select-String -Pattern 'ip="[0-9.]+:[0-9]+"' | Measure-Object).Count; ($m | Select-String -Pattern 'ks_blocks_mined\{' | Select -First 2)
```
Expected: `1` (the header), `0` (no `ip="host:port"` series anywhere), then two
`ks_blocks_mined{...ip="192.168.1.NN"...}` lines with no port. A non-zero second
number = B4 not in effect = paste, do not roll back on this alone.

4f. Reporter rotation (no restart needed):
```powershell
Get-Content C:\zkas\reporter.log -Tail 3
```
Expected: a `log rotated: RKStratum_... -> RKStratum_...` line within ~10 s of relaunch.

## 5 · Post-deploy
- Prometheus: series re-mint on any restart; with B4 the `sum without (ip)`
  in alert_rules is now a no-op. No config change required. Watch for one
  phantom "block" card in the first minute (the known restart re-mint class,
  alert_rules.yml §1) — that is the existing behavior, not this release.
- Record (ledger, next append): deploy time, old hash (§1), new hash confirmed
  (§3c), gates 4a–4f verbatim, both probes.
- `.bak-v2015` retires after ONE clean production day (BL-031 policy).
- Branch consolidation (merged-v2.0.1.6, option A) rides the close-out commit
  with STARTUP-ORDER r3.

## 6 · ROLLBACK (any gate fails) — Kron · PowerShell, same window as §1
```powershell
Stop-Process -Id (Get-NetTCPConnection -LocalPort 3034 -State Listen).OwningProcess -Force -ErrorAction SilentlyContinue; Copy-Item "$exe.bak-v2015" $exe -Force; (Get-FileHash $exe -Algorithm SHA256).Hash -eq $oldhash
```
Expected: `True`. Then relaunch `run-rc-merged.cmd` (cmd) and re-run 4a expecting
`v2.0.1.5`. B1 is open again on rollback — say so in the record.
