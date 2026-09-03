# NODE-CUTOVER v1.0.8 — r1 · 2026-09-03
### Scope: NODE + WALLETD, both to v1.0.8, one window. Node first (gated), walletd
### canary on a COPY of the wallet dir second (gated), walletd money rail third.
### Bridge untouched; it auto-reconnects (BL-032 blip taxonomy). Fork rebase separate.
### Supersedes NODE-CUTOVER-v1.0.6-r1 as the cutover runbook; that doc stays as the
### v1.0.6 record. Structure and rollback principle carried from it unchanged.
### Pins below were computed CONTAINER-SIDE from the GitHub release asset on 2026-09-03
### — an independent rail. Kron's Get-FileHash must agree or nothing starts.
###   zkas-zkas-v1.0.8-win64.zip  66,954,365 B  334cec3c31754318bca3832aab86fbcd75b9ae341cdcf2df825c2c9c9c7ebf40
###   kaspad.exe (node)           53,185,536 B  45687E24E925C4ED777290C58B3A74B68339C8FA6C84F4E303533FC04652236D
###   zkas-walletd.exe            20,260,352 B  B5B1DDA9093D1FB55A92A76D4D7CFE1ECDC67D57BCFCB0701FBFBAC7EF8C932C
### Rollback identities (read live 2026-09-03, Kron): walletd v1.0.5 = BDCBE0673C800720
### EF33D73EB68A4C6FBEBB10B3CA472E0822B8FDE08063713C @ C:\zkas\node\zkas-walletd.exe.
### Node v1.0.6 exe hash is captured in step 2 (not previously pinned on any rail).
### Config-key trap RE-VERIFIED at args.rs @ zkas-v1.0.8 (793 ln): the SAME five keys
### are CLI-only (shielded-history, verify-shielded-history, consensus-diag,
### shielded-anchor-overrides, externalip). listen/rpclisten/appdir/utxoindex/archival/
### nologfiles/perf-metrics are TOML-honored. NODE-CONTRACT §2 carries forward verbatim.
### Every command names machine + shell. Kron/PowerShell steps run as inmyh, NOT
### elevated (no firewall change in this window — H7 scoping is unchanged, walletd is
### loopback). Cross-step $variables require the SAME PowerShell window; stated per step.

## 0 · GO / NO-GO
- Node healthy: templates flowing, `zk=ok` on the bridge status line.
- No open incident on Kron. **Do not reboot Kron in this window** — the brick
  experiment (BL-087) is armed on 41/6008; process restarts do not produce one,
  a host reboot would confound the falsifier.
- Window: multi-hour, evening preferred so the 12 h acceptance tail (§13) runs
  overnight. Budget: node stop→relaunch ~15 min (datadir copy is the long pole);
  walletd canary ≤10 min; walletd main ≤10 min.
- `perf-metrics = false` is DECIDED, not drift: set false by operator decision in
  the PRF0 session (09-01) — output was dying at the INFO filter, the memory
  question it instrumented closed at BL-081 as a configured equilibrium, and the
  exporter's process collector covers RSS with no restart. The TOML header's
  "Deltas: perf-metrics on" is the stale line. Step 5 copies the key VERBATIM.
- Expected alert noise: TG "template age >30s" while the node is down; reporter
  `WARN beats DEFERRED` while walletd is down or warming. Neither is actionable.
- Ports are already where v1.0.8 defaults them: `listen = 0.0.0.0:16811` in the
  TOML, PID-verified 2026-09-03 (node 16811/16810, kaspad 16111/16110, walletd
  127.0.0.1:8501). The 1.0.8 default move is a no-op here.

## 1 · Treasury token (Kron · PowerShell — THIS window is used through step 12)
A value only the operator holds; it is its own step so no later fence carries a
placeholder. Not echoed.
```powershell
$tok = Read-Host 'treasury X-Wallet-Token'; "token length: $($tok.Length)"
```
Expected: a length, no token on screen.

