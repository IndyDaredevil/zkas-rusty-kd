# KRON-HARDENING — Windows 11 Pro as a mining appliance
### Content r2 · drafted 2026-08-26, revised 2026-08-30 · Host: Kron
### (ACEMAGICIAN K1, Win 11 Pro 25H2, build 26200.8875)
### Origin: the 08-22 stall investigation — 18 scrape failures in 2 days traced to
### Windows servicing (a Store retry loop on Microsoft.ScreenSketch) plus unexplained
### night dips. Goal: the host interferes with mining and its instruments ONLY inside
### windows the operator declares.
### Doctrine: every change ships with (a) the mechanism Windows actually respects on
### Pro, (b) an artifact readback gate, and (c) a Prometheus-side acceptance gate.
### "I disabled it" is never the proof; the quiet morning is.

---

## 0. POSITION (decided 08-22, recorded here)

- **Tier 1 (this doc): govern every update/maintenance channel** so activity fires
  only in declared windows. Updates still exist; the schedule is ours.
- **Tier 2 (rejected): full severance** (service ACLs, endpoint blocks). An
  unpatchable box custodying walletd + two network-exposed p2p stacks accumulates
  CVE debt worse than the interference it prevents.
- **Tier 3 (deferred, on record): LTSC/IoT Enterprise** is the correct OS for this
  duty — no Store, no Appraiser, 10-year security-only servicing. Priced as a
  production-host reinstall (meta-principle 8); adopt if/when Kron is rebuilt.
  Standing item, not this doc.

Channels identified on this host (each governed separately — there is no master
switch): OS cumulative updates · Store/MSIX app updates (the ScreenSketch loop) ·
Update Session Orchestrator tasks (USO_UxBroker, SmartRetry) · driver pushes ·
Defender signatures+scans · maintenance tasks (Compatibility Appraiser,
ScheduledDefrag) · Delivery Optimization · w32time.

---

## 1. RETRO-CHECK FIRST (5 min, before changing anything)

The 08-22 session ended with a falsifiable prediction: once ScreenSketch updated
cleanly, the morning grind eras stop. Four days have passed — the acceptance data
already exists. Read it before hardening, so the baseline is honest:

```powershell
Get-WinEvent -FilterHashtable @{LogName='System'; ProviderName='Microsoft-Windows-WindowsUpdateClient'; Id=20} -MaxEvents 5 | Select-Object TimeCreated, @{n='Msg';e={($_.Message -split "`r`n")[0..1] -join ' '}} | Format-List
```
Gate: newest Id-20 (install failure) is 8/22 or older → the ScreenSketch loop is
dead. Newer failures → note the package; §3 will still catch it.

```powershell
$raw = curl.exe -sG http://localhost:9090/api/v1/query_range --data-urlencode 'query=up' --data-urlencode "start=$([DateTimeOffset]::UtcNow.AddDays(-4).ToUnixTimeSeconds())" --data-urlencode "end=$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())" --data-urlencode 'step=60'
$r = ($raw -join "") | ConvertFrom-Json
($r.data.result | Where-Object { $_.metric.job -eq 'rc_merged_bridge' }).values | ForEach-Object { [pscustomobject]@{ Time = [DateTimeOffset]::FromUnixTimeSeconds([long]$_[0]).LocalDateTime; Up = [double]$_[1] } } | Where-Object Up -lt 1 | Sort-Object Time | Format-Table -AutoSize
```
Gate: read the dip list since 08-22. No morning-era dips = prediction held (log
it). Night dips may still appear — they are §7's open thread, not a hardening
failure.

---

## 2. OS UPDATE CHANNEL — notify-only, operator-triggered (Pro group policy)

Elevated PowerShell. Policy keys are the mechanism consumer disables lack; the
orchestrator respects them and does not "heal" them.

