# =============================================================================
# zkas-reporter.ps1 — zKAS block sidecar reporter (WS7, sidecar architecture)
#
# Tails the RC bridge's log, reports blue-confirmed zKAS blocks to the KDSM
# dashboard webhook in two beats on one dedup key (H_zk):
#   beat 1  (~T+5s)  : hash + worker + found_at + provisional amount
#   beat 2  (~T+60s) : same hash, exact amount from walletd history
#
# Self-healing: on every start it replays the newest log file from byte 0;
# upsert-on-hash makes replay idempotent. Zero bridge changes required.
#
# Run:      powershell -ExecutionPolicy Bypass -File C:\zkas\zkas-reporter.ps1
# Dry run:  ... -DryRun            (parse + print, no POSTs)
# One-shot: ... -ReplayOnly        (replay current log, then exit)
#
# Requires: C:\zkas\webhook-secret.txt  (one line; must match the edge
#           function's ZKAS_WEBHOOK_SECRET, see runbook)
# =============================================================================
param(
    [switch]$DryRun,
    [switch]$ReplayOnly
)

# ----------------------------- configuration --------------------------------
$LogDir        = 'C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs'
$WebhookUrl    = 'https://drfqooqkicdhhwkjjaez.supabase.co/functions/v1/zkas-block-webhook'
$SecretFile    = 'C:\zkas\webhook-secret.txt'
$WalletdUrl    = 'http://127.0.0.1:8501'
$WalletToken   = '32b6c197dea4f058e'
$StateFile     = 'C:\zkas\reporter-state.json'
$ReporterLog   = 'C:\zkas\reporter.log'
$HeartbeatFile = 'C:\zkas\reporter-heartbeat.txt'

$TailIntervalSec    = 3      # log poll cadence
$MetricsPort        = 9151   # loopback /metrics for Prometheus (one-time, elevated:
                             #   netsh http add urlacl url=http://127.0.0.1:9151/ user=inmyh )
$WalletPollSec      = 30     # walletd history poll cadence (only while beats pending)
$MatchWindowLoSec   = -5     # chain ts may precede found ts by up to 5s (clock skew)
$MatchWindowHiSec   = 180    # chain ts may lag found ts by up to 3 min
$Beat2GiveUpSec     = 3600   # stop trying to refine after 1h; leave provisional
$PostRetryMinSec    = 30     # min seconds between retries of a failed POST

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

# ----------------------------- plumbing --------------------------------------
function Log([string]$msg) {
    $line = "{0:yyyy-MM-dd HH:mm:ss.fff}  {1}" -f (Get-Date), $msg
    Write-Host $line
    Add-Content -Path $ReporterLog -Value $line -Encoding utf8
}

function Load-State {
    if (Test-Path $StateFile) {
        try { return Get-Content $StateFile -Raw | ConvertFrom-Json } catch { Log "WARN state file unreadable, starting fresh: $_" }
    }
    return [pscustomobject]@{ blocks = [pscustomobject]@{} }
}
function Save-State($state) {
    $state | ConvertTo-Json -Depth 5 | Set-Content -Path $StateFile -Encoding utf8
}

$Secret = $null
if (Test-Path $SecretFile) { $Secret = (Get-Content $SecretFile -Raw).Trim() }
if (-not $Secret) { Log "WARN no webhook secret at $SecretFile - POSTs will be rejected once the edge function enforces it" }

$script:PostFailures = 0

function Post-Block($payload) {
    if ($DryRun) { Log ("DRYRUN POST " + ($payload | ConvertTo-Json -Compress)); return $true }
    $headers = @{ 'Content-Type' = 'application/json' }
    if ($Secret) { $headers['X-Webhook-Secret'] = $Secret }
    try {
        $r = Invoke-RestMethod -Uri $WebhookUrl -Method Post -Headers $headers `
             -Body ($payload | ConvertTo-Json -Compress) -TimeoutSec 15
        if ($r.ok) { return $true }
        $script:PostFailures++
        Log ("WARN webhook non-ok: " + ($r | ConvertTo-Json -Compress)); return $false
    } catch {
        $script:PostFailures++
        Log "WARN webhook POST failed: $($_.Exception.Message)"; return $false
    }
}

function Get-WalletHistory([int]$limit) {
    try {
        return Invoke-RestMethod -Uri "$WalletdUrl/api/wallet/history?limit=$limit&offset=0" `
               -Headers @{ 'X-Wallet-Token' = $WalletToken } -TimeoutSec 10
    } catch {
        Log "WARN walletd history poll failed: $($_.Exception.Message)"; return $null
    }
}

