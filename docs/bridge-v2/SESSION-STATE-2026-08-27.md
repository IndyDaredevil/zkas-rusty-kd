# SESSION-STATE — 2026-08-27 (S11/S12 close, deploy-eve handoff)
### Supersedes SESSION-STATE-2026-08-21.md. Deep content lives in the tier
### docs (ledger @ BL-049, SCOPE r3, FLEET-DEPLOY r1) — this doc is live
### state, gates, and queue. Cite, don't re-derive.

## OPEN THE NEXT SESSION WITH
Docs-tip: 51f4193 (merged-v2.0.1.4). Code branch: merged-v2.0.1.5 @ 1b63698.
Canary v2.0.1.5 soaking on :5775/:3035 since 08-27 14:21 ET (banner + parker
gates PASSED). Then answer: (1) soak verdict + deploy done? (2) pace gate at
02:55 result (rc:solves_24h ≥ 33 = pass)? (3) conduct-law block filed?
(4) mount = post-deploy revisions?

## LIVE STATE
- **Production bridge**: v2.0.1.4, single process on :5755/:5765/:3034,
  StartTime 08-27 07:07:48 (deliberate restart). SIX workers (w1 is on the
  canary as w1c). Interim rule in force until deploy: NO browser tabs on
  :3034 (BL-045).
- **Canary**: stratum-bridge-1b63698.exe, SHA256 F1484FB5DCC7631CB29BCED90F
  2B8E89F8A5B7EACF5432CFF3603642B3E7A3F0, C:\zkas\canary\, console →
  canary-console.log (log_to_file OFF by design — reporter tail protection).
  w1c attached. Watch: A8 breaker single INFO ("balance polling disabled").
- **Reporter**: running (manual start 10:41), battery-stop flags flipped
  False (verified). Rotation across bridge restarts is native (source-
  verified) — no restart at deploy. Death #1 (~00:01) cause OPEN
  (clean-exit class); hardening spec → H2.
- **Books**: zkas_blocks square — 13 rows healed 08-27 (6 replay + 4
  catch-up + 3 provisional corrections). zkas-catchup-r1.ps1 at C:\zkas\
  (sha dff29f53…f1a7) takes any log path — the gap-healing tool.
- **UPS**: 3 units installed 08-27 (H2 headline done early). Final split:
  2×(KS7+2 KS0)@~76%, 1000VA = Kron+net only (~63W), third KS7 surge-only.
  Spare 19V brick = H2 procurement (BL-040/044: 8/15–16 convicted
  Kron-local).
- **Pace**: cold streak broke (43 @ ~14:00 vs gate 33). Morning fault
  investigation closed as tail variance — all layers measured healthy
  (BL-045-adjacent instruments validated fleet/shares/submit).

## GATES PENDING
1. **Soak-length call ~20:00 08-27**: full soak (verdict ~02:21) vs evening
   deploy on the 4h read (~18:20: A8 INFO + clean shares + zk=ok = evening
   deploy defensible; deterministic parker proof already banked).
2. **Pace gate 02:55 08-28**: rc:solves_24h < 33 → escalate node-side
   forensics; ≥ 33 → closed as variance, one ledger line.
3. **Deploy**: per FLEET-DEPLOY-v2.0.1.5-r1 (committed; mount-synced).
   Promote SOAKED BYTES (canary exe → production path); v2.0.1.5-win
   release = version-of-record only; canary-1b63698 prerelease KEPT.
   Banner gate is HARD (BL-047). Post-deploy: parker proof on :3034 lifts
   the tab ban; w1→:5755/w1m; canary down, C:\zkas\canary kept as evidence.
4. **A1' final acceptance**: max_over_time(scrape_duration_seconds[7d])
   < 5s across seven production days → BL-050 close-out + SCOPE r4.
   .bak-v2014 retires after ONE clean day (BL-031).

## QUEUE AFTER DEPLOY (unordered unless noted)
- P1 Bolt paste (brief cut+sha'd: P1-BOLT-BRIEF-r1.md, 86d2b546…a210,
  ~/zkas-lab/) — every unpasted day loses D_z/D_k curve. Then Kron sampler
  (Claude cuts on request).
- A4 residual: one eyeball — HourlyMergedReport card vs live rc: values.
- H2 window: service migration ×7 (template: at-startup trigger + delay,
  battery flags False, ExecutionTimeLimit PT0S-equivalent, exit logging),
  KRON-HARDENING §2–§6, windows_exporter package + ZkasLegDegraded
  page-tier (H6), node v1.0.6 + --ram-scale + gRPC scope-down + firecash
  wedge report send (H7), AppData\Local Defender-exclusion narrowing,
  escalation-channel design (BL-046), SG116E fw check, port-11 device ID,
  ports 15/16 mapping, KS0 8/17 boot-timestamp tell (low).
- Ledger next: BL-050 = deploy close-out (gets: soak verdict, pace-gate
  result, deploy record incl. old-exe hash, week acceptance when it lands).
- v2.0.1.6 seeds: A6 job-delivery latency; A1-hygiene bounded retention
  (event gauges are the dashboard's block store; ghosts are load-bearing).

## CONDUCT-LAW BLOCK — NOT YET FILED (operator paste → project instructions)
14. COMMANDS NAME THEIR TARGET. Every fenced command states machine + shell
    (Kron/PowerShell, Kron/cmd, MacBook/zsh); the operator's prompt is
    checked against it before execution. Bridge lifecycle = cmd;
    diagnostics = PowerShell. Ship checks: last-token integrity eyeball
    (no markup fragments); grep patterns verified against a live excerpt
    with explicit case handling; array parameters via -Command, never
    -File; any probe that can block carries its own timeout ceiling.
    (S12: seven exhibits, BL-049.)
15. DESTRUCTIVE UI DELETES CITE LINE COUNTS. Any instruction to delete a
    mount/UI file states the file's line count as the identity check at
    the destructive step. UI uploads use stage-verify-open: one command
    stages the exact files into a single-purpose folder, shasums them, and
    opens it — clicks never navigate similar paths. (S12.)
Also amend law 1b's practice note: the move-and-verify ships IN THE SAME
MESSAGE as the file, no exempt class (S12 incident: FLEET-DEPLOY).

## KEY VALUES
- Docs-tip: 51f4193 · code tip: 1b63698 · ledger seal: BL-049 (975 ln,
  6df36df6…8a60) · SCOPE r3 (1b70e2ba…632e3) · FLEET-DEPLOY r1
  (5df03ad7…4e4e)
- Canary exe: F1484FB5…A3F0 · voided stale canary (eda7090 build):
  66C27E9E…C882, retired; 8,048ms positive control banked from it
- Positive-control pair (BL-045): v2.0.1.4 parker = 8,048ms pin;
  v2.0.1.5 parker = 213–232ms ×8 + FIN eviction
- rc: recording rules faithful (A4): 32/35/27 ≡ raw at test time
- Defender exclusions: Node-v2, RKBridge, rusty-kaspa-v2, AppData\Local
  (narrow → H2), inmyh\rusty-kaspa, C:\zkas
- Production launch: run-rc-merged.cmd (repo root; env ZKAS_MERGED_NODE
  127.0.0.1:16810 + ZKAS_TREASURY_ADDRESS; --config rc-v2-smoke.yaml
  --node-mode external; exe target\release\stratum-bridge.exe)