```powershell
New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Force | Out-Null
Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name AUOptions -Type DWord -Value 2
Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' -Name NoAutoRebootWithLoggedOnUsers -Type DWord -Value 1
```
`AUOptions=2` = notify before download: nothing downloads or installs until the
operator says so, but the channel stays alive (CVE debt bounded by our own monthly
window, §8). `NoAutoReboot...` = Windows never restarts the box out from under
the fleet on its own.

Artifact gate:
```powershell
Get-ItemProperty 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU' | Select-Object AUOptions, NoAutoRebootWithLoggedOnUsers
```
Pass: 2 / 1. Settings → Windows Update will show "Some settings are managed by
your organization" — that banner is the policy holding, not a problem.

## 3. STORE / MSIX CHANNEL — the one that actually bit us

```powershell
New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\WindowsStore' -Force | Out-Null
Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\WindowsStore' -Name AutoDownload -Type DWord -Value 2
```
`AutoDownload=2` = Store auto-update off, machine-scope (the Store app's own
toggle is per-profile and soft; this is the one the 8 AM pass obeys).

Artifact gate:
```powershell
Get-ItemProperty 'HKLM:\SOFTWARE\Policies\Microsoft\WindowsStore' | Select-Object AutoDownload
```
Pass: 2. Consequence accepted: Store apps update only during §8 windows
(open Store → Downloads → Update all, with capture tools closed — the 0x80073D02
lesson).

## 4. DELIVERY OPTIMIZATION + ACTIVE HOURS (backstops)

```powershell
New-Item -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization' -Force | Out-Null
Set-ItemProperty -Path 'HKLM:\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization' -Name DODownloadMode -Type DWord -Value 0
```
Pass gate: readback shows 0 (HTTP-only, no peer traffic on the mining LAN).
Active hours: Settings → Windows Update → Advanced options → Active hours →
manual, widest span offered. Belt-and-suspenders under §2; costs nothing.

## 5. DEFENDER — exclusions for the hot paths, scans pinned to a dead hour

Defender stays ON (Tier-1 doctrine: this box custodies keys). What changes is
what it inline-scans and when it sweeps. The >100 MB/day RKStratum logs are
currently filtered on every write.

```powershell
Add-MpPreference -ExclusionPath 'C:\Prometheus'
Add-MpPreference -ExclusionPath 'C:\zkas'
Add-MpPreference -ExclusionPath "$env:LOCALAPPDATA\kaspa-stratum-bridge\logs"
Add-MpPreference -ExclusionProcess 'stratum-bridge.exe'
Set-MpPreference -ScanScheduleQuickScanTime 240
```
Adjust the bridge exe name/path and add the kaspad + zkas-node datadirs by their
real paths before running (verify with the running services' command lines —
no guessing paths from memory). `240` = quick scans at 04:00, replacing the
drifting ~5–7 PM slot the event log showed.

Artifact gate:
```powershell
Get-MpPreference | Select-Object ExclusionPath, ExclusionProcess, ScanScheduleQuickScanTime
```
Pass: every path listed, 240 shown. Security note for the doc: exclusions are a
standing trade — anything dropped into an excluded dir is unscanned; these dirs
are operator-controlled only.

## 6. MAINTENANCE TASKS + CLOCK

Named offenders from the 08-22 sweep, disabled by name (BL-031 rails: explicit
names, never wildcards):

```powershell
Disable-ScheduledTask -TaskPath '\Microsoft\Windows\Application Experience\' -TaskName 'Microsoft Compatibility Appraiser'
Disable-ScheduledTask -TaskPath '\Microsoft\Windows\Defrag\' -TaskName 'ScheduledDefrag'
```
(Appraiser: the classic drifting hour-long grind. ScheduledDefrag: retrim on an
NVMe appliance can run in our window instead.) USO tasks (SmartRetry etc.) are
SYSTEM-protected — do not fight them; §2/§3 starve them of work instead.

Artifact gate:
```powershell
Get-ScheduledTask -TaskName 'Microsoft Compatibility Appraiser','ScheduledDefrag' | Select-Object TaskName, State
```
Pass: both `Disabled`.

