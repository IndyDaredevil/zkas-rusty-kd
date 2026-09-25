# SESSION-STATE — 2026-09-11 (supersedes 2026-09-07; written ~02:20Z 09-12 = late 09-11 EDT)

Written at S28 close. Since the 09-07 doc: S27 + S28 banked (BL-113..121,
ledger 3996 ln cc28ec63…); P11 CLOSED; T-7 RESOLVED; P8 SHIPPED; reporter
r8 live; sampler r2 live; treasury reconciled to the sompi; stage table
dropped; Kron-2 returned; Pi re-scoped. ITEM-REGISTRY-r8 holds definitions.

## LIVE STATE (verified this date)

- **Kron**: v1.0.8 everywhere (same binaries; the 09-11 12:00–12:42 EDT
  restarts were manual, same builds). **Reporter r8** (e01206ae4d3b8942,
  BOM, textfile-hook r1 carried) live since 11:18 EDT; live-r6 kept at
  C:\zkas\zkas-reporter-r6.ps1 (95ba8e98) as rollback. Metrics :9151.
- **Samplers (5-min tasks, inmyh)**: NetworkHistorySampler →
  network-history-sampler-r2 (7f586bf0…; near-miss counters since 09-09
  08:15Z); KronEventsSampler → kron-events-sampler-r2 (9a1c5675…; state
  C:\zkas\kron-events-state.json; secret C:\zkas\ke-secret.dpapi).
- **Rules**: alert_rules.yml 962 ln db8a753ab9638dc8, 48 rules, group
  kron_process live (kron:process_start_seconds, kron:host_boot_seconds,
  KronProcessRestarted, KronHostBooted — severity warning). Backup
  .bak-pre-p8-20260911-210803.
- **Dashboard (mining-dash)**: drought card = pace / expected(nameplate) /
  expected(near-miss K,Z) / since-midnight / near-miss lines; kron_events
  (typed, check-constrained, metadata.source/component, dedupe_key partial
  UNIQUE); Gap distribution vs Poisson on the trailing-7d mean (p≈0.87);
  hourly card verdicts removed (p=0.48); Total zKAS = treasury via
  zkas_adjustments; kaspa_double corrected (45 rows) — 90.0% since 08-30.
- **Edge functions**: network-history-webhook (3 optional near-miss keys),
  kron-events-webhook (env KRON_EVENTS_SECRET, DEFAULT_WALLET_ADDRESS).
- **Supabase**: 0 tables without RLS (zkas_double_backfill_stage dropped
  09-09; archive ea31cbf9… on the laptop). Mailbox: inbox CLEAR; last
  inbound row 87 acked; last outbound row 85 (closed by 87).
- **Treasury**: 80,640.78 ZKAS at 09-09 read; auto-consolidate is the only
  spender (24,578,600 sompi per 38-note merge); pre-08-07 mining = derived
  plug 5,956.51 (labeled) until an fvk exists off the production host.
- **Hardware**: Kron-2 unit received with 24 GB → RETURN; Pi 5 4 GB kept
  (deadman/NUT/pollers, pwsh 7); 2 TB SSD + 27 W PSU on hand.
- **Wallet 1.0.32**; zkas-wallet#3 OPEN; 09-09 reproduction (engine bound
  to the public node with Own node selected, services.log 1788946298806)
  NOT yet posted.
- **Upstream**: zkas-rusty#8 open; A2 held for a zakura-orchard release
  carrying #362; ironwood#223 unanswered as of 09-08. Supply step 6
  (~09-10 09:00Z) passed with no screen on it — NOT read back.

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **KronEventsSampler** — `Get-Content C:\zkas\logs\kron-events-sampler.log
   -Tail 3` reads `OK no change` at 5-min cadence? (First exporter-sourced
   row arrives on the next restart of any of the six.)
2. **Supply step 6** — read `zkas_supply_check_status` and the drift series
   across 09-10 09:00Z from Prometheus; the first drift calibration event
   happened unwatched.
3. **Near-miss reference** — has the 7d window filled? Card shows "7d avg"
   instead of "since 09-09"?
4. **Double KPI** — still ≥89% since 08-30 with r8 flagging live? One SQL.
5. **H2(a)** daytime sitting: reporter + kaspad launchers, kaspad
   nologfiles=false ×2; reporter r9 (start gauge + DPAPI token) rides the
   reporter launcher and is the first live KronProcessRestarted test.
6. **Rules sitting** R-1..R-5, R-7 via the r2 hook pattern (UTF-8 bytes,
   cmd /c promtool).
7. **zkas-wallet#3** — post the 09-09 reproduction (V.1 pass, API readback).
8. **Pi build-out** T-1a: pwsh 7, NUT, off-host deadman, pollers.

## STANDING QUEUE

Per ITEM-REGISTRY-r8: H2 → rules sitting → docs sweep (runbooks r2, laws
r3 with I-37..I-46) → reporter r9 → Pi T-1a → upstream remainder →
cold-storage sweep → fork rebase. Kron-2 (T-1b) on correct stock.

## HANDOFF NOTES

S28's incidents were mechanisms, not judgment: cut from the LIVE file, not
the mount, for anything that also runs (I-42); operator-held values are a
numbered step before the fence, never a variable in it (I-43); external
tools via cmd /c + $LASTEXITCODE (I-44); bytes in, bytes out, BOM preserved
(I-45); @() at the call site (I-46); expected output is prose, never fenced.
The reveal/plaintext-token finding (BL-116) is still open — r9 closes it.
T-7's lesson generalizes: when a "loss" equals a template count, it is a
parser.
