# KRON-HARDENING — Windows 11 Pro as a mining appliance
### Content r3 · drafted 2026-08-26, revised 2026-09-02 · Host: Kron
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

## 6.5 POWER — EXECUTED SPLIT, THE CANARY, AND WHAT EACH INSTRUMENT CAN SEE

Rewritten at r3: the rebalance is EXECUTED and measured (09-02, BL-088), the
brick is replaced (BL-087), and this section now records reality, not plan.
House terminology, fixed here: **battery-backed** vs **surge-only** outlet
banks (CyberPower's own labels). "Pass-through" is retired — it collides
with UPS bypass-mode, a different thing.

**Executed map (measured loads):**

- 1500VA #1: KS7 + 2×KS0 battery-backed, **742W** · **w8m (KS7) surge-only
  ← DESIGNATED PREMISES CANARY**
- 1500VA #2: KS7 + 2×KS0 battery-backed, **752W**
- 1000VA   : Kron + SG116E + aux, **52W**, battery-backed (~11W below
  BL-048's ~63W baseline — arrived with the 120W brick; unexplained, benign,
  recorded)

The 80% artifact gate reads honestly: both 1500VAs run **82–84%** of their
900W-class inverters — inside spec for their role (bridging sags and short
cuts), NOT long-runtime protection, and nobody should later mistake them
for it. Canary trade stated deliberately: w8m is the fleet's top producer
(31.1% attribution); correct as instrumentation (a reboot you notice),
priced as a premises event costing its uptime + ramp.

**What each instrument can actually see (scoping, BL-088):** PowerPanel
watches the **1000VA ONLY** — the 1500VAs retain no onboard history and
have no PC attached; they are DARK instruments until NUT lands. Every
"zero PowerPanel rows" verdict in the series is so scoped. A rig-side-only
premises sag is visible ONLY in rig uptime counters — the canary's real
coverage.

**Brick (BL-087):** PERFEIDY 19V/6.3A/120W in service since 14:12:49 09-02;
wiggle-tested; old AS0651 (65W, ran ~87% sustained) labeled and RETAINED as
evidence. **Experiment armed with its falsifier: another 41/6008 with zero
PowerPanel rows convicts barrel-or-board and acquits the brick.** Quiet is
weak evidence — the series has shown 10+ quiet days before.

**Decided architecture (BL-088), delivery-gated:**

1. **Gateway → 1000VA** (fate-sharing with Kron is the architecture — a
   separate gateway UPS would create alive-but-blind mismatch windows).
   Gated on a 1st-floor→basement cable drop; executes in the UPS-expansion
   window. Interim exposure known and playbooked: premises loss = both
   nodes peerless (frozen templates) + deadman path dead (host-down page
   for a healthy host); decode via PowerPanel row + port census.
2. **End-state: each KS7 on its own 1500VA** with 1–2 KS0s (one unit
   carries KS7+2×KS0 ~83% — four KS0s do not divide by three). All seven
   battery-backed; **the canary role RETIRES**, succeeded by NUT + the
   gateway move. Third 1500VA ORDERED.
3. **NUT witness node** (Pi 4 kit + high-endurance SD, ORDERED): all three
   UPSs over USB → NUT (`usbhid-ups`) → nut_exporter → the existing
   Prometheus/Telegram rail. Monitoring-only, no shutdown authority.
   Identical-unit discrimination: serial-match in ups.conf, udev port-path
   fallback if serials are blank. **DRILL REQUIRED before it counts
   (BL-054): three input-pulls, three witnessed Telegram alerts.** Charter
   beyond NUT, one item at a time: off-host deadman leg (immune to §6.7's
   session semantics), rig-canary exporter (the seven UIs below, polled),
   WAN-continuity probe, off-Kron backup landing. Firm NOs: nothing
   mining-critical, no node, no LAN-critical service, nothing
   inbound-exposed — the box's value IS its independence.

**PowerPanel Personal — where it actually records (unchanged from r2).**
No Windows event log presence; the store is the UI's Event Logs view backed
by `C:\Program Files (x86)\CyberPower PowerPanel Personal\assets\PPPE_Db.db`
(SQLite). Every host-event sweep includes it. Exercise after any UPS
change: plug-pull → four second-precision rows → services check:

```powershell
Get-Service | Where-Object { $_.Name -match 'cyber|power.?panel|pwrctl' } | Format-Table Name, Status, StartType -AutoSize
```

**Graceful shutdown: the r2 deferral has EXPIRED** — its condition ("after
the rebalance, re-measured at 100%") is met. Now an open ACTION (§7): at
100% charge, read the 1000VA's estimated runtime in PowerPanel, then set
the threshold against the measured figure. The argument stands: the
archival RocksDB store is non-regrowable, and NINE uncontrolled losses
survived is a record of luck.

## 6.6 HOST MEMORY POSTURE (first railed at BL-085(2))

`C:\pagefile.sys` **fixed 16,384 MB**, `AutomaticManagedPagefile = False` —
set with Claude assistance at an earlier date, documented nowhere until
09-01. Commit limit = 31.4 GB usable + 16 GB = **47.42 GB** (measured flat,
806/806 samples). Steady state: ~79% RAM is the CONFIGURED EQUILIBRIUM
(BL-081), not drift — occupancy is not pressure; paging is (pagefile peak
1.8% over 55h). Alerting rule of record: `windows_memory_swap_pages_written_total`
sustained climb = genuine eviction; a RAM-percentage rule fires at 79%
today and means nothing.

## 6.7 DEADMAN SEMANTICS (corrected by event #9, BL-087)

`KronHeartbeat` (healthchecks.io, 5m/5m) measures **INTERACTIVE-SESSION
liveness, not host liveness**: principal `LogonType: Interactive`, no boot
trigger — a host at the logon screen is booted, healthy, and silent to it
(19 min measured, 09-02). Failure-only log at `C:\zkas\logs\heartbeat.log`
(absence = never fired). Fix rides H2 with the sampler's: boot trigger +
non-interactive principal, one principal decision covers both tasks. Until
then, a deadman page decodes via: PowerPanel row · port census · Supabase
`network_history` rows (the second off-box 5-min clock).

## 6.8 FLEET + FIREWALL INVENTORY (first railed at r3)

**Rig web UIs (uptime = the premises instrument of record):**
w1m 192.168.1.21 · w2m 192.168.1.22 · w5m 192.168.1.25 · w6m 192.168.1.26 ·
w7m 192.168.1.27 · **w8m 192.168.1.28 (surge-only canary)** ·
w9m 192.168.1.29. (Worker number = final octet.)

**gRPC scoping — BOTH legs closed (09-02):** zkas 16810 →
`zkas gRPC 16810 - MacBook only (H7)` @ 192.168.1.173 · kaspad 16110 →
`Kaspa gRPC MacBook only` @ 192.168.1.173 (BL-085(3)'s gap, found already
executed under this name; the entry's recorded pre-fix name is obsolete).
Dead disabled `ZKas gRPC LAN only` /24 rule removed 09-02.

**OPEN FINDING — program-scoped Allow rules undermine port scoping:** eight
first-run rules (`kaspad.exe` ×4, `kaspad` ×2, `zkas-node` ×2) allow
**Any port / Any remote**. Windows allows are additive, so if these share
the active profile, the port-scoped MacBook-only rules above are
NON-AUTHORITATIVE — 16110/16810 effectively open to the LAN through the
program rules. `Kaspa-Borsh-Laptop Only` (17110) likewise scoped Any
despite its name. Needs a profile-aware audit + deliberate rationalization
(explicit port rules authoritative, program rules disabled) at the RULES
SITTING — not a drive-by; a blind disable could cut a port the explicit
rules do not cover.

---

## 7. OPEN THREADS THIS DOC DOES NOT CLOSE

- **Night dips** (01:58, 01:30-era): no suspect yet; instruments as at r1.
- **Bridge robustness question** (v2.0.1.5 A-item): unchanged; both tracks
  proceed.
- **Graceful-shutdown threshold — NOW SETTABLE** (r2's deferral expired):
  measure the 1000VA runtime at 100% charge post-rebalance, set against the
  measured figure. Owner: next Kron sitting.
- **Firewall rationalization** (§6.8 finding): program-scoped Any/Any
  allows vs. port-scoped restrictions — rules sitting.
- **Gateway cable drop** (§6.5.1) — UPS-expansion window.
- **NUT build + drill** (§6.5.3) — on Pi arrival; the doc's §6.5 scoping
  paragraph is amended when the 1500VAs stop being dark.

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
- 2026-09-02 · r3 · §6.5 rewritten to executed reality (measured loads,
  battery-backed/surge-only terminology, w8m canary, brick swap + armed
  experiment, PowerPanel/1500VA scoping, gateway fate-sharing decision,
  KS7-per-unit end-state, NUT witness node + charter). NEW §6.6 pagefile
  posture · §6.7 deadman semantics · §6.8 fleet IPs + gRPC closure + the
  program-rule finding. §7: shutdown-threshold deferral expired → open
  action; new gates. Ledgered BL-087/BL-088; sources BL-081/085/086.
