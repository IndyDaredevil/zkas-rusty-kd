# SESSION-STATE — 2026-09-05 (supersedes 2026-09-03)

Written at S23 close. Since 09-03: S21 (TG cards r4→r5, dashboard sitting,
BL-099), S22 by a parallel session (turnstile verdict, zkas-rusty#8 PRs,
incidents I-16..18, BL-100..102), S23 (mailbox live, kaspa_double column of
record, BL-103..105). ITEM-REGISTRY-r5 holds definitions; this is the
snapshot + opening questions.

## LIVE STATE (verified this date)

- **Reporter r6** (f82bf0c2…) live since 05:16Z: TG block cards (birth at
  BEAT1, edit-in-place at BEAT2, 🎉 DOUBLE tag), kaspa_double on both beats,
  fail-open. Card doctrine: buzz every block + 24h digest.
- **zkas_blocks.kaspa_double is the column of record**: 1,371 rows,
  1,158 true / 213 false / 0 null, 84.5% all-time (backfilled from 63
  bridge logs, 0 mismatches). Webhook passes the field through unchanged.
- **Mailbox live** (public.project_handoffs; convention r3 sha 05074912…).
  zkas-node inbox EMPTY. Outbound to mining-dash: **row 21** (supersedes
  16–20): page-cap fix → double card from full table → drought-card
  visibility. Unread until Michael sends mining-dash to the mailbox.
- **Dashboard defect open**: 1000-row page cap; all-time stats truncated
  (66,011.28 zKAS / 1,371 rows vs shown 46,766.59 / 1,000). Row 21 ask 1.
- **v1.0.8 everywhere**; H8 CLOSED (BL-098); stranded tip gone; Button
  8/8 (v108 pin); brick experiment running since 09-02 14:12.
- **Upstream**: zkas-rusty#8 PR set open (BL-101); #6 closed.

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **Mailbox** — has mining-dash read row 21? Its reply carries the
   page-cap approach + all-time count/total (check vs 1,371 / 66,011.28 +
   since) and the double card's numbers (expect ~84.5% all-time).
2. **T-7** — SQL 24h double rate converged toward the counters' ~90%?
   `select round(100.0*count(*) filter (where kaspa_double)/count(*),1)
   from zkas_blocks where found_at >= now()-interval '24 hours';`
3. **r6 first-days**: any `WARN TG` beyond the known edit-miss class?
   `double=` on every BLOCK line? pending 0 / failures 0?
4. **Brick experiment / ERRSTREAM** — any 41/6008? any flash?
5. **Arrivals** — Pi kit (T-1) · third 1500VA (T-2).
6. **H2(a)** config preps still owed: nologfiles=false on BOTH nodes,
   versioned kaspad launcher; **H2(b)** needs a multi-hour window.
7. **Rules sitting** (R-1..R-7) — one evening; R-4 firewall, R-5 drill,
   R-6 checker reconcile + walletd pin, R-7 TG WARN widening.
8. **SESSION-CONDUCT-LAWS v2-r2** — Appendix B rows I-16..I-23 owed
   (BL-102 + BL-105).
9. **Wallet desktop default-to-public** (BL-105): upgrade 1.0.29→1.0.32
   (custody protocol), reproduce, file ISSUE-DRAFT-desktop-default-public
   upstream. Interim: Own node first on every launch.

## STANDING QUEUE

Per ITEM-REGISTRY-r5: H2 → rules sitting → P9/P10 (via row 21) → P11 →
P7/P8 → docs sweep → upstream remainder → cold-storage sweep → fork rebase.

## HANDOFF NOTES

Three sessions wrote the ledger 09-04→05; every cut re-pinned its base and
the tip moved twice (BL-099→102 between sweep and cut once). Sweep before
cut, mint ids only from a rail read (II.2). The prior-turn review on 09-05
retracted five claims (I-19..I-23); all corrected forward in-thread and in
the mailbox.
