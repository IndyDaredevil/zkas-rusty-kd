# SESSION-STATE — 2026-09-07 (supersedes 2026-09-06; written 03:40Z 09-07 = late 09-06 EDT)

Written at S25 close. Since the 09-06 doc: laws v2-r2 on five rails and
adopted by covenants via handoff_docs; rows 24–30 closed; P7 answered from
persisted data. ITEM-REGISTRY-r7 holds definitions.

## LIVE STATE (verified this date)

- **Kron**: v1.0.8 everywhere; reporter r6; check-kron = r3 content, Button
  8/8 node+walletd pinned; brick experiment since 09-02 14:12.
- **zkas_blocks**: 1,420+ rows, kaspa_double column of record, 212/212 paid
  since the 09-02 restart. Dashboard: page cap fixed, pending aging live,
  efficacy line gone.
- **Mailbox**: both inboxes CLEAR (last inbound row 29; row 30 acked by
  mining-dash 05:20Z 09-06).
- **Laws v2-r2**: repo 353801c · project instructions · container · handoff_docs
  (f41f8591…) · covenants adopted (row 29). mining-dash N/A (row 30).
- **Wallet 1.0.32**; zkas-wallet#3 OPEN, no maintainer reply as of this doc.
- **Upstream**: zkas-rusty#6 CLOSED; #8 PR set open.

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **zkas-wallet#3** — reply? `gh issue view 3 -R firecash/zkas-wallet
   --json state,comments --jq '.state + " · " + (.comments|length|tostring)'`
2. **T-7** — 24h double rate at ~90%? One SQL read.
3. **Row 27 screenshot** — a pending row and the miner-health panel; closes
   P11's first half.
4. **H2(a)** preps (daytime Kron sitting): nologfiles=false BOTH nodes;
   versioned kaspad launcher. **H2(b)** needs a window.
5. **Rules sitting** R-1..R-5, R-7.
6. **P7 card** — optional handoff: chain share by week with the Toccata
   (06-30) and custom-bridge (08-02) markers.
7. **Arrivals** — Pi kit (T-1), third 1500VA (T-2). Brick/ERRSTREAM soaks.
8. **P11 remainder** — expected-vs-pace; kron_events timeline.

## STANDING QUEUE

Per ITEM-REGISTRY-r7: H2 → rules sitting → P11 remainder → P8 → docs sweep
→ upstream remainder → cold-storage sweep → fork rebase.

## HANDOFF NOTES

Three incidents this session were process, not data: a whole-document paste
(I-26), a row to a lane without the laws (I-27), a clock narrated from
recollection across a day gap (I-28). Read the clock. The P7 method —
find the dashboard's rule in code, replicate it to the tenth, then split on
documented dates with a network event as control — is the template for
any "did our change work" question on the KAS table.
