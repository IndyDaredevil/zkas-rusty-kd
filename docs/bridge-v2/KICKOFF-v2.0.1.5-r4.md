# KICKOFF — v2.0.1.5 (Bridge Stream A + Pipeline Stream P + Host Stream H)
### Cut 2026-08-26 (late) · r4 — SINGLE clean seed doc for the v2.0.1.5 chat.
### Revision chain: r1 (d2a0d1b7...0402348) VOID same evening; r2 (da380662...7adc15)
### + ADDENDUM r2 (be3a7ba5...86c2e2) SUPERSEDED BY MERGE, preserved in git history
### at 633a285; r3 (2da14c08...0d315) VOID — cited the bridge spec by a versionless
### filename before the naming law landed (filename carries the described artifact's
### version; header carries content revision). r4 = r3 with that citation corrected.
### The r1→r2 investigation content (§2) is unchanged since r2.
### Supersedes the Stream-A framing of SCOPE-v2.0.1.5.md r1 — that doc's A-items
### were rewritten by the 08-22 stall investigation; §2 here IS the r2 scope
### content until folded back (queue item 7).
### Companion docs in docs/bridge-v2/ (all ON ORIGIN as of 633a285):
### SCOPE-v2.0.1.5.md (r1, Stream P still authoritative) · ENGINEERING-LEDGER.md
### (sealed @ BL-031) · KRON-HARDENING.md (r1, committed, NOT yet applied) ·
### SESSION-STATE-2026-08-21.md · BRIDGE-SPEC-v2.0.1.4.md (content r2, as-built
### system spec, sha 070c717d...557001) · NODE-CONTRACT-v1.0.5.md (r1,
### consumed-binary contract, sha a94ef5b5...a2ee0) ·
### archive-merged-bridge-v2-spec-draft4.md
### (frozen design history: port rationale, invariants 1-8, V-gates).

---

## 1. STATE SNAPSHOT (as of 2026-08-26 late session)

- **Bridge:** v2.0.1.4 (`merged-v2.0.1.4`; docs-tip 633a285 at r3 commit time —
  spec pair landed @ e7426e1, kickoff set @ 633a285; code identity 336b7a5
  four-way verified 08-20, unchanged). Restarted ~16:00 ET 08-26 (operator-
  reported; cause not recorded — correct here if it was a crash). Prior run:
  6 unbroken days 08-20→08-26, the best longitudinal dataset the operation has
  produced. PERISHABLE: `scrape_samples_scraped` + `scrape_duration_seconds`
  graphs over that span age out of the 15-day TSDB ~09-04 — capture before
  then if not already done.
- **Prometheus:** GLOBAL scrape 1m / timeout 55s / eval 15s (08-22 stopgap —
  note it landed globally, not per-job as designed; reporter job rides at 1m,
  accepted-for-now, revisit in H-stream). 41+4 rules deployed and verified via
  /api/v1/rules 08-22. RcScrapeFlaky + RcNearMissSilent + RcZkasBlockNot-
  ConfirmedBlue all confirmed deployed (08-22 artifact read).
- **Dashboard rail:** live, 738 blocks / 37,220.6 ZKAS at 08-22 read. Reporter
  two-beat protocol operating; RcReporterPendingStuck pended 08-22 09:18
  (walletd slow during the morning grind) and self-healed.
- **Fleet/wallet:** unchanged from memory-state (7 rigs ~14.2 TH/s nameplate;
  ~2M ZKAS custody; cold-storage sweep sequence agreed, unexecuted).
- **Host:** Win 11 Pro 25H2 (26200.8875). UPS units ONSITE, not installed.
  KRON-HARDENING r1 drafted AND committed (633a285), NOT applied. Auto-updates
  believed disabled but Store/USO channels proven active 08-22 (multi-channel
  lesson).
- **Laptop rail:** ONE canonical clone at `~/zkas/zkas-rusty-kd` (blob:none,
  docs/bridge-v2 sparse cone, gh browser-auth in Keychain, git identity
  indygold@gmail.com), fast-forwarded to tip. A duplicate clone briefly
  existed 08-26 and was verified-clean then deleted (two-clone incident, §7).
  One clone, one path, from here forward; `find` before any future clone.

