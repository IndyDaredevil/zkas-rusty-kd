# SESSION-STATE — 2026-09-25 (supersedes 2026-09-11; written ~05:40Z 09-25 = 01:40 EDT)

Written at S30 close. Since the 09-11 doc: S29 + S30 banked (BL-122..127,
ledger 4272 ln 3d55c576…). Two production cutovers on Kron, both in place,
no resync, no rollback: **stratum-bridge v2.0.1.6** (09-24) and **kaspad
v2.1.0** (09-25). Branch line consolidated to `merged-v2.0.1.6`. The 09-11
LIVE STATE items not listed below were NOT re-read in S29/S30 and carry
forward as "as of 09-11", not as verified.

## LIVE STATE (verified S29/S30, instrument named)

- **kaspad v2.1.0** — `C:\rusty-kaspa-v210\kaspad.exe` 16BD6824…1DA via
  `run-kaspad-v210.cmd` (first versioned kaspad launcher); config.toml
  unchanged; DB version 7 both tags → no resync (synced ~18 s, bridge
  status line 01:05:58 EDT 09-25). Firewall: program+port rule "kaspad
  v2.1.0 P2P inbound" (16111) pre-minted; no first-run prompt. Rollback
  operand untouched: `C:\rusty-kaspa-v2\target\release\kaspad.exe`
  084C2B92…2743 (v2.0.1). kaspad DOES write logs (nologfiles=false;
  EVAL r1 §3.3 corrected in BL-126).
- **stratum-bridge v2.0.1.6** — production path
  `C:\Users\inmyh\zkas-rusty-kd\target\release\stratum-bridge.exe`
  82CC5917…71CF (CI asset of release v2.0.1.6-win, tag eac767d); banner
  `RC merged bridge v2.0.1.6 (engine 2.0.1)`; pid 7004 owns 5755/5765/3034.
  B1 static-path traversal CLOSED (probe 200 → 404, 09-24); B2 accept
  retry, B3 Connection: close, B4 `ip` label host-only (live series
  ip="192.168.1.21", zero host:port series), B5 K/Z/D gauges capped 512.
  NOT ported: upstream `resolve_miner_label` (would collide with the
  IceRiverMiner drop relabel — BL-124). Rollback operand beside it:
  `.bak-v2015` F1484FB5…A3F0.
- **Button** (check-kron r3, elevated, 09-25 after cutover): ALL 8 UP.
  FINDING: kaspad (and bridge) are checked by presence/port only, not
  path+sha — a wrong exe would pass (BL-126). Item: check-kron r4.
- **Repo** `IndyDaredevil/zkas-rusty-kd`: `merged-v2.0.1.6` @ 3173c72
  (b5bd7e3 = --no-ff merge of merged-v2.0.1.5@2cdf081 + ws2@eac767d; then
  the S30 docs commit). `merged-v2.0.1.5` frozen at 2cdf081. Tag
  v2.0.1.6-win @ eac767d. `ws2-b1-static-traversal` kept (delete
  undecided). bridge-check 35960021888 ✓; `Tests` workflow is red on
  every branch since ≤09-11 (Check no_std / num_cpus) — pre-existing, not
  a bridge gate. The MacBook clone is a SPARSE checkout (no `bridge/`
  working files; `git am` applied via index — tree hash was the identity).
- **Mount**: ENGINEERING-LEDGER.md (4272, 3d55c576…), STARTUP-ORDER-r4.md
  (180, eff2c96a…), FLEET-DEPLOY-v2.0.1.6-r1.md (125, f53cccd1…),
  KASPAD-EVAL-v2.1.0-r1.md — all under the panel's `claude/` prefix;
  STARTUP-ORDER-r2 deleted. ITEM-REGISTRY on the mount is r7 vs r8 in the
  repo (stale; r9 owed).
- **Upstream kaspa**: rusty-kaspa v2.1.0 = tag 01b532e8…; zip
  FB25743A…8EC0F. **firecash/zkas-rusty** main @ 07ed6dd (09-22) still
  version 2.0.1, v2.1.0 NOT an ancestor → NODE-CONTRACT trigger unfired.
  zkas tags v1.0.9 / v1.0.10 (e583cbf, 09-18) exist; Kron runs v1.0.8.

## AS OF 09-11 (not re-read in S29/S30)

zkas-node/walletd v1.0.8 (Button says pinned + up 09-25 — presence, not
a re-read of health); reporter r8; samplers r2; alert_rules 48 rules;
dashboard/edge functions/Supabase/treasury/hardware/wallet/upstream-PR
state per SESSION-STATE-2026-09-11. Its opening questions 1–8 are still
open unless answered elsewhere; none were touched here.

## OPENING QUESTIONS FOR THE NEXT SESSION

1. **Clean-day gate for retiring rollback operands** (BL-031): Button
   8/8, bridge `zk=ok`, KronProcessRestarted count over 24 h = 0 for
   kaspad and bridge? If yes: delete `.bak-v2015` and note the v2.0.1
   kaspad exe as retired (leave the source tree).
2. **check-kron r4** — add path+sha pins for kaspad (16BD6824…) and
   bridge (82CC5917…); rev in the same window as any pinned artifact.
3. **zkas-node v1.0.9/v1.0.10 evaluation** — same shape as KASPAD-EVAL
   (tag-to-tag read, DB/config/RPC deltas, NODE-CONTRACT revision if
   warranted).
4. **Record hygiene, one commit**: ITEM-REGISTRY r8→r9 (B1–B5 closed at
   birth; checker gap; Defender exclusion for C:\rusty-kaspa-v210
   (BL-048 list names C:\rusty-kaspa-v2); ws2 branch keep/delete);
   SESSION-CONDUCT-LAWS r2→r3 with Appendix B rows I-26..I-31 (project
   instructions still say "last incident I-25"); prometheus.yml comment
   noting B4 landed (rules unchanged, `sum without (ip)` now a no-op).
5. The 09-11 questions 1–8, unchanged.

## STANDING QUEUE

Clean-day retirements → check-kron r4 → zkas v1.0.10 eval → record hygiene
→ H2 service migration (kaspad launcher prerequisite now met) → 09-11
queue (rules sitting, reporter r9, Pi T-1a, upstream remainder, fork
rebase once firecash carries v2.1.0).

## HANDOFF NOTES

S29/S30 incidents were all in the instruction channel, not the systems:
I-26 (cwd assumed in a fence), I-27 (second workflow gate not read), I-28
(stalled cargo holding the lock), I-29 and I-31 (a defective fence shipped
beside its correction — twice; the operator's rule: a known-bad fence is
deleted from the message, never annotated), I-30 (stale local ref in a
simulated merge). Mechanisms adopted: literal paths after a find; `+` on
every fetch before a simulation; one fence per step, no "not that one";
long commands split so a terminal line-wrap cannot truncate them (the
step-2 paste broke at a wrap once); commit messages via `-F` file. The
consumer read before B4/B5 found the one upstream change that would have
blanked the fleet from Prometheus — read this side's consumers before
calling any port "the same fix".
