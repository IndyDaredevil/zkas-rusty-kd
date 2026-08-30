# check-kron.ps1 - r2 2026-08-30 - one-shot stack state verifier ("easy button")
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\check-kron.ps1
# r2 changes (BL-065/BL-067): reporter liveness = :9151 + log-age<90m (task=Running
# is NOT liveness; log is event-paced); launcher-integrity check added (empty
# launcher was invisible until launch). Identity pins: rev IN THE SAME WINDOW as
# any artifact they pin. Start order on multi-fail:
# kaspad + zkas-node first -> walletd after node healthy -> bridge -> alertmanager -> prometheus.
$fails = 0
function Port($n) { (Get-NetTCPConnection -LocalPort $n -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1).OwningProcess }

# 0 launcher integrity (BL-065: run-zkas-node.cmd was found EMPTY)
$lc = 'C:\zkas\node-v106\run-zkas-node.cmd'
$lok = (Test-Path $lc) -and ((Get-Content $lc -ErrorAction SilentlyContinue).Count -eq 5) -and ((Select-String -Path $lc -Pattern 'shielded-history=on' -ErrorAction SilentlyContinue).Count -eq 2)
if ($lok) { Write-Host "[PASS] launcher      run-zkas-node.cmd intact (5 lines, flag x2)" -ForegroundColor Green }
else { $fails++; Write-Host "[FAIL] launcher      run-zkas-node.cmd MISSING/ALTERED - restore from STARTUP-ORDER/repo before any node restart" -ForegroundColor Red }

# 1 kaspad (KAS leg, wRPC 17110)
$p = Port 17110
if ($p) { Write-Host "[PASS] kaspad        pid=$p" -ForegroundColor Green }
else { $fails++; Write-Host '[FAIL] kaspad DOWN   start (cmd): C:\rusty-kaspa-v2\target\release\kaspad.exe --configfile "C:\Node-v2\config.toml"' -ForegroundColor Red }

# 2 zkas-node v1.0.6 - identity-checked: path + sha256 + flag (impostor detector)
$p = Port 16811
if (-not $p) { $fails++; Write-Host '[FAIL] zkas-node DOWN start (cmd): C:\zkas\node-v106\run-zkas-node.cmd' -ForegroundColor Red }
else {
  $w = Get-CimInstance Win32_Process -Filter "ProcessId=$p"
  $h = (Get-FileHash $w.ExecutablePath -Algorithm SHA256).Hash
  if (($w.ExecutablePath -ieq 'C:\zkas\node-v106\zkas-node.exe') -and ($h -eq '1B49D1FA5416130A6CB82A166E5941E778EE1266E8BD5ACB23EA810B01DC97D2') -and ($w.CommandLine -match 'shielded-history=on')) {
    Write-Host "[PASS] zkas-node     pid=$p  v1.0.6 pinned, flag on" -ForegroundColor Green
  } else { $fails++; Write-Host "[FAIL] zkas-node IMPOSTOR pid=$p ($($w.ExecutablePath)) - Stop-Process -Id $p -Force, then start (cmd): C:\zkas\node-v106\run-zkas-node.cmd" -ForegroundColor Red }
}

# 3 stratum-bridge - three ports, ONE process (BL-039)
$b = @(5755,5765,3034 | ForEach-Object { Port $_ } | Sort-Object -Unique)
if ($b.Count -eq 1 -and $b[0]) { Write-Host "[PASS] bridge        pid=$($b[0])  owns 5755+5765+3034" -ForegroundColor Green }
else { $fails++; Write-Host "[FAIL] bridge DOWN/SPLIT (pids: $($b -join ',')) start (cmd): C:\Users\inmyh\zkas-rusty-kd\run-rc-merged.cmd  - verify BOTH MERGED lines in header" -ForegroundColor Red }

# 4 walletd (start only after zkas-node is healthy)
$p = Port 8501
if ($p) { Write-Host "[PASS] walletd       pid=$p" -ForegroundColor Green }
else { $fails++; Write-Host '[FAIL] walletd DOWN  start (cmd): powershell -NoProfile -ExecutionPolicy Bypass -File C:\zkas\node\start-walletd-v1.0.5-r1.ps1' -ForegroundColor Red }

# 5 alertmanager (start BEFORE prometheus; alertmanager.yml is cwd-relative)
$p = Port 9093
if ($p) { Write-Host "[PASS] alertmanager  pid=$p" -ForegroundColor Green }
else { $fails++; Write-Host '[FAIL] alertmanager DOWN start (cmd): cd /d C:\Prometheus\alertmanager THEN: alertmanager.exe --cluster.listen-address=""' -ForegroundColor Red }

# 6 prometheus (TSDB is cwd-relative .\data - wrong cwd = silent empty history)
$p = Port 9090
if ($p) { Write-Host "[PASS] prometheus    pid=$p" -ForegroundColor Green }
else { $fails++; Write-Host '[FAIL] prometheus DOWN start (cmd): cd /d C:\Prometheus THEN: prometheus.exe --config.file="C:\Prometheus\prometheus.yml" --web.enable-lifecycle' -ForegroundColor Red }

# 7 ZkasReporter - liveness off the WORK: :9151 + log motion (BL-067)
$rl = [bool](Get-NetTCPConnection -LocalPort 9151 -State Listen -ErrorAction SilentlyContinue)
$logAge = if (Test-Path C:\zkas\reporter.log) { [int]((Get-Date) - (Get-Item C:\zkas\reporter.log).LastWriteTime).TotalMinutes } else { -1 }
if ($rl -and $logAge -ge 0 -and $logAge -lt 90) { Write-Host "[PASS] reporter      9151 up, log ${logAge}m old" -ForegroundColor Green }
else { $fails++; Write-Host "[FAIL] reporter      9151=$rl log-age=${logAge}m (expect <90m) - restart TASK-TIER: Stop-ScheduledTask ZkasReporter; Start-ScheduledTask ZkasReporter" -ForegroundColor Red }

if ($fails -eq 0) { Write-Host ""; Write-Host "ALL 8 UP - stack correct" -ForegroundColor Green }
else { Write-Host ""; Write-Host "$fails FAILURE(S) - follow the printed start lines in the order listed at top" -ForegroundColor Yellow }
