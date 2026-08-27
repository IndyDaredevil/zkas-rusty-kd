# FLEET-DEPLOY v2.0.1.5 — r1 · 2026-08-27
### Policy: PROMOTE THE SOAKED BYTES. Production deploys the exact canary
### binary (F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0,
### commit 1b63698). Rust builds are not reproducible; a fresh release build
### is a sibling, not the soaked artifact. The canary prerelease is KEPT as
### artifact-of-record. v2.0.1.5-win is created as version-of-record; its CI
### asset hash gets ledgered alongside, but production runs the soaked exe.
### Every command names machine + shell (conduct law 14).

## 0 · GO / NO-GO (verdict before any step)
- Soak: shares clean at target SPM, 0 stale/invalid regression, zero
  FORENSIC IMPLAUSIBLE in canary-console.log, zk=ok throughout, A8 breaker
  INFO observed once. (A canary block found is a bonus, not a requirement.)
- Pace gate: solves_24h ≥ 33 at the 02:55 read (or already comfortably
  above at deploy time).
- Rollback pre-staged (step 3a parks the old exe before anything is killed).

## 1 · Version-of-record release (MacBook · zsh) — non-blocking; CI grinds
## ~35 min in background while the deploy proceeds on soaked bytes
```zsh
gh release create v2.0.1.5-win --repo IndyDaredevil/zkas-rusty-kd --target merged-v2.0.1.5 --title "bridge v2.0.1.5" --notes "BL-045 accept-loop fix (spawn-per-connection + 5s read timeout; stall class dead, parker-proven) + A8 balance-WARN circuit breaker. Production runs the soaked canary bytes F1484FB5...A3F0 (canary-1b63698 kept as artifact-of-record); this release's CI asset is the distribution/record build."
```
Expected: release URL; the Actions run's guard step must print
`OK: tag 'v2.0.1.5-win' matches banner v2.0.1.5`. Guard failure = STOP,
something is wrong at the source level.

## 2 · Pre-deploy capture (Kron · PowerShell)
Self-filling — records production PID + exe path, no placeholders:
```powershell
$pid34 = (Get-NetTCPConnection -LocalPort 3034 -State Listen).OwningProcess; $exe = (Get-Process -Id $pid34).Path; "prod PID=$pid34"; "prod exe=$exe"; (Get-FileHash $exe -Algorithm SHA256).Hash
```
Expected: PID, the repo-root target\release path, and the OLD exe's hash —
all three go in the record (the hash is the rollback identity).

## 3 · Deploy (Kron · PowerShell for staging, cmd for lifecycle)
3a. Park the old exe beside itself and stage the soaked one ($exe persists
from step 2's session — run in the SAME PowerShell window):
```powershell
Copy-Item $exe "$exe.bak-v2014"; Copy-Item C:\zkas\canary\stratum-bridge-1b63698.exe $exe -Force; (Get-FileHash $exe -Algorithm SHA256).Hash
```
Expected: F1484FB5DCC7631CB29BCED90F2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0
— the soaked bytes now sit at the production path. Mismatch = STOP.
NOTE: Copy-Item onto a RUNNING exe fails on Windows (file lock). If it
errors "being used by another process": kill first (3b), then re-run 3a's
Copy-Item + hash, then 3c. Park-before-kill is preserved either way because
the .bak copy succeeds regardless (reads are allowed).

3b. Kill production (the ONE process = both instances + dashboard):
```powershell
Stop-Process -Id $pid34 -Force
```

3c. Relaunch (Kron · cmd — production lifecycle dialect; from the repo root
that run-rc-merged.cmd lives in, i.e. the directory of $exe minus
\target\release):
```bat
run-rc-merged.cmd
```
Expected in the window: the echo header, then log output to the console
per production habit.

## 4 · GATES, in order (Kron · PowerShell) — soak law: no gate, no done
4a. Banner (HARD GATE — BL-047):
```powershell
Get-Content (Get-ChildItem 'C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs\RKStratum_*.log' | Sort LastWriteTime | Select -Last 1).FullName -First 25 | Select-String "2\.0\.1"
```
Expected: `RC merged bridge v2.0.1.5 (engine 2.0.1)`. v2.0.1.4 = STOP,
rollback (step 6).

4b. Merged mode — BOTH lines (two instances):
```powershell
Get-Content (Get-ChildItem 'C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs\RKStratum_*.log' | Sort LastWriteTime | Select -Last 1).FullName -First 60 | Select-String "MERGED MINING ENABLED"
```
Expected: exactly 2 matches.

4c. Listeners + fleet reconnect (six workers; w1 still on canary):
```powershell
Get-NetTCPConnection -LocalPort 5755,5765,3034 -State Listen | Select LocalPort, OwningProcess
```
Then watch the worker table fill to 6 in the log/console (~1 min).

4d. Reporter rotation (no restart needed — verified in source):
```powershell
Get-Content C:\zkas\reporter.log -Tail 5
```
Expected within ~10s of relaunch: `log rotated: RKStratum_... -> RKStratum_...`

4e. THE PROOF ON PRODUCTION — parker vs :3034 (the port that pinned at 55s):
```powershell
$parker = New-Object System.Net.Sockets.TcpClient; $parker.Connect('127.0.0.1',3034); "parker connected at $(Get-Date -Format HH:mm:ss.fff)"; 1..8 | ForEach-Object { "{0}  {1:N0}" -f (Get-Date -Format HH:mm:ss.fff), (Measure-Command { curl.exe -s --max-time 8 http://localhost:3034/metrics | Out-Null }).TotalMilliseconds; Start-Sleep -Seconds 1 }; try { $b = New-Object byte[] 1; $parker.Client.ReceiveTimeout = 2000; $n = $parker.Client.Receive($b); "server closed parker (FIN) - read timeout PROVEN" } catch { "parker NOT closed by server - timeout not working" }; $parker.Close()
```
Expected: flat ~2xx ms × 8 + FIN line. This lifts the tab-ban.

## 5 · Post-deploy tidy
- w1's rig UI → pool back to `stratum+tcp://192.168.1.96:5755`, worker name
  back to w1m. Verify it appears in production's Instance 1 table.
- Kill the canary process; keep C:\zkas\canary\ intact (exe + console log =
  soak evidence). Canary prerelease canary-1b63698 KEPT on GitHub.
- Record: deploy time, old-exe hash (step 2), new hash confirmed, the
  v2.0.1.5-win CI asset's zip hash once built (record-only).

## 6 · ROLLBACK (if any gate fails)
```powershell
Stop-Process -Id (Get-NetTCPConnection -LocalPort 3034 -State Listen).OwningProcess -Force; Copy-Item "$exe.bak-v2014" $exe -Force
```
Then relaunch run-rc-merged.cmd (cmd). Old hash from step 2 verifies.
Canary stays up for diagnosis; tab-ban resumes.

## 7 · ACCEPTANCE WINDOW
- .bak-v2014 retires after ONE clean production day (BL-031 policy).
- A1' final acceptance: ZERO timeout-kill scrapes across SEVEN days —
  `max_over_time(scrape_duration_seconds[7d])` staying < 5s is the single
  query verdict. Passing writes BL-050 (deploy close-out) and SCOPE r4
  (release closed); failing reopens A1' with whatever the pin's timestamp
  joins against.