## 2 · Pre-window captures + rollback identities (Kron · PowerShell, same window)
```powershell
"=== node v1.0.6 exe (rollback identity) ==="; (Get-FileHash C:\zkas\node-v106\zkas-node.exe -Algorithm SHA256).Hash; "=== walletd v1.0.5 exe (must read BDCBE067...) ==="; (Get-FileHash C:\zkas\node\zkas-walletd.exe -Algorithm SHA256).Hash; "=== walletd status pre-window ==="; try { (Invoke-RestMethod -Uri 'http://127.0.0.1:8501/api/wallet/status' -Headers @{ 'X-Wallet-Token' = $tok } -TimeoutSec 10 | ConvertTo-Json -Compress) } catch { "status endpoint: $($_.Exception.Message)" }; "=== walletd history pre-window ==="; (Invoke-RestMethod -Uri 'http://127.0.0.1:8501/api/wallet/history?limit=1&offset=0' -Headers @{ 'X-Wallet-Token' = $tok } -TimeoutSec 10 | ConvertTo-Json -Compress); "=== wallet dir ==="; Get-ChildItem C:\zkas\wallets -Recurse -File | Measure-Object Length -Sum | Select-Object Count,Sum; "=== listeners baseline ==="; Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 16110,16111,16810,16811,8501,8502 } | Select-Object LocalPort,OwningProcess | Sort-Object LocalPort
```
Expected: two hashes (walletd's = `BDCBE067…713C`, else STOP — the running binary is
not the pinned one); a status JSON — RECORD `balance_sompi` and `note_count`, they are
the canary's acceptance values (if v1.0.5 has no status endpoint, the history row's
`amountZkas`/`kind` is the fallback comparison); wallet dir file count + bytes; five
listeners, none on 8502.

## 3 · Download + pin (Kron · PowerShell)
Silence: ~67 MB, seconds to a minute; curl prints a progress meter.
```powershell
New-Item -ItemType Directory -Force C:\zkas\node-v108, C:\zkas\walletd-v108 | Out-Null; curl.exe -L -o C:\zkas\node-v108\zkas-zkas-v1.0.8-win64.zip https://github.com/firecash/zkas-rusty/releases/download/zkas-v1.0.8/zkas-zkas-v1.0.8-win64.zip; (Get-Item C:\zkas\node-v108\zkas-zkas-v1.0.8-win64.zip).Length; (Get-FileHash C:\zkas\node-v108\zkas-zkas-v1.0.8-win64.zip -Algorithm SHA256).Hash
```
Expected: `66954365` and `334CEC3C31754318BCA3832AAB86FBCD75B9AE341CDCF2DF825C2C9C9C7EBF40`.
**HARD STOP on mismatch.** Steps 4–12 gated on this line.

## 4 · Extract + stage + pin both binaries (Kron · PowerShell)
Node keeps the `zkas-node.exe` naming (a copy of `kaspad.exe`, as in v1.0.6, so the
launcher and process names stay distinguishable from the real kaspad). Walletd goes
to its own versioned dir; `C:\zkas\node\zkas-walletd.exe` stays untouched as v1.0.5.
```powershell
Expand-Archive C:\zkas\node-v108\zkas-zkas-v1.0.8-win64.zip -DestinationPath C:\zkas\node-v108 -Force; Copy-Item C:\zkas\node-v108\kaspad.exe C:\zkas\node-v108\zkas-node.exe; Copy-Item C:\zkas\node-v108\zkas-walletd.exe C:\zkas\walletd-v108\zkas-walletd.exe; Get-FileHash C:\zkas\node-v108\kaspad.exe, C:\zkas\node-v108\zkas-node.exe, C:\zkas\walletd-v108\zkas-walletd.exe -Algorithm SHA256 | Select-Object Hash, Path
```
Expected: first two `45687E24…2236D`, third `B5B1DDA9…932C`. **HARD STOP on mismatch.**