## 2. WHAT THE 08-22..26 INVESTIGATION ESTABLISHED (the r1→r2 delta)

1. **A0 CLOSED, early, worse than hypothesized:** 14s scrape ceiling hit at
   ≤37h uptime; then 25s ceiling within hours of raising it. 18 scrape
   failures over 2 days (query_range table; graph rendering decimated them).
2. **BL-029's causal claim REFUTED:** metrics page is ~530 samples serving in
   ~230ms (Measure-Command, direct probe). Series growth is real (~6/hr,
   smooth linear ramp — schedule-minted, not reconnect-minted; 250→530 over
   44h) but CANNOT drive a 25s render. A1 (series retirement) demoted to
   hygiene; the smooth-ramp mechanism is itself an open question.
3. **The stall class:** episodic — floor ~230ms with rare spikes that pin at
   whatever the timeout is (12→14→25s observed). Blocked, not busy: true
   stall length has never been measured (has outlived every ceiling; 55s is
   the current best chance to measure one).
4. **Morning stalls: SOLVED and CLOSED (verdict 08-26).** Root cause: daily
   Windows Store retry-grind on Microsoft.ScreenSketch failing 0x80073D02
   (app in use — the operator's own screenshot workflow). Killed the process
   08-22; retro-check 08-26 shows newest Id-20 = 08-22 and ZERO morning-era
   up-dips in 4 days. Prediction made, prediction confirmed.
5. **NIGHT DIPS: partially resolved 08-26 — composite trigger, not one
   mechanism.** Six since 08-22, all 21:30-03:33 ET, ~1.5/day; newest five
   hours after a fresh restart (not-size verdict re-confirmed). RDP session
   log (08-26) three-way verdict:
   (a) CONVICTED for one historical dip: 8/21 01:30 disconnect/reconnect
       lands 9 SECONDS before the 01:30:22 dip — tightest coupling of any
       suspect all week.
   (b) CIRCUMSTANTIAL for 08-26 21:30: operator actively in-session running
       the retro-check at dip time; churn 7 min later.
   (c) UNCOUPLED for 08-26 01:31/03:33: no session events for ~20h around
       them — though the session idles ATTACHED for days (disconnects pair
       with reconnects seconds later), so idle-session machinery is not
       excluded. These two remain unexplained; Defender signature updates
       (one observed 22:44 08-21) stay in the lineup:
       `Get-WinEvent -FilterHashtable @{LogName='Microsoft-Windows-Windows Defender/Operational'; Id=2000,2010} -MaxEvents 30 | Select TimeCreated, Id | Sort TimeCreated | Format-Table -AutoSize`
   Composite reading: dips couple to TRANSIENT HOST ACTIVITY of any kind
   (servicing, RDP churn, operator presence). This collapses the mystery
   into Stream A's headline: the render stalls under brief events a 230ms
   endpoint should shrug off. BONUS: RDP churn = a REPRODUCIBLE TRIGGER —
   see H1's summon experiment.
5a. **CORRECTION to the 08-22 record:** the morning-era Winlogon 6003 events
   (9:05:15, 10:16:03) match RDP reconnects TO THE SECOND — they were the
   operator connecting, not TrustedInstaller servicing. One evidence leg of
   the morning conviction is reattributed; the conclusion STANDS on the
   independent legs (WU download/Id-20 events, SmartRetry, and the confirmed
   quiet-mornings prediction). Ledger candidate in its own right.
6. **The sharpened bridge question (new Stream-A headline):** why does host
   pressure turn a 230ms render into a ≥25s stall rather than a slow one? A
   loaded host should degrade the render to seconds, not freeze it. Suspect
   classes for the source read: blocking write/flush on the render path, sync
   RPC reachable from the handler, runtime-pool starvation (blue-confirm loop
   30×2s running sync on the async pool). Correlation evidence: SOME stalls
   coupled to ks_merged_zkas_rpc_ms elevation, some not (survivorship caveat:
   the worst rpc sample dies with the failed scrape).

## 3. REVISED STREAM A (bridge release v2.0.1.5)

- **A1′ (headline): make /metrics unstallable.** Source read first: trace the
  render path for anything that can block ≥ seconds (locks shared with RPC
  work, sync I/O, pool starvation). The read ANCHORS ON BRIDGE-SPEC §5
  (metrics contract) and §6 (logging contract) as the as-built reference;
  discrepancies vs the tree are ledger entries per that spec's own
  ground-truth clause. Fix shape decided by the read; the structural
  end-state is a lock-free/snapshot render that nothing the bridge does can
  starve. Acceptance: zero timeout-kills across one week INCLUDING declared
  host-pressure windows.
- **A1-hygiene: series lifecycle.** Explain the ~6/hr smooth mint (what runs
  hourly per worker?), then retire idle series with BL-025-aware design
  (grace ≥ 10m; full-labelset key; re-verify all five block rules +
  RcReporterStarved against the new coexistence behavior BEFORE deploy).
- **A2 full_clear pin · A3 estimator drift · A4 K/Z/D 24h discriminator ·
  A5 submit-latency/time-to-blue · A6 stale-share/job-latency:** unchanged
  from SCOPE r1, including their reads-first gates. A4 note: the 08-26
  restart re-arms the discriminator (needs ≥24h uptime; valid from ~08-27
  16:00).
- Release mechanics unchanged (r1 §A7): BRIDGE_BUILD→5, canary :5775/w1c
  12h soak, four-way identity, BL-030 banner guard.

## 4. STREAM P (pipeline — unchanged from r1, not started)

P1 network_history (D_z/D_k 5-min sampler; SHIP FIRST — curve is being lost
continuously at 15-day retention) → P3 worker_events → P2 worker_stats
(hourly rollups; metric-name verification first) → P4 latency consumer
(gated on A5). Bolt briefs use the 08-21 constraints-first format + the
never-reinit-history addendum; operator sets any secrets.
Citations for the enrichment work: BRIDGE-SPEC §6/§9 (kaspa-parent join
window, schema pointers) + NODE-CONTRACT §3 (RPC caveats; aux_pow is
stripped from RPC responses). Standing constraint on ALL P-stream schemas:
invariant 8, the accounting law — `solves = kas + zkas − doubles` — now
codified in BRIDGE-SPEC §7 as a contract on every consumer.

## 5. STREAM H (host — new, from the investigation)

H1 The summon experiment (FIRST — five minutes, reproducible-bug test):
   2s probe loop running (Measure-Command curl loop, 08-22 form) while
   deliberately disconnecting/reconnecting RDP several times. Stall appears
   on demand -> A1' has a test harness before any source is read. Then the
   Defender-signature diff (§2.5c) for the two unexplained dips.
H1b RDP posture check: LAN-only (no 3389 forward at the router — BL-023
   boundary discipline; RDP on a wallet-custody box is LAN-acceptable,
   WAN-unacceptable), NLA enabled, and note that the standing attached-idle
   session is itself a variable — prefer full sign-out over disconnect when
   leaving, so idle-session machinery is excluded from future correlation.
H2 First maintenance window, ONE downtime: UPS install + KRON-HARDENING
   §2–§6 application (each with its artifact gate) + Store/OS update flush +
   deliberate bridge relaunch via run-rc-merged.cmd. Post-window: §8 gates.
H3 Hardening acceptance: KRON-HARDENING §9 (7 quiet days) — clock starts at
   application, not at draft.
H4 Clock discipline: w32tm status read (the ~30-min step events, 08-22);
   fix if seconds-scale.
H5 Revisit the global-vs-per-job scrape config (reporter back to 15s once
   the bridge render is trusted again).

## 6. WORK QUEUE (proposed order for the new chat)

1. H1 summon experiment + H1b posture check (ten minutes total; a
   reproducible stall reshapes the whole A-stream read).
2. Ledger session: draft BL-032+ from §7, append via laptop rail, reseal.
3. Perishable captures if not done (six-day graphs; TSDB deadline ~09-04).
4. P1 Bolt brief out.
5. A-stream reads session (A1′ source read + A2/A3/A4/A5/A6 reads — one
   sitting, no code).
6. Schedule H2 maintenance window.
7. Fold §2–§5 of this doc back into SCOPE r2 and commit it (KICKOFF r3 +
   KRON-HARDENING are already on origin — this is the last uncommitted
   revision debt).

## 7. LEDGER CANDIDATES (BL-032+ raw list, draft in queue item 2)

- Blocked-vs-busy: a scrape duration pinned at the timeout means the handler
  didn't answer, not that it worked that long; only a direct endpoint probe
  (Measure-Command, 257ms) separates them. One probe ended three days of
  Prometheus-side inference.
- The ScreenSketch arc: host servicing masquerading as an application bug;
  the operator's own investigation tooling (screenshotting) kept the package
  in-use and fed the failure loop. Multi-channel update lesson: "auto-updates
  disabled" governed none of the channels that fired.
- Graph decimation vs query_range: medium-res graphs hid 15 of 18 failures;
  the API table is the data, the graph is a picture.
- PowerShell 5.1 → curl.exe strips embedded double quotes even from
  single-quoted strings (backtick-continuation form); write PromQL
  matcher-free and filter in PowerShell, or use the UI. (Extends the
  existing quoting note, which covered only the interactive one-liner form.)
- gh browser-auth over PAT for interactive machines: two PAT scoping
  failures vs one un-mis-scopable device flow. PATs remain correct for
  sandboxes.
- The `~` cwd trap, zsh edition: mv-to-dot from the wrong directory strands
  files silently; the prompt's directory segment is the pre-command gate
  (BL-021 corollary).
- Prometheus Table tab shows one instant, not history; "peaks as a table"
  is a query_range + client-side filter job.
- Evidence-leg reattribution without conclusion collapse: the 08-22
  morning Winlogon 6003 events were RDP reconnects (second-exact match),
  not servicing — the morning verdict survives on its independent legs plus
  the confirmed prediction. Corollary: session-scoped logs (TerminalServices
  LSM) belong in every host-event sweep alongside System/Application.
- An attached-idle RDP session is a standing host variable: disconnects pair
  with reconnects within seconds and the session persists for days; operator
  presence is therefore ambient, not evented, unless sign-out is practiced.
- Kernel-General 1/24 pairs every ~30 min = clock stepping; correlation
  precision rides on w32time discipline (open until H4 verifies).
- BL-004 cross-doc drift (08-26): the ledger's "structurally immune" wording
  was retracted by Draft 4 §6 (immunity = lockfile-drift class ONLY; the
  cdylib/risc0 LNK class lives in-workspace and took three fixes). The
  ledger inherited forward what the spec had walked back. Correct BL-004's
  text at reseal.
- Meta-principle 1, documentation tier (08-26): BRIDGE-SPEC r1 reproduced
  the retracted claim by drafting from ledger + memory instead of reading
  the in-repo artifact; the pre-move `ls` gate surfaced the old spec and
  caught it before commit. A spec cut from secondary records inherits their
  drift.
- Conduct law minted (08-26): a deliverable's destination is
  verified-or-created within the same instruction set, and multi-step
  sequences state their gating (deploy one-liners shipped against a
  roadmap-item checkout; ~2 min cost, zero writes — atomic mv failures).
- Discovery (08-26): the "incomplete docs/bridge-v2 commit" open item was
  already closed on origin pre-session (SCOPE, ledger, session-state,
  README, monitoring/, reporter/ all present at 5b37875).
- Two-clone incident (08-26, resolved same session): the failed `cd` that
  opened the session was read as "checkout never stood up" when a clone
  existed at a DIFFERENT path — a second clone was created, and the two
  briefly diverged (new at e7426e1, old self-reporting "up to date with
  origin" at 5b37875 from stale refs — BL-020 live, at repo scale). The
  old clone held the only copies of two uncommitted docs, exactly where the
  hazard analysis predicted. Laws: `find` for existing clones before
  cloning; a clone's path is part of its identity and gets recorded like a
  sha; "up to date with origin" is a statement about last-fetched refs,
  never about origin.

## 8. STANDING CARRIES (unchanged, from memory/08-21 state)

Cold-storage sweep (UPS-adjacent timing) · 287.58 row disposition · history
recovery session (~110 blocks, WS3-FT minimal cut) · log retention rule ·
2-file replay patch offer · zkas-node memory slope · kaspad-death alert ·
firecash dmg bug report · mimalloc A/B (parked) · covenant C1 (laptop,
separate thread) · fire drill (RcReporterDown, five minutes, still unrun).
