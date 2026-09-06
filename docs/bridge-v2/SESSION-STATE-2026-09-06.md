# SESSION-STATE — 2026-09-06 (supersedes 2026-09-05)

Written at S24 close. Since 09-05: zkas-wallet#3 filed from a measured
walkthrough; wallet on 1.0.32; dashboard arc 14→23 closed with every pin
verified; checker consolidated; T-5/P9/P10/R-6 closed. ITEM-REGISTRY-r6
holds definitions; this is the snapshot + opening questions.

## LIVE STATE (verified this date)

- **Kron**: v1.0.8 everywhere; reporter r6 (f82bf0c2…) with kaspa_double
  on both beats; **check-kron.ps1 = r3 content (8CECBABC)**, Button 8/8
  with node AND walletd pinned; brick experiment running since 09-02 14:12.
- **zkas_blocks**: kaspa_double column of record; 1,420 rows / 68,146.61
  zKAS / 84.4% doubles at 03:45Z 09-06; dashboard shows the same numbers
  (page cap fixed, RPC zkas_blocks_total_amount).
- **Mailbox**: both inboxes CLEAR (rows 14–23 acked on both sides).
- **MacBook wallet 1.0.32** (asset sha d49597a1…; badge v1.0.32; About/
  plist say 1.0.31-1 — trust badge + asset sha only). Balance gate passed.
  **zkas-wallet#3 OPEN** (pinned 1fe41ce1…, comment #1 posted).
- **Upstream**: zkas-rusty#6 CLOSED 09-01; #8 PR set open (BL-101).

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **zkas-wallet#3** — maintainer reply? `gh issue view 3 -R
   firecash/zkas-wallet --json state,comments --jq '.state + " · " +
   (.comments|length|tostring)'`. Interim on every launch: Own node +
   Shielded history first.
2. **T-7** — 24h double rate at ~90% yet? One SQL read.
3. **H2(a) preps** (daytime Kron sitting): nologfiles=false on BOTH nodes;
   versioned kaspad launcher. **H2(b)** needs a multi-hour window.
4. **Rules sitting** (R-1..R-5, R-7): walletd distress alert
   (missing_history-aware), ZkasLegDegraded, swap rule, firewall
   rationalization, RcReporterDown drill, TG-WARN widening.
5. **Arrivals** — Pi kit (T-1) · third 1500VA (T-2).
6. **Brick / ERRSTREAM** — any 41/6008? any flash?
7. **SESSION-CONDUCT-LAWS v2-r2** — Appendix B rows I-16..I-25 owed.
8. **Dashboard P11** (stale-pending aging · expected-vs-pace · kron_events
   timeline) — next mining-dash handoff; the miner-health "16 effective"
   line rides with it.

## STANDING QUEUE

Per ITEM-REGISTRY-r6: H2 → rules sitting → P11 → P7/P8 → docs sweep →
upstream remainder → cold-storage sweep → fork rebase.

## HANDOFF NOTES

Two corrections this session were the operator's, not the record's (I-25;
the row-16 diagnosis was right after all). The walkthrough-with-lsof
method is now the standard for any claim about what the wallet app does.