## 5 · Config + node launcher (Kron · PowerShell)
TOML copied verbatim (header line only changed) and written BOM-free — a BOM in a
TOML is a parse failure the node reports as a missing key. Launcher copied from
v106 with the two version tokens patched; `cd /d %~dp0` keeps `zkas-node.exe`
relative, so the exe path needs no edit.
```powershell
$t = [IO.File]::ReadAllText('C:\zkas\node\zkas-node-v106.toml'); $n = ($t -split "`r?`n" | Select-String -Pattern 'v1\.0\.6 config').Count; $t2 = $t -replace 'ZKas node v1\.0\.6 config - carries zkas-node-758k\.toml forward\. Deltas: perf-metrics on\.', 'ZKas node v1.0.8 config - copied VERBATIM from zkas-node-v106.toml 2026-09-03; only this header line changed. perf-metrics=false is DECIDED (PRF0 09-01, BL-081), not a delta to reverse.'; [IO.File]::WriteAllText('C:\zkas\node\zkas-node-v108.toml', $t2, (New-Object System.Text.UTF8Encoding $false)); "header anchors matched: $n (expect 1)"; "--- v108 toml key lines ---"; Select-String -Path C:\zkas\node\zkas-node-v108.toml -Pattern '^(appdir|listen|rpclisten|archival|utxoindex|nologfiles|perf-metrics|externalip)' | ForEach-Object { $_.Line }; "--- diff v106 vs v108 (expect ONE line) ---"; Compare-Object (Get-Content C:\zkas\node\zkas-node-v106.toml) (Get-Content C:\zkas\node\zkas-node-v108.toml) | ForEach-Object { "$($_.SideIndicator) $($_.InputObject)" }
```
Expected: `matched: 1`; the eight key lines unchanged from v106 (`listen = "0.0.0.0:16811"`,
`archival = true`, …); Compare-Object shows exactly two lines (the old header `<=`, the
new `=>`). Any other diff line: STOP, the copy is not verbatim.
```powershell
$c = Get-Content C:\zkas\node-v106\run-zkas-node.cmd -Raw; $c2 = $c -replace 'zkas-node-v106\.toml', 'zkas-node-v108.toml' -replace 'v1\.0\.6 launcher', 'v1.0.8 launcher (copied from the previous launcher 2026-09-03; flags unchanged incl. externalip)'; [IO.File]::WriteAllText('C:\zkas\node-v108\run-zkas-node.cmd', $c2, (New-Object System.Text.ASCIIEncoding)); Get-Content C:\zkas\node-v108\run-zkas-node.cmd; "--- v106 tokens remaining (expect 0) ---"; (Select-String -Path C:\zkas\node-v108\run-zkas-node.cmd -Pattern 'v106|v1\.0\.6').Count
```
Expected: the launcher text with `--configfile C:\zkas\node\zkas-node-v108.toml
--shielded-history=on --externalip=108.95.94.128:16811`; remaining count `0`.