Clock (the ~30-min step events from 08-22 — every cross-instrument correlation
rides on this):
```powershell
w32tm /query /status
```
Read `Last Successful Sync` + the offset. If steps are seconds-scale, tighten:
```powershell
Set-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Services\W32Time\TimeProviders\NtpClient' -Name SpecialPollInterval -Type DWord -Value 900
Restart-Service w32time
w32tm /resync
```
Gate: `w32tm /query /status` shows sync within the last 15 min and sub-second
offset on later rechecks; Kernel-General 1/24 events stop appearing in pairs
every half hour.

## 6.5 POWER — UPS LOAD SPLIT, POWERPANEL, AND THE READABLE DISCRIMINATOR

Added at r2. Three CyberPower units installed 08-27 (BL-044). This section is
the plan of record for the physical rearrangement; it was designed in BL-048
on 08-27 and lived ONLY in the ledger until r2 — a runbook cannot be followed
from an entry it does not cite.

**Target load split (BL-048; hierarchy over symmetry).** Rigs are
ride-through-OPTIONAL (all seven hard-dropped and self-recovered in every
historical event); custody + network is ride-through-REQUIRED.

- 1500VA #1: KS7 + 2 × KS0  ≈ 760W / 76%
- 1500VA #2: KS7 + 2 × KS0  ≈ 760W / 76%
- 1000VA   : Kron + SG116E switch + aux ONLY (~63W)
- third KS7: surge-only outlet, no battery

Latent fault this corrects: unit 1 measured 1.01kW against a CP1500's ~1000W
inverter — fine on passthrough, trips at the next outage, i.e. the UPS itself
as the would-be next power event. Physics forbids balancing it away
(3 × KS7 ≈ 505W each, so any two on one bank overload by construction). An
operator-proposed three-way balance putting a KS7 with Kron on the 1000VA was
REJECTED on inverter math (95% of a 600W-class inverter) and runtime inversion
(Kron's runtime spent in the rig's first three minutes).

**Artifact gate (post-rebalance):** each unit's PowerPanel load% below 80, and
the 1000VA's estimated runtime read and recorded.

**PowerPanel Personal — installed 08-30, and where it actually records.**
BL-044 declared every future power event a "one-bit diagnosis" via the UPS.
Event eight (08-28) arrived unreadable and BL-054 recorded why. Root cause is
now known and is NOT host configuration: **PowerPanel does not write to the
Windows event log at all.** Event ID 105 will never appear on this host; no
`CyberPower`/`PowerPanel` channel is created.

The real store is the PowerPanel UI's Event Logs view, backed by:

```
C:\Program Files (x86)\CyberPower PowerPanel Personal\assets\PPPE_Db.db
```

SQLite. **Every host-event sweep must include this store** alongside System /
Application / TerminalServices-LSM. Services: `PowerPanel Personal Service`
and `PowerPanel Personal Service Monitor`, both Running / Automatic.

**The exercise is mandatory, not optional.** A ~12-second deliberate wall-plug
pull on 08-30 produced four second-precision rows (Utility Power Failed →
Battery discharging → Utility restored → Battery stopped discharging) and zero
Windows System-log entries. Kron rode it with no process impact. Repeat this
after any UPS change, and after the rebalance:

```powershell
Get-Service | Where-Object { $_.Name -match 'cyber|power.?panel|pwrctl' } | Format-Table Name, Status, StartType -AutoSize
```

Pass: both services Running / Automatic, AND the plug-pull transfer visible in
the PowerPanel Event Logs view. An instrument that has never been exercised is
not an instrument (BL-054); the corollary is that exercising it is also the
only way to learn where it records (BL-058).

**Runtime, measured.** 11 min at 08-30 with 2 KS0 Ultras still on the 1000VA,
battery at 97% and still recharging (so understated). Post-rebalance forecast
~33–40 min — better than 3× by Peukert, but the LOW half of BL-048's 30–60
band, and still short of the one known 45-minute premises outage (08-27).

