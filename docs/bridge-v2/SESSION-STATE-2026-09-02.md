# SESSION-STATE — 2026-09-02 (supersedes 2026-08-30)

Written at the close of S19. The 08-30 document predates reporter r3, the
deadman, A3's solution, v1.0.8, the PRF0 session, and event #9 — five
threads of drift. This is the current world.

## LIVE STATE (verified this date)

- **Bridge v2.0.1.5** on both legs, seven rigs, ledger `2b09238`-era config
  unchanged. Gauge `ks_zkas_estimated_network_hashrate_gauge` serves a
  stranded-anchor constant 95% of ticks — KNOWN, inert, fix seeded
  (BL-079/086: `Some(tip_hash)→None` at kaspaapi.rs:543/:511, v2.0.1.6).
- **Reporter r3** (`eb2b813d…`) on the money rail: fail-closed provisional,
  ERRSTREAM, walletd visibility metrics. Defer-gate and ERRSTREAM both
  fired correctly at event #9 (BL-087). pending 0 / failures 0.
- **KronHeartbeat deadman** live (5m/5m, drilled both ways 08-31) — with
  BL-087's semantics correction: it measures INTERACTIVE-SESSION liveness;
  a host at the logon screen is silent. Fix rides H2.
- **windows_exporter** six collectors + process filter (:9182). Thermal:
  board-scoped WMI negative (BL-075). Memory: configured equilibrium
  ~79% RAM, no pressure (BL-081) — pagefile FIXED 16 GB, documented at
  BL-085(2).
- **zkas-node v1.0.6** archival + **zkas-walletd v1.0.5** (:8501) —
  **both superseded by the v1.0.8 release; H8 is briefed, rehearsed, and
  next** (BL-089). Stranded tip e8dc1a03… still at tipHashes[0] on Kron
  (measured 09-02, BL-086); heals itself ≤12h after H8.
- **Power**: PERFEIDY 120W brick in service since 14:12:49 09-02 —
  **experiment armed** (BL-087): another 41/6008 + zero PowerPanel rows
  convicts barrel-or-board. Old AS0651 labeled/retained. Outlet map final
  (BL-088): w8m = surge-only premises canary; 1500VAs are DARK instruments
  until NUT lands; gateway→1000VA gated on a cable drop.
- **MacBook wallet app v1.0.29** (custody + Covenants++ harness): /reveal
  era retired, App Lock on (single challenge spans both wallets), embedded
  node v1.0.8 synced, balance 200,451,802,596,746 sompi / 7 notes verified
  (BL-090).
- **Upstream**: firecash/zkas-rusty#6 CLOSED CONFIRMED — six stranded tips
  pruned on their archival nodes; fix 9a464d51 CONTAINED in v1.0.8
  (compare-verified, BL-089).
- **Supabase rails**: zkas_blocks (beat2-latency = walletd health
  instrument, BL-078) + network_history (P1 curve; ~5-bucket gap
  17:50–18:15 UTC 09-02, annotate before reading).

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **D2 verdict** — gate fires 09-04 ~01:09 EDT:
   `max_over_time(scrape_duration_seconds{job="rc_merged_bridge"}[7d]) < 5s`
   (last read: 13ms-class). Pass → close BL-050, retire SCOPE r5 from the
   mount (laptop archive keeps it), open v2.0.1.6. **D1 rides the same
   sitting** (.bak-v2014, -WhatIf first) — never standalone.
2. **Arrivals**: Pi kit + high-endurance SD (witness node — build NUT,
   drill three pulls, then charter items in order) · third 1500VA (rig
   redistribution window + gateway cable drop + canary retirement).
3. **ERRSTREAM soak**: next console flash →
   `Get-Content C:\zkas\reporter.log -Tail 50 | Select-String ERRSTREAM`.
4. **Brick experiment**: any new 41/6008? (Silence is weak evidence;
   the falsifier is what's crisp.)
5. **kaspad gRPC narrowing** (BL-085(3)) — one firewall edit, still
   UNEXECUTED: 16110 open to the rig subnet, unauthenticated.
6. **H8 scheduling** — the next multi-hour window: node+walletd →
   v1.0.8, canary-first on a wallet-file COPY; maintenance-blip riders
   fold in (externalip activation, kaspad nologfiles=false + versioned
   launcher, /warm into STARTUP-ORDER, port audit). Acceptance: beat2 p50
   ~200s · poll-failures flatline · give-ups 0 · stranded tips leave
   tipHashes ≤12h.
7. **Subsidy stepped** 45.24092998 → 42.70175169 (09-02): roll every
   expected-value figure; the 28-row give-up reconciliation (post-H8) uses
   era-correct values.

## STANDING QUEUE (order)

H8 → H2 (service migration + deadman/sampler principals + UPS riders) →
rules sitting (walletd distress alert, ZkasLegDegraded, H6 rules-only on
swap_pages_written, RcReporterDown fire drill) → P2 pointer change
(rc:fleet_hashrate_delivered_hps) → P7 first query → P8 metric-name pin →
cold-storage sweep (case strengthened twice this week) → docs sweep
(KRON-HARDENING r3 w/ rig IPs + power rewrite · WINDOWS-EXPORTER runbook ·
NODE-CONTRACT-v1.0.8 + NODE-CUTOVER r2 (fix -AsByteStream) + STARTUP-ORDER
r2, all post-H8 · FLEET-DEPLOY r2 · BRIDGE-SPEC §2 · covenant note) →
upstream remainder (ladder docs PR w/ params.rs F-04/F-05 · surplus note
w/ txids · anchor-semantics note citing BL-086's sink discriminator).

## HANDOFF DEFECTS CORRECTED HERE

The 08-29 handoff's Q1 prompted a standalone D1 — WRONG, D1 is coupled to
D2 (this doc states it correctly). The 08-30 doc's est_hashrate_z
"bimodality WATCH" is CLOSED (BL-086: arithmetic, same root cause as A3,
no residual).