## 6 · Walletd launchers — main + canary (Kron · PowerShell)
Both derive from `start-walletd-v1.0.5-r1.ps1` by anchored replacement, so every
guard (already-listening, sha, secret-present), the DPAPI handoff and the log capture
carry unchanged. Main: exe + sha + version tokens. Canary additionally: port 8502,
wallet dir `C:\zkas\wallets-canary-v108`, log prefix, and **two spend guards** —
`--no-auto-consolidate` (background consolidation is a broadcast; a copy holds the
same seeds) and `--no-custodial` (seed-holding endpoints 403). Never launch the canary
without the first; the second is belt-and-braces and has a documented fallback.
```powershell
$s = Get-Content C:\zkas\node\start-walletd-v1.0.5-r1.ps1 -Raw; $m = $s -replace "\`$Exe        = 'C:\\zkas\\node\\zkas-walletd\.exe'", "`$Exe        = 'C:\zkas\walletd-v108\zkas-walletd.exe'" -replace "\`$ExeSha     = 'BDCBE0673C800720EF33D73EB68A4C6FBEBB10B3CA472E0822B8FDE08063713C'", "`$ExeSha     = 'B5B1DDA9093D1FB55A92A76D4D7CFE1ECDC67D57BCFCB0701FBFBAC7EF8C932C'" -replace 'start-walletd-v1\.0\.5-r1\.ps1 - SOLE start path', 'start-walletd-v1.0.8-r1.ps1 - SOLE start path (v1.0.8 cutover 2026-09-03; derived from v1.0.5-r1 by anchored replacement: exe path + sha only)' -replace 'set-walletd-secret-r1\.ps1 first', 'set-walletd-secret-r2.ps1 first'; Set-Content -Path C:\zkas\node\start-walletd-v1.0.8-r1.ps1 -Value $m -Encoding UTF8 -NoNewline; "--- main launcher: new anchors present (expect 1 each) ---"; (Select-String -Path C:\zkas\node\start-walletd-v1.0.8-r1.ps1 -Pattern 'walletd-v108\\zkas-walletd\.exe').Count; (Select-String -Path C:\zkas\node\start-walletd-v1.0.8-r1.ps1 -Pattern 'B5B1DDA9093D1FB55A92A76D4D7CFE1ECDC67D57BCFCB0701FBFBAC7EF8C932C').Count; "--- old anchors remaining (expect 0) ---"; (Select-String -Path C:\zkas\node\start-walletd-v1.0.8-r1.ps1 -Pattern 'BDCBE0673C800720|node\\zkas-walletd\.exe').Count
```
Expected: `1`, `1`, `0`.
```powershell
$s = Get-Content C:\zkas\node\start-walletd-v1.0.8-r1.ps1 -Raw; $k = $s -replace 'start-walletd-v1\.0\.8-r1\.ps1 - SOLE start path', 'start-walletd-v1.0.8-canary-r1.ps1 - CANARY on a COPY of the wallet dir, port 8502, SPEND-DISABLED (--no-auto-consolidate --no-custodial). Stop and delete the canary dir after acceptance' -replace "\`$ListenPort = 8501", "`$ListenPort = 8502" -replace "'C:\\zkas\\wallets'", "'C:\zkas\wallets-canary-v108'" -replace '"walletd-\$stamp\.', '"walletd-canary-$stamp.' -replace "'--allow-origin',  'http://localhost:5173'", "'--allow-origin',  'http://localhost:5173',`r`n    '--no-auto-consolidate',`r`n    '--no-custodial'"; Set-Content -Path C:\zkas\node\start-walletd-v1.0.8-canary-r1.ps1 -Value $k -Encoding UTF8 -NoNewline; "--- canary anchors (expect 1,1,2,2,2,1) ---"; foreach ($p in 'ListenPort = 8502', 'wallets-canary-v108', 'walletd-canary-\$stamp', '--no-auto-consolidate', '--no-custodial', 'walletd-v108\\zkas-walletd\.exe') { (Select-String -Path C:\zkas\node\start-walletd-v1.0.8-canary-r1.ps1 -Pattern $p).Count }; "--- money-rail dir referenced in canary (expect 0) ---"; (Select-String -Path C:\zkas\node\start-walletd-v1.0.8-canary-r1.ps1 -Pattern "'C:\\zkas\\wallets'").Count
```
Expected: `1 1 2 2 2 1` then `0`. (Log prefix reads 2: out + err log lines. Both spend
guards read 2: the header line and the argList line.) A `0` on any spend guard is a HARD STOP — do not run the canary.

## 7 · Stop walletd (Kron · PowerShell, same window)
Reporter goes `WARN beats DEFERRED` from here until step 12 — expected.
```powershell
$w = Get-NetTCPConnection -LocalPort 8501 -State Listen -ErrorAction SilentlyContinue; if ($w) { Stop-Process -Id $w[0].OwningProcess; Start-Sleep 3 }; "8501 listeners after stop (expect 0): $((Get-NetTCPConnection -LocalPort 8501 -State Listen -ErrorAction SilentlyContinue | Measure-Object).Count)"
```

## 8 · Wallet dir cold copies — rollback AND canary (Kron · PowerShell, walletd STOPPED)
Two copies from a quiescent dir: `backup\wallets-pre-v108` is the ROLLBACK and is
never touched again this window; `wallets-canary-v108` is what the canary opens.
```powershell
robocopy C:\zkas\wallets C:\zkas\backup\wallets-pre-v108 /MIR /R:1 /W:1 | Select-Object -Last 8; robocopy C:\zkas\wallets C:\zkas\wallets-canary-v108 /MIR /R:1 /W:1 | Select-Object -Last 8; "--- identity: every file hash equal across the three dirs (expect 0 differing) ---"; $a = Get-ChildItem C:\zkas\wallets -Recurse -File | ForEach-Object { (Get-FileHash $_.FullName -Algorithm SHA256).Hash }; $b = Get-ChildItem C:\zkas\backup\wallets-pre-v108 -Recurse -File | ForEach-Object { (Get-FileHash $_.FullName -Algorithm SHA256).Hash }; $c = Get-ChildItem C:\zkas\wallets-canary-v108 -Recurse -File | ForEach-Object { (Get-FileHash $_.FullName -Algorithm SHA256).Hash }; ((Compare-Object $a $b) + (Compare-Object $a $c) | Measure-Object).Count
```
Expected: robocopy summaries with `FAILED 0`; final count `0`.

## 9 · Stop node + datadir cold copy (Kron)
9a. In the cmd window running `run-zkas-node.cmd`: Ctrl+C, wait for the process to
exit (the log's last lines show a clean shutdown). Then confirm from PowerShell:
```powershell
"node listeners after stop (expect 0): $((Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 16810,16811 } | Measure-Object).Count)"
```
9b. Datadir cold copy — THE rollback path across the layout boundary. 38.4 GB
(read 2026-09-03), 643 GB free. **SILENCE PROFILE: one line per file, several
thousand files, 5–15 min depending on disk.** Do not interrupt.
```powershell
robocopy C:\zkas\node-data C:\zkas\backup\node-data-pre-v108 /MIR /R:1 /W:1 | Select-Object -Last 10; "--- identity: file count + bytes both sides ---"; Get-ChildItem C:\zkas\node-data -Recurse -File | Measure-Object Length -Sum | Select-Object Count,Sum; Get-ChildItem C:\zkas\backup\node-data-pre-v108 -Recurse -File | Measure-Object Length -Sum | Select-Object Count,Sum
```
Expected: `FAILED 0`; Count and Sum identical on both lines. **Step 10 gated on this.**

## 10 · Launch v1.0.8 node (Kron · cmd — a NEW console window, keep it open)
```cmd
C:\zkas\node-v108\run-zkas-node.cmd
```
Expected: startup lines, then DB open, then sync/DAA lines. First write is the
layout boundary — from here rollback is §14, never v1.0.6-on-new-data.

## 11 · Node acceptance (Kron · PowerShell)
BL-001 health, two reads ≥60 s apart, tip figures must ADVANCE:
```powershell
Get-ChildItem C:\zkas\node-data -Recurse -Filter *.log | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | Get-Content -Tail 12; "--- listeners (expect node on 16811 + 16810, same PID) ---"; Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 16810,16811 } | Select-Object LocalPort,OwningProcess
```
Bridge-side: status line returns to `zk=ok`, template-age cards resolve. Then the
detector, from the MacBook — the stranded tip is EXPECTED to still be present at this
point; it leaves at the first pruning-point advancement (§13), not at launch:
**MacBook / zsh**
```
cd /Users/pearsonmw/zkas-lab/proto && grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto -d '{"getBlockDagInfoRequest":{}}' 192.168.1.96:16810 protowire.RPC/MessageStream | python3 -c 'import sys,json; r=json.load(sys.stdin)["getBlockDagInfoResponse"]; t=r["tipHashes"]; v=set(r["virtualParentHashes"]); d=[h for h in t if h not in v]; print(r["virtualDaaScore"], "tips="+str(len(t)), "vpar="+str(len(v)), "diff="+(",".join(x[:8] for x in d) or "-"))'
```
Expected now: `diff=e8dc1a03` (baseline still holds). RECORD the DAA — §13's clock
starts here.

## 12 · Walletd — CANARY, then MONEY RAIL (Kron · PowerShell, same window as step 1)
12a. Canary on the copy, port 8502. Node must be up (step 11). **SILENCE PROFILE:**
walletd answers nothing while it builds its subtree cache — ~270 s at v1.0.5, release
notes claim ~4× faster scan and "v8 checkpoints load again (no full rescan)"; budget
5 min, `NOT LISTENING after 60s` from the launcher is EXPECTED and not a failure —
poll the status endpoint until it answers.
```powershell
& C:\zkas\node\start-walletd-v1.0.8-canary-r1.ps1
```
If it exits immediately with an error naming `--no-custodial` (refusing a seed-holding
dir), that is the documented fallback: remove ONLY that flag from the canary script
(`--no-auto-consolidate` stays) and relaunch. Then, once listening:
```powershell
$t0 = Get-Date; do { Start-Sleep 10; try { $st = Invoke-RestMethod -Uri 'http://127.0.0.1:8502/api/wallet/status' -Headers @{ 'X-Wallet-Token' = $tok } -TimeoutSec 10 } catch { $st = $null }; "{0,4}s  answered={1}" -f [int]((Get-Date) - $t0).TotalSeconds, ($null -ne $st) } until ($st -or ((Get-Date) - $t0).TotalSeconds -gt 600); $st | ConvertTo-Json -Compress
```
Expected within ~5 min: a status JSON. ACCEPTANCE (compare to step 2's capture):
`synced` true · `missing_history` false · `warming`/`loading` false · `blocks_behind`
0 or single digits · **`note_count` EQUAL to pre-window** · **`balance_sompi` EQUAL to
pre-window** · `spend_ready` may read false under `--no-custodial` (expected). A
different note_count or balance is a STOP: the wallet file did not load cleanly, and
the money rail is NOT cut over — rollback is unnecessary because nothing moved.
12b. Stop the canary and remove the copy — it holds the same seeds:
```powershell
$cw = Get-NetTCPConnection -LocalPort 8502 -State Listen -ErrorAction SilentlyContinue; if ($cw) { Stop-Process -Id $cw[0].OwningProcess; Start-Sleep 3 }; Remove-Item -Recurse -Force C:\zkas\wallets-canary-v108; "canary dir present (expect False): $(Test-Path C:\zkas\wallets-canary-v108)"; "8502 listeners (expect 0): $((Get-NetTCPConnection -LocalPort 8502 -State Listen -ErrorAction SilentlyContinue | Measure-Object).Count)"
```
12c. Money rail on the real dir, port 8501 — gated on 12a's equalities:
```powershell
& C:\zkas\node\start-walletd-v1.0.8-r1.ps1
```
Same silence profile. Then the same poll on 8501:
```powershell
$t0 = Get-Date; do { Start-Sleep 10; try { $st = Invoke-RestMethod -Uri 'http://127.0.0.1:8501/api/wallet/status' -Headers @{ 'X-Wallet-Token' = $tok } -TimeoutSec 10 } catch { $st = $null }; "{0,4}s  answered={1}" -f [int]((Get-Date) - $t0).TotalSeconds, ($null -ne $st) } until ($st -or ((Get-Date) - $t0).TotalSeconds -gt 600); $st | ConvertTo-Json -Compress; "--- reporter should re-arm: provisional_known ---"; (Invoke-WebRequest -Uri 'http://127.0.0.1:9151/metrics' -UseBasicParsing -TimeoutSec 10).Content -split "`n" | Select-String 'zkas_reporter_provisional_known'
```
Expected: status JSON with `note_count` and `balance_sompi` equal to step 2 and
`spend_ready` true; `zkas_reporter_provisional_known 1` within one reporter poll.
Listeners table now matches step 2's baseline exactly.

## 13 · Acceptance tail — operator-scheduled, ≥12 h after step 11's DAA
The gate that closes H8. Same instrument as the baseline (BL-092, 20/20 with the
tip present):
**MacBook / zsh**
```
cd /Users/pearsonmw/zkas-lab/proto && for i in $(seq 1 20); do grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto -d '{"getBlockDagInfoRequest":{}}' 192.168.1.96:16810 protowire.RPC/MessageStream | python3 -c 'import sys,json; r=json.load(sys.stdin)["getBlockDagInfoResponse"]; t=r["tipHashes"]; v=set(r["virtualParentHashes"]); d=[h for h in t if h not in v]; print(r["virtualDaaScore"], "tips="+str(len(t)), "vpar="+str(len(v)), "diff="+(",".join(x[:8] for x in d) or "-"))'; sleep 15; done | tee ~/zkas-lab/stranded-detector-zkas-post-v108.log
```
PASS: `diff=-` in 20/20 and the node log shows the archival cleanup line (`pruned N
unmergeable side-branch tips`, expect N ≥ 1). Plus, over the same 12 h from
Prometheus: `beat2` p50 in the ~200 s class, `poll_failures` flat, give-ups 0.
FAIL (tip still present after 12 h and one pruning-point advancement): a finding,
not a rollback trigger — the node is otherwise healthy; record and bring it to the
session.

## 14 · ROLLBACK (only path: restore across the layout boundary)
Node: stop v1.0.8 (9a pattern) → `robocopy C:\zkas\backup\node-data-pre-v108
C:\zkas\node-data /MIR /R:1 /W:1` (9b silence profile) → relaunch the v1.0.6 pair
`C:\zkas\node-v106\run-zkas-node.cmd` (its own TOML and flags; identity = step 2's
first hash). Never v1.0.6 binary on v1.0.8 data.
Walletd: stop (7 pattern) → `robocopy C:\zkas\backup\wallets-pre-v108 C:\zkas\wallets
/MIR /R:1 /W:1` → `& C:\zkas\node\start-walletd-v1.0.5-r1.ps1` — its sha guard
enforces `BDCBE067…713C` itself. Never v1.0.5 binary on v1.0.8-written wallet files.
Both halves roll back independently.

## 15 · RECORD AT CLOSE (ledger / session-state)
Node + walletd shas as launched (steps 4, 12c); step 11 DAA (tail clock start); step
2 vs 12c note_count/balance equalities; canary time-to-answer; canary dir deletion
confirmed; perf-metrics decision from §0; §13 result with its log sha; listeners
table post-window. Riders handed on, not executed here: `/api/wallet/warm` +
`warm_wallets` → STARTUP-ORDER r2; NODE-CONTRACT-v1.0.8 (§2 verbatim, §6 walletd
deltas: `--no-custodial`, `--allow-default-token` default OFF, new status fields);
exporter include-regex audit for `zkas-api`/`shielded-pay` if ever launched.