**Graceful shutdown: DEFERRED, deliberately.** Any runtime-based threshold set
now would be calibrated against a load about to change by two thirds. Set it
only after the rebalance, against a re-measured figure taken at 100% charge.
The argument for setting it at all: the archival zkas-node's RocksDB store is
one of a closed, non-regrowable set, and eight uncontrolled power losses have
been survived so far — which is a record of luck, not of safety.

---

## 7. OPEN THREADS THIS DOC DOES NOT CLOSE

- **Night dips** (01:58, 01:30-era): no suspect from Defender or servicing yet.
  Instruments: the fixed step-3 event sweep on those windows + the 2s probe loop
  overnight. If they survive §2–§6, they get their own investigation.
- **Bridge robustness question** (v2.0.1.5 A-item, sharpened 08-22): why does
  host I/O pressure turn a 230ms render into a ≥25s stall rather than a slow one?
  Host hardening reduces the trigger frequency; the bridge fix removes the
  failure class. Both proceed.
- **UPS install**: DONE 08-27 (×3). Superseded by §6.5 below, which carries the
  load split and the PowerPanel findings. Remaining open thread is the
  graceful-shutdown threshold, which cannot be set until after the rebalance.

## 8. THE MAINTENANCE WINDOW (monthly, operator-declared)

All deferred activity lands here, in order, one window:
1. Announce to self: rigs will ride priority failover; sub-minute flicker normal.
2. Store updates: close capture/vendor apps → Store → Update all → verify no
   Id-20 within the hour.
3. OS updates: Settings → Windows Update → download/install what §2 held back →
   restart deliberately (this restart also resets the bridge's series count —
   the A1-hygiene drip — and clears any in-use servicing state).
4. Relaunch bridge via run-rc-merged.cmd ONLY (BL-017/BL-019: both ENABLED lines
   read on every launch).
5. Post-window gates: four-way bridge identity · `up` clean · scrape_duration
   ~230ms floor · reporter metrics listener line · dashboard live row on next
   block.

## 9. ACCEPTANCE — the whole doc's pass/fail (one week after applying §2–§6)

```powershell
$raw = curl.exe -sG http://localhost:9090/api/v1/query_range --data-urlencode 'query=up' --data-urlencode "start=$([DateTimeOffset]::UtcNow.AddDays(-7).ToUnixTimeSeconds())" --data-urlencode "end=$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())" --data-urlencode 'step=60'
$r = ($raw -join "") | ConvertFrom-Json
($r.data.result | Where-Object { $_.metric.job -eq 'rc_merged_bridge' }).values | ForEach-Object { [pscustomobject]@{ Time = [DateTimeOffset]::FromUnixTimeSeconds([long]$_[0]).LocalDateTime; Up = [double]$_[1] } } | Where-Object Up -lt 1 | Format-Table -AutoSize
```
PASS: zero dips outside declared windows for 7 days, no Id-20 events, no
rpc-elevation eras, scrape_duration floor ~230ms throughout, RcScrapeFlaky
silent. Anything else: the residue is real signal — investigate it, don't
tune it away.

## CHANGE LOG
- 2026-08-26 · r1 · initial draft (post-ScreenSketch investigation). Applies to
  Win 11 Pro 25H2; §2/§3 mechanisms are Pro-specific (policy keys).
- 2026-08-30 · r2 · adds §6.5 POWER: BL-048's UPS load split promoted from the
  ledger into the plan of record; PowerPanel install, its ACTUAL log store
  (PPPE_Db.db — the Windows event log carries nothing), the mandatory
  plug-pull exercise, the measured 11-min runtime, and the deliberate deferral
  of the graceful-shutdown threshold until after the rebalance. §7's "UPS
  install" thread retired to a pointer. Ledgered BL-058.