# provisional amount = the most recent coinbase amount walletd has seen.
# Self-updating across subsidy steps; never a hardcoded constant (BL-028).
$script:ProvisionalAmt = 0
function Update-ProvisionalAmount {
    $h = Get-WalletHistory 5
    if ($h -and $h.rows) {
        $cb = $h.rows | Where-Object { $_.kind -eq 'coinbase' } | Select-Object -First 1
        if ($cb) { $script:ProvisionalAmt = [double]$cb.amountZkas }
    }
}

function Newest-LogFile {
    Get-ChildItem "$LogDir\RKStratum_*.log" -ErrorAction SilentlyContinue |
        Sort-Object { [long]($_.BaseName -replace 'RKStratum_','') } |
        Select-Object -Last 1
}

# read new complete lines from a file past $offset without holding a handle;
# returns @{ lines = [string[]]; newOffset = [long] }
function Read-NewLines([string]$path, [long]$offset) {
    $fs = [IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read,
          ([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
    try {
        if ($fs.Length -le $offset) { return @{ lines = @(); newOffset = $offset } }
        $fs.Seek($offset, 'Begin') | Out-Null
        $len = $fs.Length - $offset
        $buf = New-Object byte[] $len
        $read = $fs.Read($buf, 0, $len)
        $text = [Text.Encoding]::UTF8.GetString($buf, 0, $read)
        $lastNl = $text.LastIndexOf("`n")
        if ($lastNl -lt 0) { return @{ lines = @(); newOffset = $offset } }  # partial line only
        $complete = $text.Substring(0, $lastNl + 1)
        $consumed = [Text.Encoding]::UTF8.GetByteCount($complete)
        return @{ lines = ($complete -split "`r?`n" | Where-Object { $_ }); newOffset = $offset + $consumed }
    } finally { $fs.Dispose() }
}

# ----------------------------- parsing ---------------------------------------
$FoundRe = [regex]'^(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d+)([+-]\d\d:\d\d) .*ZKAS BLOCK FOUND! H_fc: ([0-9a-f]{64}), Worker: (\S+), full_clear: (true|false)'
$BlueRe  = [regex]'ZKas block confirmed BLUE! H_fc: ([0-9a-f]{64})'

# hash -> @{ w; foundEpoch; foundIso }
$FoundTable = @{}

function Process-Line([string]$line, $state) {
    $m = $FoundRe.Match($line)
    if ($m.Success) {
        $dto = [DateTimeOffset]::Parse(($m.Groups[1].Value -replace ' ','T') + $m.Groups[2].Value)
        $FoundTable[$m.Groups[3].Value] = @{
            w = $m.Groups[4].Value
            foundEpoch = $dto.ToUnixTimeMilliseconds() / 1000.0
            foundIso = $dto.ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'")
        }
        return
    }
    $b = $BlueRe.Match($line)
    if ($b.Success) {
        $h = $b.Groups[1].Value
        if (-not $FoundTable.ContainsKey($h)) { Log "WARN BLUE for unknown hash $($h.Substring(0,12)) - no FOUND line seen"; return }
        if (-not $state.blocks.$h) {
            $f = $FoundTable[$h]
            $state.blocks | Add-Member -NotePropertyName $h -NotePropertyValue ([pscustomobject]@{
                w = $f.w; foundEpoch = $f.foundEpoch; foundIso = $f.foundIso
                b1 = $false; b2 = $false; lastTry = 0; firstSeen = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
            }) -Force
            Log "BLOCK blue-confirmed $($h.Substring(0,12)) worker=$($f.w) found=$($f.foundIso)"
        }
    }
}

# ----------------------------- beats -----------------------------------------
function Run-Beats($state, $usedTxids) {
    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $pendingB2 = @()
    foreach ($h in @($state.blocks.PSObject.Properties.Name)) {
        $blk = $state.blocks.$h
        if (-not $blk.b1) {
            if (($now - $blk.lastTry) -lt $PostRetryMinSec) { continue }
            $blk.lastTry = $now
            $ok = Post-Block @{
                block_hash = $h; miner_name = $blk.w; found_at = $blk.foundIso
                amount = $script:ProvisionalAmt
            }
            if ($ok) { $blk.b1 = $true; Log "BEAT1 sent $($h.Substring(0,12)) amt(prov)=$($script:ProvisionalAmt)" }
        }
        if ($blk.b1 -and -not $blk.b2) {
            if (($now - $blk.firstSeen) -gt $Beat2GiveUpSec) {
                $blk.b2 = $true; Log "WARN BEAT2 gave up on $($h.Substring(0,12)) - provisional amount stands"
            } else { $pendingB2 += $h }
        }
    }
    if ($pendingB2.Count -eq 0) { return }
    # order-preserving greedy: match oldest block first so near-simultaneous
    # blocks (observed as close as 2s apart) cannot cross-claim each other's
    # payout row - same consumption discipline the 653/653 backfill join used
    $pendingB2 = @($pendingB2 | Sort-Object { $state.blocks.$_.foundEpoch })

    $hist = Get-WalletHistory ([Math]::Min(50, $pendingB2.Count * 3 + 10))
    if (-not $hist -or -not $hist.rows) { return }
    $cbRows = @($hist.rows | Where-Object { $_.kind -eq 'coinbase' -and -not $usedTxids.ContainsKey($_.txid) })
    # Surplus guard: the treasury also receives coinbase income that is NOT
    # ours (third-party miner on the published launcher address, paid as exact
    # N-multiples of the subsidy band). Only single-block payments may match;
    # anything >=1.5x the current band is logged and excluded.
    if ($script:ProvisionalAmt -gt 0) {
        foreach ($sr in @($cbRows | Where-Object { [double]$_.amountZkas -ge ($script:ProvisionalAmt * 1.5) })) {
            if (-not $usedTxids.ContainsKey($sr.txid)) {
                $usedTxids[$sr.txid] = 'surplus'
                Log "WARN surplus treasury income (not our block): $($sr.amountZkas) zKAS txid=$($sr.txid.Substring(0,12)) - excluded from matching"
            }
        }
        $cbRows = @($cbRows | Where-Object { [double]$_.amountZkas -lt ($script:ProvisionalAmt * 1.5) })
    }

    foreach ($h in $pendingB2) {
        $blk = $state.blocks.$h
        $best = $null
        foreach ($r in $cbRows) {
            if ($usedTxids.ContainsKey($r.txid)) { continue }
            $d = ($r.timestamp / 1000.0) - $blk.foundEpoch
            if ($d -ge $MatchWindowLoSec -and $d -le $MatchWindowHiSec) {
                if (-not $best -or [Math]::Abs($d) -lt [Math]::Abs($best.d)) { $best = @{ r = $r; d = $d } }
            }
        }
        if ($best) {
            $amt = [double]$best.r.amountZkas
            $ok = Post-Block @{
                block_hash = $h; miner_name = $blk.w; found_at = $blk.foundIso; amount = $amt
            }
            if ($ok) {
                $blk.b2 = $true
                $usedTxids[$best.r.txid] = $h
                Log ("BEAT2 sent $($h.Substring(0,12)) amt=$amt dt={0:N1}s txid=$($best.r.txid.Substring(0,12))" -f $best.d)
            }
        }
    }
}

# ----------------------------- metrics (Prometheus) --------------------------
$script:Listener = $null
function Start-MetricsListener {
    try {
        $script:Listener = [System.Net.HttpListener]::new()
        $script:Listener.Prefixes.Add("http://127.0.0.1:$MetricsPort/")
        $script:Listener.Start()
        $script:MetricsCtx = $script:Listener.BeginGetContext($null, $null)
        Log "metrics listener on http://127.0.0.1:$MetricsPort/metrics"
    } catch {
        $script:Listener = $null
        Log "WARN metrics listener failed - run ONCE elevated: netsh http add urlacl url=http://127.0.0.1:$MetricsPort/ user=$env:USERNAME  ($($_.Exception.Message))"
    }
}

function Serve-Metrics([int]$blocks, [int]$pending) {
    if (-not $script:Listener) { return }
    while ($script:MetricsCtx.IsCompleted) {
        try {
            $ctx = $script:Listener.EndGetContext($script:MetricsCtx)
            $script:MetricsCtx = $script:Listener.BeginGetContext($null, $null)
            $body = ("# TYPE zkas_reporter_blocks_total counter`n" +
                     "zkas_reporter_blocks_total $blocks`n" +
                     "# TYPE zkas_reporter_pending gauge`n" +
                     "zkas_reporter_pending $pending`n" +
                     "# TYPE zkas_reporter_post_failures_total counter`n" +
                     "zkas_reporter_post_failures_total $($script:PostFailures)`n")
            $buf = [Text.Encoding]::UTF8.GetBytes($body)
            $ctx.Response.ContentType = 'text/plain; version=0.0.4'
            $ctx.Response.OutputStream.Write($buf, 0, $buf.Length)
            $ctx.Response.Close()
        } catch {
            Log "WARN metrics serve: $($_.Exception.Message)"
            try { $script:MetricsCtx = $script:Listener.BeginGetContext($null, $null) } catch {}
            break
        }
    }
}

# ----------------------------- main ------------------------------------------
Log "=== zkas-reporter starting (DryRun=$DryRun ReplayOnly=$ReplayOnly) ==="
$state = Load-State
if (-not $state.blocks -or $state.blocks -is [hashtable]) {
    $state | Add-Member -NotePropertyName blocks -NotePropertyValue ([pscustomobject]@{}) -Force
}
$usedTxids = @{}   # txid -> hash, session-scoped; state's b2 flags are the durable record

Update-ProvisionalAmount
Log "provisional amount source: $($script:ProvisionalAmt) zKAS (latest walletd coinbase)"
Start-MetricsListener

$cur = Newest-LogFile
if (-not $cur) { Log "FATAL no RKStratum_*.log in $LogDir"; exit 1 }
$offset = [long]0
Log "tailing $($cur.Name) from byte 0 (startup replay)"

$lastWalletPoll = 0
while ($true) {
    # rotation: a newer file means the bridge restarted
    $newest = Newest-LogFile
    if ($newest.Name -ne $cur.Name) {
        $r = Read-NewLines $cur.FullName $offset          # drain the old file
        foreach ($l in $r.lines) { Process-Line $l $state }
        Log "log rotated: $($cur.Name) -> $($newest.Name)"
        $cur = $newest; $offset = 0
    }

    $r = Read-NewLines $cur.FullName $offset
    foreach ($l in $r.lines) { Process-Line $l $state }
    $offset = $r.newOffset

    $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $needBeats = @($state.blocks.PSObject.Properties.Name | Where-Object {
        -not $state.blocks.$_.b1 -or -not $state.blocks.$_.b2 }).Count
    if ($needBeats -gt 0 -and ($now - $lastWalletPoll) -ge $WalletPollSec) {
        Update-ProvisionalAmount
        $lastWalletPoll = $now
    }
    if ($needBeats -gt 0) { Run-Beats $state $usedTxids; if (-not $DryRun) { Save-State $state } }

    $blockCount = @($state.blocks.PSObject.Properties.Name).Count
    Serve-Metrics $blockCount $needBeats
    Set-Content -Path $HeartbeatFile -Encoding ascii -Value ("{0} blocks={1} pending={2}" -f `
        $now, $blockCount, $needBeats)

    if ($ReplayOnly -and $needBeats -eq 0) { Log "replay complete, exiting (-ReplayOnly)"; break }
    Start-Sleep -Seconds $TailIntervalSec
}
if (-not $DryRun) { Save-State $state }
Log "=== zkas-reporter exiting ==="
