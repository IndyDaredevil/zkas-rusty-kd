# ENGINEERING LEDGER — zKAS/KAS Merged Mining Operation
### Standing, append-only record of bugs fixed, major corrections, and lessons learned.
### Convention: new entries appended at session close with the next BL-### id.
### Session-state docs reference this file; do not duplicate its content there.
### Last entry: BL-099 (2026-09-04)

Format per entry: **Codebase/Domain · Symptom · Root cause · Fix · Lesson**

---

## Repository map (all repos referenced by ledger entries)

### Own forks (github.com/IndyDaredevil)
- **zkas-rusty-kd** — THE production codebase: RC v2.1 in-tree merged bridge
  (rusty-kaspa fork). Local: `C:\Users\inmyh\zkas-rusty-kd`.
  Branches: the production line IS the current `merged-vE.N.G.B` branch —
  each release branch becomes canonical on release, so there is NO
  merge-to-production step (lineage is strictly linear; verified 2026-08-20).
  Current: `merged-v2.0.1.4` @ `336b7a5` = origin = tag `v2.0.1.4-win` =
  running exe. Lineage: `233c5f7` (v2.0.1.2) → `10892ed` (v2.0.1.3) →
  `336b7a5` (v2.0.1.4). `merged-ws1-port` is RETIRED and is a BL-019-class
  trap for as long as it exists (building from it silently omits v2.0.1.1+
  features) — verify it is deleted.
  Workflows: `bridge-check.yaml` (push to `ws2-*`/`merged-*` → check +
  clippy-advisory + test, bridge crate only — THE compile gate for operators
  without local Rust), `deploy.yaml` (release published → win64 zip assets;
  target the branch in the release UI), `ci.yaml` ("Tests", full-workspace,
  ignorable noise), `musl-toolchain.yaml`. Releases: `rc-c15-canary`.
  Caveat BL-019: production exe is a release-zip copy sitting in
  `target\release` — local builds overwrite it silently.
- **zkas-pool-kd** — pool-bridge fork (v1.1.0 lineage). Retired from
  production (superseded by RC) but historically important: `d5c3e29` =
  the FCMM→ZKMM lockfile fix (BL-005), release `v0.3.3-win`, and origin of
  the c.15 near-miss line (BL-012).
- **zkas-rusty** — early node fork (`v0.3.2-win` tag). Node duty retired to
  official binaries (meta-principle 6); kept for history.

### Upstream (github.com/firecash)
- **firecash/zkas-rusty** — node upstream; official releases are the
  production node binaries (`zkas-v1.0.5` = win64 zip + zkas-anchor-pins.tsv).
  Key commits: `1650f35` FCMM→ZKMM rename · `424b7036` the broken lock pin
  (ancestor of rename) · `30492529` fixed pin target · `e3589f7` NU1 @ DAA
  757,000 · `f73a697` pin file · `deac95c` dev-fee accrual · `6c20e7a`
  rejectDetail RPC · `742b40b` --consensus-diag divergence reports.
  Our anchor-wedge issue lives here (BL-001).
- **firecash/zkas-pool** — pool/bridge upstream. Committed Cargo.lock still
  pinned to 424b7036 (BROKEN, BL-005) as of 2026-08-04 ~17:45 ET; our FCMM
  issue filed here. Watch: `d79bf68` "bridge: wire merged mining YAML into
  runtime" — upstream is implementing WS5 themselves; re-diff before porting.
- **firecash/solo-dual-mode** — operator installer kit. v1.0.6 release
  binaries are built from the VENDORED `bridge-src/` tree (ZKMM-correct,
  findstr-verified); the README's build-from-zkas-pool path reproduces BL-005.

### What zKAS is (chain identity, source-verified 2026-08-12)
A fresh-genesis (2026-07-26) sovereign chain built on **rusty-kaspa v2.0.1**
— the modern Toccata-era Kaspa node, same version as our production KAS node,
p2p protocol 10. Lineage: rusty-kaspa → firecash-rusty (added shielded pool +
AuxPoW; the `h_fc`/FCMM fossils) → zkas-rusty (July 2026 rebrand). It is NOT
pre-Crescendo code: mainnet params set `crescendo_activation: always()` AND
`toccata_activation: always()` — both Kaspa hardfork rule-bundles active from
genesis (launched already-forked, same pattern as merged_mining always()).
What *looks* pre-Crescendo is only the block rate: `Bps::<1>` — 1 block/sec,
classic Kaspa cadence on the modern engine, sensible for a ~19-node network.
Delta vs Kaspa: (1) Orchard shielded pool (Zcash-lineage: notes, nullifiers,
viewing/proving keys, and the shielded anchor index — BL-001's home, fixed by
their own first hardfork ZKAS-NU1 @ DAA 757,000); (2) shielded coinbase (one
zkas: output per mergeset blue); (3) AuxPoW merged mining (MERGE_MINE_MAGIC
"ZKMM" né FCMM, `ZKMM||H_fc` commitment in the parent Kaspa coinbase
extra_data, check_pow_gated native-or-aux, active from genesis);
(4) tokenomics: stepped subsidy 53.80083582, 5% dev fee accrued per ~1000 DAA
post-NU1, two-step tail (c0022c9); (5) own genesis/ports/HRP + one hardfork
of its own history already (NU1); (6) full Toccata script stack live from
genesis: covenants (`covenants_enabled`, crypto/txscript/src/covenants.rs),
KIP-10 introspection opcodes (OpTxInputSpk 0xbf family), in-script zk
precompiles (Groth16 + risc0), and KIP-21 seq_commit + OpChainblockSeqCommit
(0xd4) — the latter explicitly laid down at genesis for the planned trustless
KAS<->ZKAS bridge ("canonical-R": a covenant reads the chainblock's PoW-
committed shielded state root). External-tooling note: covenant IDEs/compilers
targeting Kaspa's Toccata opcode set (e.g. Silverscript) produce bytecode that
is semantically valid on zKAS as-is — the chain cannot tell what authored a
script; friction is tooling-level only (zkas: HRP/RPC plumbing, fork-drift if
upstream adds post-v2.0.1 opcodes, and the shielded pool stays opaque to
script except via seq_commit).

### Access patterns (sandbox-side)
Fine-grained PAT at `/home/claude/.gh_token` (re-provision per container —
absent after resets). Public clones work tokenless. Rate-limit fallbacks:
tarball diffing via `codeload.github.com`, pinned-SHA source reads via
`raw.githubusercontent.com`.

---

## Consensus & chain-level

**BL-001 · 2026-08-02/03 · firecash/zkas-rusty (node) — Anchor-index wedge**
Fresh node syncs wedged permanently: "N disqualified vs. 0 valid chain blocks",
"coinbase transaction is not built as expected", from the very first sync.
Root cause: last-write-wins shielded anchor index — sibling mergeset blocks share
a tree root; whichever a node indexes last wins; wrong (orphan) winner = permanent
divergence. Per-node, ordering-dependent. Published pin file (360 roots,
`--shielded-anchor-overrides=`) did NOT fix fresh syncs — we localized a
post-curation landmine to blocks timestamped ~09:14–09:40 ET Aug 3 (pins validated
100% clean until that window, then cliff to 0-valid). Reproduced identically on
their own official v1.0.5 binary. Fix (theirs): ZKAS-NU1 hardfork @ DAA 757,000
(multi-producer anchor resolution) + archival snapshot bootstrap. Ours: issue
filed with binary repro + localization.
**Lessons:** "IBD completed successfully" ≠ healthy — only advancing DAA +
UTXO-validated>0 count. The `=` is REQUIRED in `--shielded-anchor-overrides=<file>`.
A fix verified on the maintainer's own node (pins-on-wedged-state) can still fail
your case (fresh sync) — remediation paths are not interchangeable.

**BL-002 · 2026-08-04 · zKAS network — NU1 split**
Post-activation network split (pre-fork miners); nodes on the minority side showed
tip-lag + majority-rejection at ~0% CPU. Fix: dev's regenerated archival snapshot
(--archival MANDATORY for that snapshot; untested --yes would prune) + new peer
204.10.194.28:17951. Straggler "coinbase mismatch at daa X" warnings from pre-fork
binaries = benign per dev.
**Lesson:** during fork windows, re-download artifacts (same filename ≠ same file;
re-verify sha256) and treat "synced" claims with a canonical-tip cross-check.

**BL-003 · 2026-08-04 · Consensus understanding — mergeset deferred payouts**
"Our block paid the wrong address!" — twice. Not a bug: a block's coinbase pays the
miners of its MERGESET (blocks it merges), never its own miner; your reward arrives
in a LATER chain block's coinbase. Explorer block pages show what that block pays
OTHERS.
**Lesson:** verified end-to-end when the exact 53.80083582 note landed in walletd.
Also: zKAS subsidy is stepped, not continuously decaying (three OG blocks over
~1h45m, zero decimal drift).

## Bridge (zkas-pool lineage → zkas-pool-kd fork)

**BL-004 · 2026-08-02 · Windows build — cdylib link failure**
LNK2019 `sys_alloc_aligned` (risc0-zkvm-platform via kaspad→kaspa-wrpc-server
cdylib). Git-dep cdylibs get built on Windows and DLLs require full symbol
resolution (unlike ELF). Fixed twice-over: workflow cdylib-strip step +
`cfg(not(windows))` gating of the kaspad dep. Linux `cargo check` preflight cannot
catch link-class errors (documented gap, held exactly).
**Lesson:** [CORRECTED 2026-08-27 — see BL-043] in-workspace builds are
structurally immune to the LOCKFILE-DRIFT class only (the BL-005 shape); this
cdylib/risc0 LNK class lives in-workspace and took three fixes. Draft 4 §6
walked the broader claim back; this entry had inherited it forward.

**BL-005 · 2026-08-04 · zkas-pool Cargo.lock — FCMM→ZKMM magic bytes (THE bug)**
Every AuxPoW zKAS submission ever made by the bridge rejected "invalid
proof-of-work", deterministically, while the KAS leg worked and masked it.
Root cause: committed Cargo.lock pinned all kaspa-* crates to zkas-rusty 424b7036,
a git-ancestry-proven ancestor of rename commit 1650f35 (FCMM→ZKMM). Bridge embeds
the magic via `AuxPow::embed_commitment()` from the PINNED crate → literal "FCMM"
where the node's `MERGE_MINE_MAGIC = *b"ZKMM"`. Commit 1effba1 claimed the rename
but never moved the resolved lock. Evidence: node-side dump of rejected block
6bc36949...'s parent coinbase decoded to ASCII "2.0.1/FCMM6bc3694...".
Fix: `cargo update -p kaspa-consensus-core` (424b7036 → 30492529), commit lock,
rebuild (v0.3.3-win). Verified: findstr ZKMM hits / FCMM empty; then 6 zKAS +
2 merged-KAS blocks within hours. Scope: solo-dual-mode v1.0.6 RELEASE binaries
unaffected (built from vendored bridge-src); SOURCE builds per the README broken.
Reported upstream with working fix.
**Lessons:** git deps stay pinned until `cargo update` even with branch=main —
a rename commit that doesn't move the lock fixes nothing. "Works for me" reports
demand a provenance question before a code re-evaluation. Suggested hardening:
unit test asserting MERGE_MINE_MAGIC == *b"ZKMM" turns lockfile drift into CI red.

**BL-006 · 2026-08-03 · pool bridge — polling-listener fallback (design defect)**
WARN "Using polling-based block template listener (concrete KaspaApi not
provided)" — trait-object wiring silently drops push notifications; block_wait_time
becomes the actual job cadence. Mitigated 1000→200ms; real fix = WS2 of the port
(notification-driven templates; polling strictly fallback with loud WARN).
**Lesson:** silent capability degradation must log loudly; "the code has
notifications" ≠ "notifications are wired".

**BL-007 · 2026-08-04 · pool bridge — wallet capture semantics**
Two blocks paid the operator's OLD web wallet despite corrected rig configs.
Root cause: bridge captures the stratum username wallet ONCE at authorize
(default_client.rs:292, single write site); IceRiver config-page save does NOT
re-authorize a live session. Fix: rig reboot forces reconnect.
**Lesson:** wallet/worker changes on IceRivers require reconnect, not save.
Related: clean_wallet()/WALLET_REGEX only match kaspa: prefixes; zkas: addresses
take a fallback-tolerant path (empirically flows the authorized address through
correctly; exact acceptance path never fully traced — never set
POOL_FALLBACK_ADDRESS).

**BL-008 · 2026-08-04 · pool bridge — table counts zKAS blocks only**
`Blocks` column increments only in record_block_found (native zKAS claim); the
Kaspa-parent leg lives solely in ks_merged_parent_submit_total metrics. Not fixed
in the pool bridge; addressed in the RC's K/Z/D table design.

## Bridge (RC, zkas-rusty-kd — production line, engine-prefixed versioning)

**BL-009 · 2026-08-05..11 · coinbase_tag_suffix "ZKMM" collision**
A user-configurable suffix of literally "ZKMM" passed sanitization and killed 100%
of merged submissions (committed-hash scan finds the wrong ZKMM). Guard implemented
and committed.
**Lesson:** sanitizers must reject the protocol's own magic strings.

**BL-010 · 2026-08-05..11 · monitoring — `ip` label includes ephemeral source port**
Every worker reconnect minted a permanent new Prometheus series instead of reusing
the existing one. Fix: drop-relabel. (A later revert appeared in branch history —
"monitoring: revert ip port-strip rela..." — verify final state when touching
monitoring.)

**BL-011 · 2026-08-12 · c.14 "IMPLAUSIBLE ratio" anomaly — RESOLVED as a math
correction, not a code bug.** Session-best near-miss ratios (historical 63.1%,
today's 0.04–0.7%) flagged IMPLAUSIBLE by the >1e-6% forensic threshold.
c.15's independent BigUint event gate cross-checked the f64 latch path:
same-millisecond agreement (latch 7.11e-1% vs event k=0.7105%), bits-decode of the
forensic line's own numbers internally perfect (0x1a024bed → d=1.56e16 ≈ status
line's 1.57e16). Conclusion: ratios were always correct; the threshold dropped the
2^32 stratum-diff factor — a diff-1024 share FLOORS at ~0.028% of target, five
orders of magnitude above the "implausible" line, so it fired on everything.
The 63.1% was a genuine near-block (P≈1/2250 shares — inevitable over days).
Follow-up queued: c.16 recalibrates threshold (>100%, or >50% for near-block
pages) and softens wording.
**Lessons:** before debugging data, debug the expectations coded around it.
An independent-path cross-check (integer vs float) localizes "impossible" values
in one observation. The c.12 design comment's units error ("ratios in 1e-9%
range") also mispriced event-line frequency — design comments deserve the same
verification as code.

**BL-012 · 2026-08-12 · c.15 near-miss event line — shipped**
Ported the v0.3.3 per-event near-miss line into the RC as a pure-BigUint gate,
deliberately independent of the c.12 f64 path. Canary (:5775, w1c, 12 min):
16 events @1.36/min, k∈[0.1041,0.7105] (floor visible at the gate's mathematical
0.0977% minimum), z/k=0.949–0.979 stable, session-best flow undisturbed, 77/0/0.
All predictions hit. Deployed to production same day.

**BL-030 · 2026-08-20 · version identity — BL-019's banner lesson closed
STRUCTURALLY (v2.0.1.4)**
Standing weakness: "the version banner is only a valid identity instrument if
bumping it is part of the release checklist" — and v2.0.1.3 duly shipped a
binary whose banner still reported the prior version, because the bump was a
manual step and manual steps get skipped.
Fix, both halves shipped together: (a) the banner now DERIVES from
`CARGO_PKG_VERSION` + a `BRIDGE_BUILD: u32` ordinal, replacing the hardcoded
`BRIDGE_RELEASE` constant, so the version cannot drift from the manifest;
(b) a `deploy.yaml` guard extracts the release tag and the banner string at
release time and FAILS THE BUILD on mismatch (canary-tag-aware,
substring-safe; empty extraction fails loud rather than passing silently).
Verified in production 2026-08-20 12:50:20 —
`RC merged bridge v2.0.1.4 (engine 2.0.1)` on first launch of the new binary,
which is also the deploy pipeline's own proof that the copy landed.
**Lesson:** a checklist item that can be forgotten belongs in CI, not in the
checklist. The engine prefix echoed in the banner doubles as a rebase-drift
alarm — if it ever disagrees with the node engine, the tree was rebased onto a
different kaspad lineage.

## Monitoring & alerting

**BL-013 · 2026-08-04 · Prometheus — series-birth blindness**
`increase()` cannot see a NEW series' first increment (born mid-window at value 1
= delta 0). First double-payday fired only ZkasBlockFound (ks_blocks_mined is
warm-up zero-inited per worker; ks_merged_parent_submit_total is not). Fix: the
`X unless X offset 1m` birth idiom on MergedKaspaBlockFound/-Rejected +
DoubleBlockFound (widened to 2m). Elegant fix queued for the port: warm-up
zero-init the merged counters.
**Lesson (generalized as the "birth-clause" pattern):** any `unless offset` /
increase-based alert on zero-initialized counters needs a `> 0` guard or birth
handling — false fires on every series birth otherwise.
**>> SUPERSEDED 2026-08-18 — see BL-025. The birth clause was DELETED from all
five block rules; its premise (warm-up zero-init never reaches the TSDB) went
silently false when the bridge changed. Do not reapply this pattern.**

**BL-014 · 2026-08-05..11 · amtool check-config does not validate bot_token_file**
The file is read at send time, not config-check time — a green check-config can
still mean dead Telegram alerts.

**BL-015 · 2026-08-10 · missing-alert diagnostic (open at last session)**
KAS block mined 23:59:40 appeared on-chain, no Telegram alert. Leading hypothesis:
process restart at midnight boundary killed the metric series before the `for: 30s`
threshold completed; secondaries: double-block inhibit rules (added 08-09),
group-flush timing, birth-clause aggregation. Verify resolution status.

**BL-016 · luck metrics — two standing artifacts**
(a) Luck%/Rig figures are meaningless until a full uninterrupted 24h window
(rate-window vs restart math — produced both 168.89% and 1.41% artifacts).
(b) By design, our Luck uses NAMEPLATE hashrate (14.2 TH/s) as denominator — a
capture-efficiency KPI, not pure Poisson luck; ~6% nameplate-vs-effective gap
means 1.05x nameplate ≈ 0.99x Poisson. Also: zKAS network hashrate was once pinned
stale at 18.06 PH/s making zKAS Luck read ~1.9x low — corrected to ~34.2 PH/s;
bridge should export a LIVE gauge (roadmap).
**>> CLOSED 2026-08-20 — see BL-028. The gauge shipped in v2.0.1.4; every pinned
value including 34.2 is retired, and pinning itself is the error (D_z/D_k moved
9% in 20 minutes).**

**BL-024 · 2026-08-18 · Prometheus — a FAILED SCRAPE truncates lookback, and
`up` cannot tell you it happened (THE bug of this session)**
Symptom: a Telegram storm of 21 phantom block cards — RcKasBlockFound,
RcZkasBlockFound and RcMergedDoubleBlockFound, seven workers each, one card per
alertname (group_by alertname + `{{ range .Alerts }}` renders seven alerts as ONE
message), against zero real solves.
Root cause, measured end to end: the bridge's `/metrics` render was hitting the
12s `scrape_timeout` ceiling — `max_over_time(scrape_duration_seconds[8h]) =
12.001298s`, i.e. killed by the timeout, not a natural maximum. Every failed
scrape writes a staleness marker, and **a staleness marker terminates lookback**.
So `<counter> offset 2m` evaluated across one resolves to EMPTY while the live
counter is nonzero — and BL-013's birth clause (`X > 0 unless X offset <w>`)
fires on every nonzero counter on every worker at once. ONE failed scrape is
sufficient: no outage, no restart, no label churn, no bridge fault.
Evidence (two independent instruments agreeing exactly):
  `count_over_time(up{job="rc_merged_bridge"}[8h])`      = 1918 attempts
  `count_over_time(ks_double_blocks_mined[8h])`          = 1885, ALL SEVEN
                                                            workers identical
  -> 33 missing counter samples == 33 `up == 0` buckets, to the sample.
The identical count across all seven also ruled out `ip` churn as a contributor
in that window (churn would desynchronise one worker's count).
Fix: (a) DELETE the birth clause from all five block rules — see BL-025 for why
that is now safe; (b) `scrape_timeout` 12s -> 14s (mitigation only; 14s is the
hard ceiling, it must stay <= the 15s `scrape_interval`); (c) NEW RcScrapeFlaky,
since RcBridgeDown's `for: 2m` structurally cannot see scattered single-scrape
failures — 33 of them passed with RcBridgeDown never leaving `pending`.
**Lessons:**
- **`up` is SYNTHESIZED by Prometheus on every scrape ATTEMPT and exists at
  value 0 on failure.** `count_over_time(up[6h])` therefore returns a PERFECT
  sample count through a total target outage. It measures whether Prometheus was
  awake, NOT whether data arrived. This produced a false all-clear mid-session
  and nearly closed the investigation on the wrong conclusion. To detect data
  gaps, count the SUBJECT series and diff against `up` — never `up` alone.
- Any rule reading `offset` is only as sound as the scrape's continuity.
  Intermittent scrape failure is a distinct failure class from target-down and
  needs its own alert; a `for:` long enough to debounce an outage is by
  construction blind to it.
- A max that lands within microseconds of a configured ceiling is the ceiling,
  not a measurement.

**BL-025 · 2026-08-18 · monitoring — BL-013's birth clause RETIRED (warm-up
zero-init reaches the TSDB after all)**
BL-013 added `X > 0 unless X offset <w>` because `increase()` cannot see a
series' first increment (born mid-window at 1 => delta 0). That premise no longer
holds. Measured on `:3034`, post-drop and post-labeldrop:
`ks_blocks_not_confirmed_blue` and `ks_zkas_blocks_not_confirmed_blue` both read
**series=7 zeros=7** — counters that have NEVER incremented on ANY worker, yet
all seven series exist at zero. So the RC's warm-up zero-init reaches Prometheus
in the COUNTING label context (`miner=""`), not only in the
`miner="IceRiverMiner-v1.1"` context that `prometheus.yml` drops. Series are born
at 0 and `increase()` sees 0 -> 1 unaided.
The birth clause therefore had **no true-positive path left**; its only remaining
behaviour was firing on already-nonzero series (BL-024). Deleted from all five
block rules. Range windows widened 1m/2m -> 3m at the same time (BL-026).
Residual risk accepted: a block found within one scrape interval of a worker
authorizing would be born at 1 and go unseen. Requires a solve inside 15s of a
rig connecting; at ~2 blocks/hr fleet-wide, negligible, and the hourly card
reconciles it.
Retained: `sum without (ip)` wrapping every block expression — the RC never
retires a session's series, so a reconnect leaves the retired series in the SAME
scrape as the live one. `increase()` goes INSIDE the sum (per-series first, then
summed); inverting that makes a stale series' expiry look like a counter reset.
**Lessons:** a guard written against a bridge behaviour must be re-verified when
the bridge changes — BL-013's premise was true when written and silently false
later. `series=N zeros=N` on a counter that has never fired is the one-query test
for whether zero-init survives the relabel pipeline. Note also that `wallet` is a
SECOND latent churn vector (captured once at authorize, BL-007) — harmless now,
but do not reintroduce a per-series zero test without accounting for it.

**BL-026 · 2026-08-18 · Alertmanager — inhibition never worked, and the obvious
fix silently DELETES notifications**
Symptom: every double produced three Telegram cards (KAS + ZKAS + DOUBLE),
singles first, despite a correct inhibit rule (`source_matchers` /
`target_matchers`, `equal: ['worker']`).
Root cause: **timing, not matchers.** The prior config comment claimed "all three
rules live in the same rule group, so they reach Alertmanager in a single POST."
False. Same rule group = same EVALUATION cycle, not same FIRING time — and the
counters do not increment together: `ks_double_blocks_mined` cannot increment
until BOTH legs confirm blue, and the blue-confirm loop is 30 x 2s. Alertmanager
evaluates inhibition AT NOTIFICATION TIME, and cannot recall a message already
sent.
Second trap, found before shipping: raising `group_interval` to cover the skew
can silently swallow block cards entirely. These alerts only stay FIRING while
the increment sits inside the rule's range window, so the firing span is
`(window - for:)`. At `[1m]` minus `for: 30s` that was ~30s — and with
`send_resolved: false`, a group flushing after its alerts resolve notifies
NOTHING. A 120s `group_interval` against a 30s firing span would have traded
triple-cards for zero cards.
Fix: widen the block rule windows 1m/2m -> **3m** AND set info route
`group_wait`/`group_interval` to **90s**. Fires ~T+37s, resolves ~T+187s, worst
flush T+127s — ~60s margin. Also: `group_interval`, not `group_wait`, governs
steady state, because `group_by: ['alertname']` means the group is created once
and persists for the process lifetime; every later block joins an EXISTING group.
Raising `group_wait` alone would have fixed only the first block after a restart.
Cost accepted: two blocks on the SAME worker inside 3 min render as one card
(~3% at w9m's rate).
**Lessons:** alert_rules.yml and alertmanager.yml are COUPLED — a range window is
a notification deadline, and changing one without the other can delete cards
rather than merely delay them. Inhibition requires the source alert to be firing
at the target's flush instant, which is a statement about counter increment
ORDER, not about rule-file layout. STATUS: UNVERIFIED IN PRODUCTION — the next
real double is the test, and the failure mode to watch for is SILENCE (block
cards stop while K/Z/D keeps climbing), not noise.

**BL-028 · 2026-08-20 · zKAS network hashrate — the PIN is not stale, it is
UNPINNABLE (BL-016(b) closed by retiring the concept, not by re-deriving it)**
v2.0.1.4 shipped `ks_zkas_network_difficulty_gauge` /
`ks_zkas_estimated_network_hashrate_gauge` (30s stats loop mirroring the KAS
leg). First live readings retire every constant we ever used:
  08-19 measured 28.02 PH/s · pin was 34.2 PH/s (~22% high)
  08-20 12:5x  D_z 9.193e15 -> 19.42 PH/s
  08-20 13:1x  D_z 1.0016e16 -> 20.51 PH/s   (+9% in ~20 minutes)
i.e. ~30% drop in a day on a ~19-node network (one sizeable miner leaving moves
it), and the retired 34.2 pin would now read ~70% high.
**The free instrument:** for a SINGLE share the near-miss pair satisfies
k/z = T_k/T_z = D_z/D_k EXACTLY, independent of how lucky that share was. So
every dual-leg near-miss line is a ratio measurement at zero cost. Four
independent confirmations inside one hour, two code paths:
  session-best pair   2.40 / 4.08          = 0.588
  gauges (D_z/D_k)    9.193e15 / 1.55e16   = 0.593
  12:55 zKAS solve    84.0878 / 140.2743   = 0.5995
  13:13 full double   974.8795 / 1502.4410 = 0.6489   (gauges then 0.646)
Prior day the same ratio was 0.917/0.922. It moves, and it moves fast.
**Fix/consequence:** the Luck denominator must read the gauge, not a constant;
the "re-derive the pin to ~31-32 PH/s" open item is CLOSED as misconceived.
Same two solves also gave the third and fourth production confirmations of
v2.0.1.2's winner exclusion: `clears=Z` at 140% and `clears=BOTH` at 1502%
both passed with the chain's session-best UNMOVED and zero false FORENSIC
IMPLAUSIBLE.
Minor open: estimated_hashrate/difficulty read 2.11 then 2.048 against the
theoretical 2.0 for 1 BPS — consistent with the estimator using an OBSERVED
blockrate window rather than the target rate. Drift, not a constant offset;
low priority.
**Lessons:** a constant is a measurement with the timestamp deleted. Before
building an instrument, check whether an existing log line already contains
the quantity — the near-miss pair had been printing D_z/D_k on every share
since c.15 and nobody read it as such.

**BL-029 · 2026-08-20 · Prometheus — `scrape_duration_seconds` is plausibly
UPTIME-dependent, not load-dependent (CLOSED 2026-08-27 — causal claim REFUTED, see BL-033)**
BL-024 measured `max_over_time(scrape_duration_seconds[8h]) = 12.001298s`, at
the 12s timeout ceiling, on a process with days of uptime. Post-v2.0.1.4
restart the same target reads `[15m] = 0.0057s`, `[30m] = 0.0071s` — three
orders of magnitude lower.
These numbers are NOT comparable and the low one is NOT an all-clear: the
process had <1h uptime. Mechanism that predicts exactly this shape: the RC
never retires a worker's series, so every reconnect adds series to the
`/metrics` render and render cost grows with UPTIME, independent of hashrate
or share load. The phantom-card storm would then be the tail of one such climb,
reset by every restart.
Test: re-run at `[8h]` after several days of unbroken uptime. If it is
climbing, more timeout headroom is NOT available as a fix — 14s against a 15s
`scrape_interval` is the hard ceiling (BL-024) — and the real remedies are
series retirement in the bridge or a longer scrape interval.
Exonerated: v2.0.1.4's new 30s stats loop is a BACKGROUND loop writing gauge
values, not work on the render path, so it is not a contributor.
**Lesson:** a duration measured on a freshly restarted process cannot clear a
ceiling that was hit on an old one. Any "we fixed it" reading taken after a
restart must state the uptime alongside the number, or it is not evidence.

## Operator environment & process

**BL-017 · 2026-08-04 · cmd.exe — `set` without `=` is a query, not an assignment**
`run-merged-example.cmd` lost `ZKAS_KASPA_PAY=`; bridge silently ran aux-only
(fail-safe confirmed in source: empty → None client → no misdirection possible).
**Lesson:** the two ENABLED startup lines are the contract — read them every launch.

**BL-018 · 2026-08-04 · PATH collision on launcher scripts**
Running the .cmd by full path from another cwd executed a DIFFERENT
stratum-bridge.exe found on PATH (an old kaspad-1.1.0-bundling tool) → AddrInUse
crash against production ports 16110/16111 (production unharmed — it already held
them). Fix: `cd /d %~dp0` as the launcher's first line, now standard.

**BL-019 · toolchain/process — build ≠ deploy**
The running process must be killed explicitly before relaunch; a sub-second cargo
"Finished" means stale tree. Relaunch ONLY via run-rc-merged.cmd (env-var baking:
ZKAS_MERGED_NODE + ZKAS_TREASURY_ADDRESS both required or silent plain-mode).
Corollary (2026-08-12): production runs a release-zip exe copied into
target\release — the next local cargo build will silently overwrite it; keep the
branch merged so source matches binary.

**BL-020 · git — fetch before trusting status; verify the running artifact**
`git status` reports against last-fetched refs (bit us on multi-machine repos).
Correct file on disk + a 200 reload response can mask stale running configs —
Prometheus /api/v1/rules is the only reliable check. Downloaded files consistently
land in the wrong directories — verify path before debugging contents (this drove
the inline-format-patch delivery preference).

**BL-021 · 2026-08-12 · Windows patch delivery — two new traps, both solved**
(a) `git apply` failed on the operator's checkout: CRLF working tree vs LF patch
context (byte-exact matching). (b) The PowerShell fallback failed first try:
.NET `[IO.File]::ReadAllText` resolves RELATIVE paths against the PROCESS cwd
(system32), not PowerShell's location — safety rails (anchor-count assert) caught
it with zero writes. Final pattern that works: PowerShell here-string → direct
anchored insertion with EOL detection + absolute paths + fail-loud asserts, then
CI (bridge-check on merged-* branches) as the compile test, release-tag targeted
at the branch as the build, canary on :5775 as the runtime test.
**Lesson:** for an operator without local Rust, the pipeline IS the toolchain —
and every script must assume nothing about cwd or line endings.

**BL-022 · IceRiver fleet facts (validated)**
Pre-authorize extranonce handshake required and works; priority-failover mode (not
round-robin) is the correct rig config — a merged-rig appearing on the production
RKStratum dashboard IS the failover alarm; firmware may briefly test backup pools
on boot (sub-minute flicker = normal). Peer sweeps: KAS 42 out/8 in, zKAS 16/8,
validated on Crescendo+Toccata; knees are hardware/load-dependent — re-sweep after
full merged cutover.

**BL-023 · security/network boundaries (standing)**
NEVER forward 16810 (zKAS RPC) — p2p 16811 only; RPC stays loopback. Windows
firewall rules: plain "Allow", all profiles (Public included — adapter
classification is the classic silent-failure). Walletd passphrase sits plaintext
in launch scripts — accepted tradeoff on a single-user box; seed on paper is the
real recovery.

**BL-027 · 2026-08-18 · diagnostic process — the investigation was wrong three
times before it was right**
Kept because the failure pattern is more instructive than the fix. Successive
root-cause hypotheses for the phantom storm, each killed by evidence:
  1. `ip` label churn re-minting live series — killed by one clean `ip` per
     worker, no retired duplicates.
  2. Prometheus scrape gap > 5m lookback — killed by `count_over_time(up[6h])`
     = 1438/1440... which was itself a FALSE all-clear (BL-024), so this one
     was killed for the wrong reason and had to be revisited.
  3. Alertmanager restart flushing held state — killed by `resets()` and by the
     ALERTS series showing genuine pending->firing transitions in Prometheus.
  4. (correct) Single-scrape staleness truncating lookback.
What actually cracked it: counting the SUBJECT series and diffing against the
scrape attempt count — two instruments producing the same number (33) with no
fitting. What cost the most time: reasoning from the ledger and from remembered
config instead of reading the deployed artifact first. The `> 0` guard was
diagnosed as MISSING when it was present and documented in the file as
load-bearing.
**Lessons:** read the artifact before naming a cause — meta-principle 1 applies
to configs, not just code. An explanation that fits all observations is not
thereby correct; prefer the query that would FALSIFY it. When a hypothesis is
killed, re-examine the instrument that killed the PREVIOUS one — a false negative
propagates forward silently. Absence of evidence in a shared screenshot is not
evidence of absence (the "only doubles fired" inference was wrong; singles fired
too and were simply not in frame).

**BL-031 · 2026-08-20 · Windows file hygiene — two traps in cleaning up the
anchored-insert rail's own backups**
(a) **`Copy-Item` PRESERVES the source mtime.** A `.bak-*` file's
`LastWriteTime` therefore describes its CONTENT date, not when the backup was
taken: `kaspaapi.rs.bak-v2014` read 8/6 for a backup made on 8/19. Any
date-based cleanup rule deletes the wrong set. The SUFFIX is the only reliable
discriminator, so name the files explicitly.
(b) **`-WhatIf` is preview-only and deletes nothing.** The correct two-step is
WhatIf -> read the target list -> rerun without the flag; it is the same shape
as the anchored-insert rail's assert-count guard, and it is supported on
`Remove-Item` / `Move-Item` / `Set-Content` / `Unregister-ScheduledTask`.
(`-Confirm` is the per-item prompting sibling.)
Standing rule for this tree: **never `git clean`** it — `rc-v2-smoke.yaml` is
the PRODUCTION bridge config and is still untracked, alongside `solo-dual-mode/`
and the c14/c15 patches. Explicit paths only.
Retention policy adopted: keep the current release's `.bak-*` set until that
build has a full day in production, then delete by name.
**Lesson:** the backup rail generates its own cleanup hazard, and the metadata
you would naturally sort by is actively misleading.

## 2026-08-22 → 08-27 — scrape-stall investigation, wedge incident, host & boundary audit

**BL-032 · 2026-08-26 · Kron/zkas-node v1.0.5 — 3.8h zKAS-leg outage: RPC wedge
during a sustained machine-degradation plateau (trigger unidentified;
memory-pressure suspected, uninstrumented)**
Symptom: TG "template age >30s" cards from ~12:03; zKAS leg PLAIN
12:03:04→15:50 operator restart; KAS leg mined throughout. One zKAS block found
12:29:16 against a ~26-min-stale template, submit failed (likely rejectable
regardless — stale parent commitments). ~7–8 expected blocks (~380 ZKAS)
foregone.
Measured chain: discrete onset 12:02:04 → box enters a step-function degraded
state (KAS RPC mean 2.5→73.3ms, 3.3%→91.2% of samples >10ms, flat plateau until
restart — no prodrome, six-day baseline flat) → zKAS RPC dies, returns
12:02:14–12:03:04, wedges permanently: process/p2p alive, reconnects accepted,
requests never served (27 timeouts among 425 instant refusals prove continued
bridge probing), no crash in Application log → cleared only by node restart,
which also freed RSS and ended the plateau.
Root cause: UNPROVEN. Candidates ranked: (1) memory cliff — ~24h after last
manual relief restart, operator-observed >80% RAM, plateau + recovery-on-
RSS-free fit; Event 2004 empty but 2004 gates on commit exhaustion, not RAM
thrash, so absence is not exoneration. (2) eliminated: LAN gRPC covenant-client
load (ports opened 08-24, last session ended 03:00 08-24, latency census flat
across the gap). (3) eliminated: degradation prologue and
bridge-reconnect-failure (both killed by first-hit ordering and the 10s
recoveries through operator restarts — BL-027 pattern, wrong twice before
right).
Blip taxonomy banked: `zk=stale` singles with clean connections = the zKAS
node's ~16-min housekeeping cycle stalling 100–250ms (chronic, benign, flat
for six days; zKAS baseline janky at ~2× the KAS leg). `PLAIN` blips with
`Not connected` = operator sub-60s relief restarts (incl. 08-24 03:56 —
session ran to 03:00). Bridge auto-reconnect proven good on both legs across
all of them.
Fixes: windows_exporter + memory/process-RSS alerts (converts the manual >80%
observation into a card AND the next occurrence into a diagnosis; closes the
standing memory-slope item with data); node v1.0.6 with file logging ON (this
wedge has no node-side witness — meta-principle 4's scalp); `--ram-scale` on
kaspad; gRPC firewall rules scoped to the MacBook IP (BL-023 corollary: the
08-24 LAN opening left unauthenticated RPC reachable by seven unaudited-
firmware rigs); upstream report to firecash (wedge behavior is theirs
regardless of trigger: RPC service hangs permanently after a stall, no fault
raised, survives at TCP-accept level); v2.0.1.5: page-tier `ZkasLegDegraded`
on sustained template age (the age clock ticked to 13,328s while only an info
card fired — detection at T+30s, human at T+3.8h; the gap was escalation, not
detection), retire the 2,872/day structurally-dead balance WARN (shielded
treasury has no UTXOs; a call that cannot succeed firing every 30s trains
WARN-blindness).
**Lessons:** simultaneous drops across independent connections indict the
hosts, not the links — a fleet of clients is a free topology probe. The status
line's `rpc k=/z=` fields were a 10s-cadence stall seismograph printing since
deploy, unread (BL-028 recurring: check whether an existing log line already
contains the quantity). An incident timeline must include environment changes,
not just code changes — the 08-24 port opening sat outside every hypothesis
until volunteered, because it lived in no log. And Event 2004's absence bounds
nothing below commit exhaustion: know what an instrument's silence actually
excludes before citing it.

**BL-033 · 2026-08-22..26 · Prometheus/bridge — the scrape-stall investigation:
blocked-vs-busy; BL-029's causal claim refuted**
A0 closed early and worse than hypothesized: 14s scrape ceiling hit at ≤37h
uptime, then the 25s ceiling within hours of raising it; 18 scrape failures
over 2 days (query_range table). One direct probe collapsed the hypothesis
space: `Measure-Command` on :3034 = 230–257ms serving ~530 samples — a page
that renders in a quarter-second cannot honestly take 25s. BL-029's causal
claim REFUTED (pointer added at BL-029): series growth is real (~6/hr smooth
schedule-minted ramp, 250→530 over 44h — mechanism itself still open) but
cannot drive the render cost; series retirement demoted to hygiene. The stall
class: episodic — floor ~230ms with rare spikes that PIN at whatever the
timeout is (12→14→25s observed) = blocked, not busy; true stall length never
measured (outlived every ceiling; the 55s timeout is the standing best chance).
Sharpened A-headline: why does host pressure turn a 230ms render into a ≥25s
freeze rather than a slow render — suspect classes: blocking write/flush on
the render path, sync RPC reachable from the handler, runtime-pool starvation
(blue-confirm loop 30×2s). Correlation: SOME stalls coupled to
ks_merged_zkas_rpc_ms elevation, some not (survivorship caveat: the worst rpc
sample dies with the failed scrape).
**Lesson:** a scrape duration pinned at the timeout means the handler didn't
answer, not that it worked that long; only a direct endpoint probe separates
blocked from busy. One probe ended three days of Prometheus-side inference.

**BL-034 · 2026-08-22 · instruments — three rendering-layer traps in one
investigation**
(a) Graph decimation: medium-res graphs hid 15 of 18 scrape failures; the
query_range API table is the data, the graph is a picture. (b) The Prometheus
Table tab shows one instant, not history; "peaks as a table" is a query_range
+ client-side filter job. (c) PowerShell 5.1 → curl.exe strips embedded double
quotes even from single-quoted strings in the backtick-continuation form;
write PromQL matcher-free and filter in PowerShell, or use the UI (extends the
existing quoting note, which covered only the interactive one-liner form).
**Lesson:** every instrument has a rendering layer between the operator and
the data; know what each layer discards before citing its output.

**BL-035 · 2026-08-22→26 · host — morning stalls SOLVED: Store retry-grind;
one evidence leg reattributed, verdict stands**
Root cause: daily Windows Store retry-grind on Microsoft.ScreenSketch failing
0x80073D02 (app in use — the operator's own screenshot workflow kept the
package busy and fed the failure loop). Process killed 08-22; retro-check
08-26: newest Id-20 = 08-22 and ZERO morning-era up-dips in 4 days —
prediction made, prediction confirmed. CORRECTION to the 08-22 record: the
morning Winlogon 6003 events (9:05:15, 10:16:03) match RDP reconnects TO THE
SECOND — they were the operator connecting, not TrustedInstaller servicing;
the conviction stands on its independent legs (WU download/Id-20 events,
SmartRetry, the confirmed quiet-mornings prediction).
**Lessons:** host servicing can masquerade as an application bug, and the
investigation instrument can feed the failure it is investigating.
"Auto-updates disabled" governed none of the channels that fired —
multi-channel verification or nothing. Evidence-leg reattribution without
conclusion collapse is a legitimate move when independent legs hold; and
session-scoped logs (TerminalServices LSM) belong in every host-event sweep
alongside System/Application.

**BL-036 · 2026-08-26→27 · host/bridge — night dips: composite reading
weakened; RDP churn and Defender both refuted for the open pair**
Six dips since 08-22, all 21:30–03:33 ET. RDP session-log verdict was
three-way: one 9-second coupling (8/21 01:30), one circumstantial (08-26
21:30), two uncoupled (08-26 01:31/03:33). H1 summon experiment (08-27
00:08–00:14): 180-probe 2s loop with three event-log-verified
disconnect/reconnect cycles overlaid — floor 214–229ms unbroken, max 323ms
landing 0.5s after a reconnect: a second-exact coincidence at noise amplitude,
exactly the shape that made the 8/21 coupling look convicting. RDP churn
REFUTED as a sufficient trigger. Defender diff: a nightly 01:00:1x signature-
update metronome sits 31 min from the 01:31 dip and hours from 03:33 —
exonerated (a nightly cause cannot produce episodic effects without a
coincident condition, and it is not even temporally coupled). The 01:31/03:33
dips remain UNATTRIBUTED; the discriminating variable is likely
bridge/node-side (rpc_ms coupling per BL-033), not host-side.
**Lesson:** a negative harness result is a result — ten minutes of controlled
stimulus refuted the reproducible-trigger hypothesis before any source was
read. Second-exact coincidences at noise amplitude are how false convictions
form; amplitude is part of the evidence, not just timing.

**BL-037 · 2026-08-22 · laptop rail — auth and cwd traps**
gh browser-auth (device flow) preferred over fine-grained PATs for interactive
machines: two PAT scoping failures vs one un-mis-scopable device flow; PAT
scope errors are silent. PATs remain correct for sandboxes. The `~` cwd trap,
zsh edition: mv-to-dot from the wrong directory strands files silently; the
prompt's directory segment is the pre-command gate (BL-021 corollary).
**Lesson:** on interactive machines prefer auth flows that cannot be
mis-scoped, and verify cwd before every filesystem or git command.

**BL-038 · 2026-08-26 · laptop rail — the two-clone incident (BL-020 live, at
repo scale)**
A failed `cd` at session open was read as "checkout never stood up" while a
clone existed at a DIFFERENT path; a second clone was created and the two
briefly diverged (new @ e7426e1; old self-reporting "up to date with origin"
at 5b37875 from stale refs). The old clone held the only copies of two
uncommitted docs — exactly where the hazard analysis predicted. Resolved same
session: verified clean, deleted. ONE canonical clone: `~/zkas/zkas-rusty-kd`.
**Lessons:** `find` for existing clones before cloning; a clone's path is part
of its identity and gets recorded like a sha; "up to date with origin" is a
statement about last-fetched refs, never about origin.

**BL-039 · 2026-08-27 · host — the interactive session is the production kill
domain; single-process bridge topology confirmed**
Session inventory (S11): SIX production processes run in RDP Session 1 —
stratum-bridge, kaspad, zkas-node, zkas-walletd, prometheus, alertmanager;
only ZkasReporter runs as a scheduled task. A sign-out (operator- or
servicing-forced) kills the entire stack; the kickoff's prefer-sign-out
recommendation was gated on this inventory, tested, and REJECTED. The
attached-idle session stays a documented ambient variable; service/task
migration (ZkasReporter pattern) filed to H2. Port-ownership check also
settled topology: ONE process (name `stratum-bridge`) owns :5755, :5765 and
:3034 — "instance 1/2" are listeners within a single OS process: one kill
target in the deploy sequence, one crash domain across both fleets
(BRIDGE-SPEC §2 clarification due). Name-based process greps must include
`stratum-bridge` explicitly.
**Lesson:** inventory session ownership before adopting any logoff or restart
practice — a shipped recommendation that would have killed production survived
until measured.

**BL-040 · 2026-07-09→08-27 · host — uncommanded power-loss series: six
events, an instrument conflict, and a cluster shape (OPEN: rig cross-check)**
Six Kernel-Power 41 + 6008 pairs: 7/9, 7/21, 8/7, 8/15, 8/16, 8/17. All five
decodable events: BugcheckCode=0, PowerButtonTimestamp=0 — no bluescreens, no
held button; operator hands excluded by decode and recollection (clean
operator shutdowns log 1074+6006 and appear on different dates). Bridge logs
corroborate all four August events as mid-flight kills (no Ctrl+C, no
"completed" lines — contrast the 08-26 15:50 deliberate tail). INSTRUMENT
CONFLICT: 6008 message bodies claim shutdowns 8–22 min before boot; bridge log
mtimes prove the system alive ≤~40s before each boot. Resolution: 6008's
"shutdown time" is a stale heartbeat (lags to the last recorded timestamp) —
the written-to log is the better clock; dark gaps compress from minutes to
seconds, weakening sustained-outage relative to brief-transient. Cluster shape
(intervals 12d, 17d, 8d, 1d, 1d, then 10+ quiet days with NOTHING changed)
argues against monotonic brick degradation; episodic premises transients or an
intermittent connector fit. DECISIVE TEST OPEN and perishable: any rig uptime
spanning 8/17 = Kron-local fault (19V brick/barrel; UPS insufficient alone);
rig boots clustered at event times = circuit-side (UPS sufficient). H2 UPS
install is correct under every surviving theory.
**Lessons:** Windows' unexpected-shutdown timestamp is a lower-bound
heartbeat, not a death time — a file being written is the better clock. A
fault series that self-quiesces with nothing changed is evidence about the
fault class, not reassurance.

**BL-041 · 2026-08-27 · boundary — WAN/LAN audit: expected-state corrected,
posture verified**
AT&T gateway (192.168.1.254): NAT/Gaming carries exactly TWO deliberate
forwards, both to Kron (hostname-verified WIN-BEEMRR5U33V) — 16111 tcp/udp
(kaspad P2P) and 16811 tcp/udp (zkas-node P2P); no 3389 or other forwards; IP
Passthrough OFF; IPv6 firewall ON, no exceptions. NLA verified on
(UserAuthentication=1). "Zero rules targeting Kron" was the EXPECTED state in
two shipped closure statements and was WRONG — the P2P exposure is now
recorded as policy (rationale: inbound peers for solo block propagation; risk
class: zkas-node v1.0.5's young parser WAN-reachable on a custody box; UDP is
surplus — narrow to TCP if the UI allows). Managed bridge = TP-Link TL-SG116E
v2.20 (fw 20230505) at .191: pure-L2 Easy Smart switch — no L3 services or
cloud agent by construction; VLAN off; non-default credentials; HTTP mgmt,
LAN-only. Fleet map recorded: ports 1/2/5/6 = KS0 Ultras (.21/.22/.25/.26),
ports 7/8/9 = KS7 Lites (.27/.28/.29). Standing inventory line: both nodes'
gRPC opened to the LAN 08-24 for covenant testing (scope-down to MacBook IP
filed in BL-032's fix list). OPEN riders: port-11 unidentified 100M device;
ports 15/16 (1000MF) mapping; SG116E firmware currency check → KRON-HARDENING.
**Lesson:** audit against the artifact, not the expected state — the boundary
held, but for partially different reasons than the record assumed.

**BL-042 · 2026-08-22 · host — clock stepping (OPEN until H4)**
Kernel-General 1/24 pairs every ~30 min = w32time stepping the clock;
correlation precision of every host-event join in this era rides on it. H4:
w32tm status read; fix if the steps are seconds-scale.
**Lesson:** before trusting second-exact joins across logs, verify the clock
is not being stepped between them.

**BL-043 · 2026-08-26 · documentation — a spec cut from secondary records
inherits their drift (BL-004 corrected in-place this commit)**
BRIDGE-SPEC r1 reproduced BL-004's retracted "structurally immune" claim by
drafting from ledger + memory instead of the in-repo artifact; the pre-move
`ls` gate surfaced the old spec and caught it before commit. Draft 4 §6 had
walked the claim back (immunity = lockfile-drift class ONLY; the cdylib/risc0
LNK class lives in-workspace and took three fixes); the ledger inherited the
broad claim forward. BL-004's lesson line carries the dated correction as of
this commit.
**Lesson:** meta-principle 1 has a documentation tier — verify claims against
the in-repo artifact, not downstream records of it; secondary sources
faithfully replicate the absence of each other's corrections.

**BL-044 · 2026-08-27 · host — seventh power event closes BL-040's test:
mixed etiology; UPS installed; 6008 heartbeat lag calibrated**
Premises-wide outage 02:05–02:50 (operator-witnessed, 45 min dark) — a
signature distinct from all six prior events (≤~40s). UPS ×3 (CyberPower)
installed the same morning; every future uncommanded Kron drop while rigs
ride through is now near-proof of the DC-side fault class — the UPS is an
instrument as well as protection.
BL-040's rig cross-check CLOSED on pre-outage readings: W7/W8 at 13d (boot
≈8/14) rode through 8/15, 8/16 AND 8/17 → the 8/15–16 cluster is CONVICTED
Kron-local; the 19V brick / DC barrel moves from suspect to convicted class;
spare brick on the H2 procurement list (the fault a UPS cannot cover). KS0s
at 10d (boot ≈8/17) leave 8/17 ambiguous — possibly leg-scoped (KS0 boot
timestamp is the tell, low priority). Etiology formally MIXED: Kron-local
class + premises class coexist in one series.
Heartbeat CALIBRATED against ground truth: 6008 reported "shutdown at
1:29:52" vs known ~02:05 death = ~35 min lag on a quiet System log; the 8/16
boot wrote NO 6008 at all (marker not reliably written). Boot side exact:
Event 41 at 02:50:05 vs power restored ~02:50. BL-040's log-mtime-over-6008
resolution upgrades from inference to measurement.
Recovery cost measured: full six-process manual relaunch in dependency order
(nodes 02:53 → monitoring 02:53–54 → walletd 02:55:07 → bridge 02:55:25),
power-on → mining in ~5 min, at 3 AM — BL-039's service-migration case,
quantified. A4's discriminator clock reset by the restart (valid ~02:55
08-28).
W9 watch item: spontaneous rig self-reboot ~8/25, operator-witnessed (not
initiated), no coincident host/premises event, same-circuit W7/W8 unaffected
→ rig-internal (watchdog class). Single occurrence = watch, not
investigation; exact timestamp recoverable from the six-day log
(RKStratum_1787244620.log — one more reason that file is archive-grade);
invisible to current alerting (~2-min stratum gap) → per-worker disconnect
counter folded into SCOPE r2 A6.
**Lessons:** an installed UPS converts every future power event into a
one-bit diagnosis — protection and instrument in one. When a ground-truth
event occurs, calibrate the instruments against it while it is fresh: the
6008 lag went from argued to measured for free.

**BL-045 · 2026-08-27 · bridge/prom — THE STALL ROOT CAUSE: a serial,
timeout-less HTTP accept loop; fixed in v2.0.1.5**
`serve_http_loop` served one connection at a time — no spawn, no read
timeout. Any slow, idle, or half-open client parked the whole server at
`read()` and every queued scrape pinned at whatever Prometheus' timeout was
(12→14→25→55s all observed; the stall outlived four ceilings). True render
floor measured at ~5ms (min_over_time 3.9ms, ~530 series) — every prior
"~230ms floor" was curl.exe process-startup cost: instrument overhead, not
server time. Canonical parker: the operator's own dashboard browser (polls
plus speculative preconnects that send nothing). Confirmed by a five-event /
five-activity-window join on 08-27 alone: 03:40 (post-recovery dashboard
check), 08:29 + 08:39 (pace investigation, 55.0s pins), 08:40 (13.29s
partial — a parked socket releasing mid-scrape as a tab closed), 09:00
(1.85s, interactive pulls). Mechanistically closes BL-036's unattributed
01:31/03:33 pair (an idle-session tab works the door autonomously — no
session events required) and explains H1's negative (churn shook the
doorknob; it never parked the door). Deterministic reproduction, both
directions: a silent TCP parker pins v2.0.1.4 at a measured 8,048ms;
v2.0.1.5 (spawn-per-connection + 5s read timeout, ~10 lines in one
function) serves 213–232ms × 8 under the identical parker and then EVICTS
it with a FIN at the timeout — proven from the parker's own socket. Second
ledgered case of the investigation instrument feeding the failure it
investigated (ScreenSketch precedent). Interim mitigation (no :3034
browser tabs) held from discovery to deploy.
**Lessons:** instrument overhead is part of every floor measurement — a
floor measured through curl is curl's floor, not the server's. A serial
accept loop is a shared-fate contract with your least cooperative client.
And a duration pinned at the timeout still means the handler never ran —
BL-033's lesson, now with its mechanism.

**BL-046 · 2026-08-27 · host/reporter — reporter double-death across the
outage: battery-stop convicted; the page was delivered and slept through**
Death #1, ~23:52–00:01: wordless — last write a BEAT1 mid-cadence, no
error, no Scheduler restart attempted → condition-stop-or-clean-exit class,
OPEN (5.3-day runtime; ExecutionTimeLimit=P3650D excludes limits; pre-UPS
excludes battery). `RcReporterDown` fired 00:06→10:36 (ALERTS series) and
the Telegram card was DELIVERED at ~00:01 — the pipeline is exonerated end
to end; the 10.5h gap was a solo operator asleep. Escalation-channel design
(page-tier loudness vs accepted overnight latency) → H2, as an explicit
choice. Death #2, 02:52:24: the logon trigger DID fire at 02:51 (the 02:55
recovery was seven-of-seven attempted, correcting the earlier six-of-seven
read); the reporter replayed the pre-outage log, every POST failed — SSL
trust to Supabase, attributed to WAN-recovery transients while the gateway
itself rebooted, NOT clock skew: the boot-era time step measured +318ms and
steady-state w32time discipline is ±1–23ms per 30min → **BL-042/H4 CLOSED
healthy**; this era's second-exact log joins are validated. Then the UPS's
USB battery registered and `StopIfGoingOnBatteries=True` stopped the task —
RestartCount=3 never fires on condition stops. The protection stopped the
protection. Both battery flags flipped False same day (verified); the H2
service-migration template inherits the two flags + an at-startup trigger
(+delay) + top-level exit logging so no death is ever wordless again. Data
healed same day: the reporter's startup replay posted 6 (its from-byte-0
newest-log replay is real self-healing — corrects the earlier "starts at
EOF" claim), and zkas-catchup-r1.ps1 posted the 4 blocks from the
unreplayed 02:55-era log and corrected the 3 provisional rows
(Beat2GiveUpSec=3600 had expired them) — 13 rows exact, join dt 0.2–4.3s,
zero unmatched, upsert-on-hash making every re-POST safe.
**Lessons:** on a UPS-protected host every scheduled task's battery-stop
defaults are armed against you. A delivered page is only half an escalation
design. And the two-beat/upsert idempotency design paid for itself — any
log the reporter can read is a gap it can close.

**BL-047 · 2026-08-27 · CI/release — a stale-cache build shipped v2.0.1.4
bytes under a v2.0.1.5 tag**
canary-eda7090: the tag's tree verified correct by raw read
(BRIDGE_BUILD=5, new accept loop present) — yet the built exe announced
v2.0.1.4. Mechanism: actions/cache restored a fully-built `target/` (cache
key = Cargo.lock hash only, untouched by the patch) and cargo's mtime-based
fingerprints judged the patched crates up-to-date against the fresh
checkout; packaging zipped the cached exe. BL-030's banner guard validates
tag-vs-SOURCE (sed on main.rs) and is structurally blind to a stale
ARTIFACT. Silver lining: the stale canary furnished BL-045's measured
positive control (8,048ms) before being retired. Fix: `target/` removed
from BOTH cache blocks in deploy.yaml (deps stay cached; the workspace
always rebuilds; ~10–15 min/build bought with correctness), caches purged
(`gh cache delete --all`), release recreated cold → canary-1b63698, banner
v2.0.1.5 verified as a HARD GATE before the soak clock started. Also
banked: GitHub auto-attaches "Source code" zips named `repo-tag` to every
release — the build asset is `zkas-<tag>-win64.zip`; one wrong download
burned learning it.
**Lessons:** the running artifact's self-report (banner) is the only
version check that sees through the entire build+deploy chain — the CI-era
corollary of four-way identity, now a gate that precedes any soak. A cache
key must cover everything that determines the artifact, or the cached layer
must be rebuilt by construction.

**BL-048 · 2026-08-27 · host — UPS load audit and Defender exclusion
inventory**
Post-install audit caught a latent fault: unit 1 at 1.01kW against a
CP1500's ~1000W inverter — fine on passthrough, trips at the next outage:
the UPS itself as the would-be eighth power event. Physics forbids
rebalancing it away: 3 × KS7 ≈ 505W each, so any two on one battery bank
overload by construction. Resolution: hierarchy over symmetry — rigs are
ride-through-OPTIONAL (all seven hard-dropped and self-recovered in every
historical event), custody/network is ride-through-REQUIRED. End split:
2 × (KS7 + 2 KS0) ≈ 760W/76% on the 1500VA units; the 1000VA carries Kron +
switch + aux only (~63W → 30–60 min runtime; yesterday's 45-minute outage
class becomes a non-event for the custody stack); the third KS7 rides
surge-only. An operator-proposed three-way rig balance (KS7 on the 1000VA
with Kron, ~570W) was rejected on inverter math (95% of a 600W-class
inverter) and runtime inversion (Kron's runtime spent in the rig's first
three minutes). Defender ExclusionPath inventory recorded: C:\Node-v2,
C:\RKBridge, C:\rusty-kaspa-v2, C:\Users\inmyh\AppData\Local (wholesale —
exempts browser caches and temp staging on a custody box; narrowing to the
specific app dirs → H2), C:\Users\inmyh\rusty-kaspa, C:\zkas (added 08-27).
**Lessons:** a UPS's battery-path wattage is the constraint, not its VA
badge; order protection by recovery cost, not wattage symmetry; on-line
load% prices the outage, not the day.

**BL-049 · 2026-08-22→27 · process — command-shipping trap catalog (seven
exhibits; law minted to conduct tier)**
(1) `</br>` markup leaked into a fenced command (Kernel-Power decode);
(2) Select-String's case-insensitive default matched "not found" while
hunting FOUND; (3) a production grep shipped on a pattern the record
already held as a dead fossil (`KASPA BLOCK`); (4) `powershell -File` does
not parse comma-lists into array parameters (-OnlyHashes arrived as one
string; `-Command` invocation is the fix); (5) zsh commands executed on
Kron's PowerShell (the `&&` parse error saved the run — wrong machine,
wrong shell, one keystroke from harmless); (6) the first parker test's
unguarded curl wedged the console instead of measuring the block it was
built to measure; (7) `</parameter>` fragments recurring in fenced blocks
under session fatigue (three further instances, all caught pre-execution).
Total cost: ~ten minutes and zero damage — every exhibit was caught by an
expected-output mismatch, which is the point. The consolidated law lives at
conduct tier (project instructions), not here: fenced commands name their
target machine and shell; last-token integrity eyeball before ship;
patterns verified against live excerpts with explicit case handling; array
parameters via -Command; every probe that can block carries its own
ceiling.
**Lesson:** expected-output statements are the safety net that converted
all seven errors into cheap catches — the discipline that finds the error
is the same one that bounds its cost.

---

## Notable non-bugs (investigations that closed as understanding, kept because
they cost real time and will recur)
- Mergeset deferred payouts (BL-003) — misread twice before sticking.
- The p9f9d2d "mystery wallet" — operator's own pre-existing web wallet.
- cashlandhawks "works for me" — release-binary vs source-build provenance
  (made the FCMM report stronger once reconciled, not weaker).
- Kaspa-block "rejected but paid" confusion — two hashes, two chains, one solve;
  the [MERGED] log's generic "Kaspa" naming means the TEMPLATE-SOURCE node
  (zKAS on 16810) in kaspaapi contexts.
- Zero-block droughts: 5–6h at ~2 TH/s is P≈0.2 — verify with telemetry
  (target-decode, near-miss frequency) before suspecting breakage.

## Meta-principles (the ones that actually cracked cases)
1. Verify every claim against logs/source/chain: "code reads right" < "verified on
   my hardware" < "verified with money".
2. Independent-path cross-checks (integer vs float, explorer vs node, KDSM vs
   bridge) localize bugs in one observation.
3. Evidence chain before/after any fix: byte-level proof (findstr/decode) +
   live behavioral proof.
4. File logging on for anything under investigation; console scrollback is not
   evidence.
5. One variable at a time; keep forensic artifacts (old exes, wedged datadirs)
   until the issue thread closes.
6. Official binaries for consensus; own pipeline only where it adds value.
7. Scope claims precisely; a reconciled counter-report strengthens a bug report.
8. Production is sacred: +100/canary port offsets, failover-first, control-rig
   holdouts, never bet the fleet on unproven code.

**BL-050 · 2026-08-28 · v2.0.1.5 fleet deploy — CLOSED (pending A1' 7-day
window)**
Deploy 01:09:02 EDT · policy: soaked bytes promoted. Old exe 66C27E9E...CC882
parked `.bak-v2014`; production = canary exe F1484FB5...A3F0 @ commit 1b63698;
PID 15160→14940. Gates: 4a banner v2.0.1.5 ✓ · 4b 2× MERGED ✓ · 4c three
listeners one PID ✓ · 4d reporter rotation +0.5s ✓ · 4e parker: 8× flat
214–266ms + server FIN — BL-045 proven on production; tab-ban lifted. Fleet
7/7 (incl. w1m) at +4min, 284/0/0, zk=ok. Block 7e106acab547 (w7m) rode the
kill/relaunch: BEAT1 pre, BEAT2 post, txid d4b18c88bfeb.
GO evidence: FORENSIC IMPLAUSIBLE=0; canary tail zk=PLAIN 00:34–00:54
canary-local (production zk=ok verified 01:03 pre-kill); soak/pace/share
health on operator attestation. Deviations: 3a lock-branch taken, resumed at
failed link (13c); 3c shipped with cmd `&&` into PS 5.1 — relaunched via
`Start-Process cmd`; law: lifecycle commands name their shell AND ship in it.
Record build: release v2.0.1.5-win @ merged-v2.0.1.5, guard OK 05:04:58Z,
win64 zip f90b3769...cfe1cbe (distribution only, never deployed).
Open: A8-channel WARN pair 01:13:03 (UTXO fetch, post-restart — watch for
breaker INFO or self-clear); canary process kill/confirm (step 5b) if not
yet done.

**BL-051 · 2026-08-22→29 · prom/alertmanager — THE BLOCK-CARD LOSS: a
documented two-file coupling that was always three, broken by a scrape
change nobody connected to it**
Symptom, operator-observed: block Telegram cards fell to ~1 in 10 after
08-22 while hourly cards stayed perfect. Not a rule bug and not a route bug —
Prometheus collected everything. The span in which Alertmanager can notify is
`firing span = range window - scrape_interval - for`. At 15s scrapes that was
180-15-30 = 135s against the info route's 90s group_wait: 45s of margin. The
08-22 change to 60s scrapes (H5-era, made for the BL-045 stall) cut it to
180-60-30 = EXACTLY 90s — a dead tie with group_wait, decided by sub-second
ordering inside Alertmanager, with `send_resolved: false` making every loss
silent. MEASURED 08-29 11:00-12:00 via ALERTS query_range at 15s step: four
blocks (11:15 w9m, 11:24 w9m, 11:52 w8m, 11:54 w9m), all doubles, four firing
spans of EXACTLY 90s, ONE card delivered. Hourly cards corroborate to the
block: tot K 41→45, Z 45→49, D 40→44. The inhibit rule was working correctly
throughout — one card per double is the design, so the loss was 1 card per
event, not 3. Both files carried the coupling in their own comments and both
named only each other; alert_rules.yml even shipped the arithmetic ("scrape
<=15s + eval <=15s + for 30s") and the warning "IF YOU SHORTEN THESE WINDOWS,
SHORTEN group_interval TO MATCH". Nobody shortened a window. FIX:
`keep_firing_for: 2m` on the three block rules — 210s span, 120s margin,
INDEPENDENT of scrape_interval (the only lever that is), and under the info
route's 300s repeat_interval so no duplicate cards. Lowering group_wait was
unavailable: the inhibit rule needs the 90s hold for the double to catch the
singles (blue-confirm skew, BL-026 era). Gates: promtool 41 rules · reload 200
· /api/v1/rules keepFiringFor=120 on all three (BL-020 readback). Deployed
sha 26DF1660...B45656; repo mirrors synced at commit 96eff28 — and they were
found stale since 08-22 01:06, eleven hours before the 08-22b revision, with
alertmanager.yml at 4,591 bytes against 11,511 deployed (the entire 08-18
inhibition analysis absent from the rail).
**Lessons:** a documented coupling is only as strong as its enumeration of
TERMS — scrape_interval was in the equation and on nobody's list. When a
config comment states arithmetic, the arithmetic's inputs are part of the
contract and every one of them needs naming. And prefer the lever that is
independent of the variable that broke you.

**BL-052 · 2026-08-28→29 · process — PROBE TRAP CATALOG: seven instruments
that reported success while failing (the BL-049 sequel, measurement tier)**
BL-049 catalogued commands that failed to run. This is the worse class:
probes that RAN and LIED. (1) `curl -s -o NUL` — `-s` suppressed the error,
`-o NUL` discarded the empty body, so a connection failure rendered as
silence and was read as success; exit 7 the whole time. This single probe
anchored SIX successive wrong mechanisms about a walletd outage (pool
exhaustion, connection-slot starvation, address family, proxy interception,
firewall/filter driver, accept-queue backlog) before `-w "%{local_ip}"`
printed `:-1` and collapsed all of them. (2) `%ERRORLEVEL%` on a cmd line
joined with `&` expands at PARSE time — printed `exit=0` for a curl that
returned 28. (3) `max_over_time(...)` unlabeled returns ONE SERIES PER
TARGET; `$r.data.result[0]` printed prometheus's 2.88s and hid two 55s pins.
(4) PowerShell here-strings drop the newline before the closing `'@`, welding
an insert onto its anchor (`## Revision`) — caught by the post-sha gate, but
only AFTER the write had happened. (5) `Get-Content | Measure-Object -Line`
skips empty strings: 821 vs the file's real 862 lines. (6) PID
misattribution — 16096 was read as the reporter's powershell and was CHROME;
that error poisoned three hypotheses and was never checked with a one-line
`Get-Process`. (7) `Win32_Battery` returned empty on a box where
`Get-PnpDevice` shows `HID UPS Battery` Status OK — absence of a WMI class is
not absence of hardware.
**Lessons:** an instrument that cannot report its own failure will report
success instead. Every probe carries a failure channel — exit code, http_code,
or an enumeration that shows all rows rather than the first. Verify the
IDENTITY of anything you build a hypothesis on (a PID, a class, a series
count) before the hypothesis, not after it fails. And compute a post-edit sha
in memory and gate on it BEFORE writing to disk, not after.

**BL-053 · 2026-08-28→29 · walletd/host — the browser parker class claims a
second service; and the wedge that outlived it**
Six Chrome→:8501 connections (treasury page, `file:///` origin) opened
18:59-19:18 and were still ESTABLISHED seven hours later; walletd never reaps
them. Same operator behaviour as BL-045's canonical parker, different daemon,
and walletd's accept path has no timeout either — it is not ours to patch, so
it rides the firecash report. Killing Chrome released all six and did NOT fix
the fault: walletd itself was wedged (accepting, never answering; then
refusing at accept), from 17:05 08-28 until a restart at ~02:35 08-29, ~9.5h.
Process alive throughout, port LISTENING, CPU flat at 0s/60s, 23.4 MB working
set (fully trimmed — pages evicted because nothing touched them). Cost: exact
amounts deferred on every block in the window; two blocks aged past
`Beat2GiveUpSec=3600` to permanently provisional pending catchup. Root cause
OPEN — a 270s startup cost (BL-055) does not explain a 9.5h failure that
began 8 minutes after a successful start. Remediation that DID land: both
operator-facing surfaces confirmed reachable from the MacBook at
192.168.1.96 (:9090 Prometheus bound `::`, :3034 bridge bound `0.0.0.0`), so
the dashboard view leaves the custody box entirely. The treasury page cannot
follow — walletd is loopback-bound and the page is a local `file:///` — and
stays an RDP-only, close-when-done surface. STANDING RULE: no browser left
open on Kron.
**Lesson:** when a fault class is identified, enumerate every service that
shares the exposure rather than fixing the one instance that bit — the parker
class had a second victim for weeks, in the daemon holding custody.

**BL-054 · 2026-08-28 · host — EVENT EIGHT: the first post-UPS power event,
and the discriminator that could not be read**
Kernel-Power 41 at 09:44:18 with 6008 claiming shutdown 09:30:20.
`LastBootUpTime` 09:44:16 confirms wevtutil renders LOCAL time on this box —
BL-044's calibration stands and needs no correction pass. Better clock per
BL-040: bridge log `RKStratum_1787894876.log` last write 09:43, successor
started 10:17:31 → dark ≤76s, and the 6008 lag measures ~13 min here against
~35 min on 08-27 (lag is a function of System-log quiet, not a constant). The
sub-minute signature matches the six Kron-local events, not the 45-minute
premises outage. BUT THE VERDICT IS UNAVAILABLE: BL-044 declared every future
event a "one-bit diagnosis" via the UPS, and the first one arrived unreadable.
Event ID 105 (power-source transition) is NOT LOGGED on this host —
`Get-WinEvent` returns NoMatchingEventsFound over the whole log. The USB link
is present (`Get-PnpDevice` → `HID UPS Battery`, VID_0764 CyberPower, Status
OK) but no CyberPower/PowerPanel service is installed, so nothing records
transfers. Event eight is therefore UNCONVICTED — Kron-local by signature,
unproven by instrument. ZkasReporter battery flags verified both False
(BL-046 trap disarmed). RECOVERY COST, measured: dark 09:43 → bridge back
10:17:31 = ~34 minutes, gated on operator logon at 10:13 — against BL-044's
~5 min when the operator was already awake at 3 AM. That is the realistic
H2 service-migration figure.
**Lesson:** an instrument that is designed but never exercised is not an
instrument. BL-044 banked "near-proof" on a capability that was never tested
end-to-end; the test came from the fault, not from us, and failed. Validate a
diagnostic path against a synthetic event before relying on it.

**BL-055 · 2026-08-29 · walletd — first artifact pin, first log, first
launcher; and a 269.7s startup cost nobody had ever seen**
walletd ran for the operation's entire life with NO launcher script and NO
sha pin — hand-typed command line, varying per session, on a binary with no
version banner. Closed today. Identity pinned:
BDCBE0673C800720EF33D73EB68A4C6FBEBB10B3CA472E0822B8FDE08063713C, mtime
2026-08-02 22:32:34. Version confirmed v1.0.5 behaviourally — `/api/wallet/
balance` returns `notes` WITHOUT `?notes=1` and carries no `note_count`
(NODE-CONTRACT §6 delta list, used as a version oracle). The v1.0.6 node
cutover did NOT touch it (node went to `C:\zkas\node-v106\`, walletd lives in
`C:\zkas\node\`). THREE DEFECTS FIXED: (a) `--wallet-secret` rode argv,
readable by any local process via `Win32_Process`/`Get-CimInstance` — now a
DPAPI CurrentUser blob at `C:\zkas\walletd-secret.dpapi` handed over as
`ZKAS_WALLET_SECRET`; residual is a same-user PEB read, which needs a binary
change (upstream candidate). DPAPI chosen over a plaintext file because the
wallet seeds are encrypted FROM this secret and a plaintext copy one directory
above them turns disk theft into custody compromise; blob is machine- and
profile-bound, so the password manager is mandatory. (b) `--proof-threads`
unset → all 16 logical cores available to Halo2 on a six-service box; now 6.
(c) walletd HAS NO LOGGING FLAG — stdout IS the log — so every prior wedge was
witnessless; launcher redirects to `C:\zkas\logs\walletd-<stamp>.out.log`.
FIRST MEASUREMENT off that log: `subtree cache built in 269.7s (1529415
leaves, notes=471)`, single-threaded at exactly 1.00 core, during which the
daemon accepts connections and answers NOTHING. Reporter WARNs stopped 6s
after that line. This is a per-restart cost that scales with leaf count on a
1 BPS chain — it gets worse every week, and it ELEVATES the walletd v1.0.6
cutover, whose §6 claim is exactly this path (656s→~76s). Correctness proven:
notes=472, synced=True, scanned_blocks=2,953,223, balance 5,946,525,827,141
sompi (59,465.25827141 ZKAS), served in 0.1s once warm. Also banked:
auto-consolidate's real cadence is "one merge of up to 38 notes per 60s, only
while nothing is proving" (log line, more precise than --help); execution
policy is AllSigned and EVERY script on this box runs via `-ExecutionPolicy
Bypass` — enforced-but-routed-around, a posture KRON-HARDENING should either
commit to (signing) or drop. DEVIATIONS, recorded per law 9:
`set-walletd-secret-r1.ps1` declared VOID — it reported SUCCESS on a
1-character capture (paste into a masked console field silently dropped all
but one keystroke) because its only check was that the blob round-tripped,
which a wrong-but-consistent value passes; r2 adds a GUI credential dialog
(paste works), an 8-char minimum, and a length echo confirmed BEFORE the
write. And the cutover changed THREE variables at once — launch host
(cmd→PowerShell), secret delivery (argv→env), and proof-threads (unset→6) —
violating meta-principle 5; when the new instance appeared to hang, none of
the three could be attributed, and the operator's own question about the
launch shell is what redirected the investigation to the contract's 656s
figure. Launcher also deviates from house convention (`.ps1`, not
`run-walletd.cmd` beside `run-zkas-node.cmd`/`run-rc-merged.cmd`) — r2 due.
**walletd does NOT auto-start on reboot.** The launcher is a prerequisite for
H2 service migration, not a substitute for it; six of seven production
processes remain manual and Session-1 bound (BL-039).
**Lessons:** a round-trip check proves CONSISTENCY, not CORRECTNESS — verify
against the artifact's real purpose (walletd opening three wallets), not
against the encoding surviving a decode. A consumed binary with no version
banner and no launcher has no identity and no reproducible invocation; both
are cheap and neither existed here for months. And meta-principle 5 applies
hardest exactly when several improvements are ready at once.

**BL-056 · 2026-08-27→29 · docs rail — THE RETIRED BRANCH THAT KEPT TAKING
COMMITS: three documents stranded for two days, and three false absence-calls
made from single-rail reads**
At the S11/S12 close (08-27) the laptop clone was resting on
`merged-v2.0.1.4`. Three documentation artifacts committed there and nowhere
else: SCOPE-v2.0.1.5 r3 (`6ecfb06`, 302 ln, `1b70e2ba…632e3`),
FLEET-DEPLOY-v2.0.1.5-r1 (`51f4193`, 115 ln, `5df03ad7…a4e4e`), and
SESSION-STATE-2026-08-27 (`51f4193`, 99 ln, `a7bdd148…4b02d`). On 08-28
`merged-v2.0.1.5` became canonical at the fleet deploy. The ledger portion of
that same close was rescued by the `5da64aa` cherry-pick — because BL-045..049
were visibly missing and someone went looking. The three docs were not, because
nobody looked. For two days the production branch's SCOPE was r2, header
reading `Status: READS PENDING (no code started)`, describing a release that
had been running seven rigs since 08-28. Surfaced only by a cross-rail sha
audit run for an unrelated reason (a session-summary request).
**Root cause: branch retirement has no retirement step.** The lineage law says
each release branch becomes canonical on release and the line is strictly
linear — but nothing freezes the predecessor or verifies that everything on it
reached the successor. A retired branch that still accepts writes is a
BL-019-class trap: `merged-ws1-port` was the code instance of this, and the
docs rail just produced its own.
**Second failure, same sitting, Claude-side: three absence-claims, all false,
all from single-rail reads.** (a) ENGINEERING-LEDGER declared "mount stale at
BL-049" — read off the file's OWN header line 5, which had never been bumped
when BL-050..055 were appended; the file was byte-current at 1167 ln
`29405554…a450` on both rails and contained all six entries. (b) SCOPE r3
declared "cut, uploaded, never committed" — it was at `6ecfb06`. (c)
SESSION-STATE-2026-08-27 declared "does not exist; the new cut is a new
document" — 99 lines on `.4`, and it already answered three of the four
questions the audit was reconstructing. Each claim was an inference from one
rail's working tree to the artifact's existence. The instruments that
falsified them are cheap and were available throughout: `git log --all
--diff-filter=A -- <path>`, a content sha, `git branch -a --contains`.
**The mount was never the wrong rail.** It was a HYBRID with no single git
anchor — SCOPE and FLEET-DEPLOY faithful to `.4`'s docs tip, ledger and the two
v1.0.6 node docs faithful to `.5` — which is why it read as simultaneously
ahead and behind depending on which file was sampled. Law 2f was honored at
every individual commit; what failed is that a mount synced from two branches
has no verifiable identity.
**Fix, `a2d8650` on `merged-v2.0.1.5`:** path-scoped `git checkout
merged-v2.0.1.4 -- <3 paths>` rather than cherry-pick — `6ecfb06` also carries
a ledger append already present on `.5` via `5da64aa`, so a pick would have
conflicted on a file needing no change. Bytes verified by sha at stage, before
commit; all three matched the `.4` pins exactly. Same commit corrected the
ledger self-header (BL-049 → BL-055; 1 ins / 1 del) with the post-edit sha
PREDICTED container-side before the write (`3c73ee05…e656`) and confirmed on
readback. Commit arithmetic reconciled independently: 295 ins / 25 del, with
SCOPE's 80+24 = the 104 changed lines the pre-commit `.4`↔`.5` diff had
forecast. `merged-v2.0.1.4` retired to read-only as of this commit; it now
holds nothing `.5` lacks.
**Law 15's line-count identity check failed here and was amended.** The ledger
read 1167 lines both before AND after the header fix — a single-line
substitution leaves the count invariant, so only the sha discriminated
`29405554` from `3c73ee05`. Destructive mount deletes now cite line count AND
content sha. Law 14 gained an exhibit: a trailing `</parameter>` markup
fragment rode the commit block and zsh parse-errored it — the last-token
integrity eyeball exists for exactly this and was not run, on the very commit
that was reconciling the law into the instructions. Nothing executed; the chain
resumed at the failed link per 13c. Law 16 minted (rails are independent;
absence is rail-scoped), generalizing 13d from refs to rails.
**Deviation recorded per law 9:** mount sync BATCHED at session close rather
than riding each commit (2f), operator-authorized in-conversation — three
artifacts land within the hour and two upload passes double the wrong-file
click exposure that stage-verify-open exists to eliminate. Divergence is named
here rather than silent, which is the interest 2f protects.
**Still open from the 08-27 handoff:** the 02:55 08-28 pace gate
(`rc:solves_24h ≥ 33`) has no recorded result on any rail.
**Lesson:** a rail is not the record. A file's absence from one branch, one
mount, or one working tree is evidence about that rail alone — and a
document's own header is not an instrument for its currency, because headers
are written by hand and content is written by append. Three cross-rail
commands would have prevented every false call in this sitting, and a
retirement step on the outgoing branch would have prevented the incident that
prompted them.

## 2026-08-29→30 — S14: P1 shipped, PowerPanel exercised, rail-enumeration laws

**BL-057 · 2026-08-27→30 · process — a deliverable shipped with a correct
move-and-verify one-liner that was never run, and an absence-claim built on
an incomplete rail list**
`P1-BOLT-BRIEF-r1.md` was cut, sha-pinned `86d2b546…a210`, presented, and
shipped 08-27 with a correct `mv`-and-`shasum` one-liner. It never reached
`~/zkas-lab/`. SESSION-STATE tracked it as "cut and pinned… unpasted" — which
monitors the DOWNSTREAM USE while silently assuming the landing. Cost: two
days of D_z/D_k curve, permanently, at 15-day retention.
Compounding error, Claude's: three rails were checked (mount absent; laptop
absent by name AND by content grep; repo `--diff-filter=A` across all fetched
branches empty) and the artifact was declared **VOID** — omitting the
CONVERSATION rail, the one rail that is append-only and guaranteed to have
held it, since every deliverable is authored there. Recovery took one
`conversation_search` + one `read_conversation`; reconstruction from the
transcript hashed to `86d2b546…a210` **byte-identical**, proving it was
r1 RECOVERED, not a re-cut. No renumber, no VOID. Note the sha pin is what
made recovery verifiable: a pin converts "gone" into a testable claim, and it
was equally available before the VOID was written.
**Lessons:** the rails are mount · repo (per branch) · working tree ·
CONVERSATION, and the conversation rail is the rail of first resort for any
deliverable. A VOID declaration destroys an artifact's identity in the record
and carries law 15's evidentiary bar, not law 16's — never issued from
absence alone when a sha pin exists. And law 1b's one-liner is only half a
protocol: a deliverable that gates other work needs its landing CONFIRMED,
not assumed.

**BL-058 · 2026-08-30 · host/UPS — PowerPanel installed and EXERCISED;
BL-044's assumed channel REFUTED, real channel found, runtime measured**
BL-054 recorded that event eight was unreadable because Event ID 105 is not
logged on this host. Root cause now known and it is not host configuration:
**PowerPanel Personal does not write to the Windows event log at all.** No
`CyberPower`/`PowerPanel` log channel is created (`Get-WinEvent -ListLog`
empty); a deliberate 12-second wall-plug pull produced ZERO System-log
entries. BL-044's "one-bit diagnosis" was banked on a channel that was never
going to carry the signal.
The instrument DOES exist, one layer over: the PowerPanel UI's Event Logs
view, backed by `C:\Program Files (x86)\CyberPower PowerPanel Personal\
assets\PPPE_Db.db` (SQLite; snapshot pinned `0586A64C…F56B3`). The exercise
produced four second-precision rows: `00:16:45 Utility Power Failed,
transferred backup mode` · `00:16:45 Battery is discharging` · `00:16:54
Utility Power restored` · `00:16:57 Battery stopped discharging`. Every
future host-event sweep must include this store alongside System /
Application / TerminalServices-LSM.
Services `PowerPanel Personal Service` + `…Service Monitor`, both Running /
Automatic. Kron rode the transfer with zero process impact.
**Estimated runtime measured at 11 min** with 2 KS0 Ultras still on the
1000VA unit (battery at 97%, still recharging — so 11 is understated).
Corroborates BL-048 independently: if the KS0s are ~2/3 of the load, the
remainder is ~63W, which is exactly BL-048's derived figure for Kron +
switch + aux. Post-rebalance forecast narrows from BL-048's 30–60 min band
to **~33–40 min** (better than 3× by Peukert), i.e. the LOW half — still
short of the one known 45-minute premises outage, so graceful-shutdown
configuration stays on the table. Threshold must NOT be set until after the
rebalance, then re-measured by the same plug pull at 100% charge.
PROCESS NOTE: the step-5 read was first run WITHOUT the plug pull. Empty
output was correctly treated as ambiguous and disambiguated by asking, not
inferred — had it been read as "PowerPanel doesn't log," it would have been
a false conviction on an unexercised probe, i.e. BL-054's own error one
layer up.
**Lesson:** BL-054 said an instrument never exercised is not an instrument.
The corollary: exercising it is also the only way to learn WHERE it records.
Both the channel and the runtime figure came from twelve seconds of
deliberate fault injection with nothing at stake.

**BL-059 · 2026-08-29 · P1 — ingest + sampler SHIPPED end to end, every gate
verified against the deployed artifact rather than the vendor's report**
Bolt reported both artifacts live. Treated as a claim, not evidence. Verified:
constraint 4 (fail-closed) tested BEFORE the secret was set — the one path
observable exactly once — returning 401 `{"error":"Unauthorized"}`, an
APPLICATION-tier body proving our handler ran rather than Supabase's platform
JWT check. Then T1 `inserted` 200 · T2 `duplicate` 200 (the UNIQUE index
genuinely arbitrating — constraint 5 is structural, not application logic) ·
T3 401 · T4 400 naming the missing field. Float round-trip exact through
JSON (`d_z` 1.45e16, `est_hashrate_k` 3.09e17). RLS: SELECT only for
anon/authenticated, no write path outside the service role.
Kron side: `set-nh-secret-r1.ps1` (`2CCEFCFB…F0207`) inheriting all three
BL-055 guards — GUI credential dialog for paste-safety, 8-char minimum,
length ECHOED and confirmed before write; 15 chars captured, cross-checked
against an independent count taken on the MacBook. `network-history-sampler
-r1.ps1` (`E4869402…30DA68`), ONE-SHOT by design — cadence from the task's
5-minute repetition trigger, not an internal loop, because a resident loop is
a thing that can wedge, be battery-stopped (BL-046), or die wordlessly, and
all three have bitten this operation. Task `NetworkHistorySampler` registered
with both battery flags disarmed; verified firing at `00:00:03` and
`00:05:03` into buckets no hand-run produced, `LastTaskResult 0`,
`NumberOfMissedRuns 0`.
File transfer to Kron: browser download, sha-verified both ends. Clipboard
paste into an editor is NOT an acceptable transfer for sha-gated files —
Windows editors save CRLF, LF becomes CRLF, and the identity check dies while
the script still runs.
**Lesson:** a vendor's "both artifacts are live" is a claim about intent; the
acceptance tests are the artifact. And test the fail-closed path FIRST — it
is destroyed by the very configuration that makes everything else work.

**BL-060 · 2026-08-29 · reporter/prom — `Serve-Metrics` is coupled to the
beat loop; and a 26-hour "gap" that never existed**
Six-day capture (08-20→08-27, step 60s, archived — see BL-061) shows
`zkas_reporter` scrape duration max **55.011s @ 08-23 00:34**, 8 samples >5s,
4 >10s of 8,522. NOT the BL-045 parker class: `Serve-Metrics` uses
`BeginGetContext`/`IsCompleted`, so a connection that sends nothing never
completes a context and is skipped — structurally immune to the silent
parker that took the bridge and walletd. The actual mechanism is loop shape:
`Serve-Metrics` is called ONCE per main-loop iteration, LAST, after
`Run-Beats`. Per-call timeouts are correct individually (`Post-Block` 15s,
`Get-WalletHistory` 10s) but one iteration makes MANY — a POST per pending
block plus a history poll. Four pending blocks against a slow webhook is 60s
in one iteration with the endpoint unserved throughout. Metrics availability
is coupled to beat-processing latency, and the coupling is unbounded in
pending-block count. Cheap fix: serve metrics before beats, or between POSTs.
Same capture raised an apparent 26h data gap (8,522 samples vs 10,080 on the
other two targets, with only 10 `up==0`) — the BL-024 shape, where `up` is
synthesized per ATTEMPT and absence of attempts reads as neither up nor down.
One query killed it: `first=2026-08-21 01:59`, ZERO gaps >5min, and
10,080−8,522 = 1,558 exactly matches the 1,559 minutes from window start to
first scrape. The reporter job simply began being scraped that day.
Coverage is unbroken. Claim RETRACTED.
**Lessons:** a per-call timeout bounds a call, not an iteration — audit the
loop, not the call. And a missing-sample count is a hypothesis, not a
finding, until the gap's POSITION is read.

**BL-061 · 2026-08-29 · monitoring — the six-day pre-fix window archived
before expiry; D2 now has a positive control**
Retention read from the running artifact (`/api/v1/status/flags`):
`retention.time 15d`, `retention.size 0B` — the ~09-04 horizon carried across
four documents as an assumption is now MEASURED. Captured 08-20 00:00 →
08-27 00:00 EDT at step 60s (1:1 with the scrape interval; coarser steps hid
15 of 18 failures once already), three series × 3 targets, to
`~/zkas-lab/perishable-2026-08-20_26/`: `scrape_duration_seconds`
(`eb76a51b…c0bf0`), `scrape_samples_scraped` (`9065bc26…15b3df`), `up`
(`7fab0b01…e5be3c`).
Content verified, not merely fetched — the failure mode being HTTP 200 with
`series=0`, i.e. the retention wall wearing a success code. `rc_merged_bridge`
max **55.005s @ 08-26 01:31**, 56 of 10,080 over 5s (0.556%). Decisive
detail: **56 over 5s and 56 over 10s — zero samples in between.** The
distribution is bimodal with nothing in the middle; a render degrading
gracefully produces a tail, this produces a step. BL-033's blocked-vs-busy
thesis confirmed at population scale rather than by one probe. The single
worst sample in six days is 08-26 01:31 — one of the two dips BL-036 left
unattributed and BL-045 later closed as the parker.
**Lesson:** capture a phenomenon's baseline while it still exists; an
acceptance gate without a positive control is an assertion.

**BL-062 · 2026-08-28→30 · monitoring/analysis — the pace gate read, and a
variance error corrected mid-session**
`rc:solves_24h` at 08-28 02:55:00 EDT = **32** against threshold 33. FAILED
by one, and the gate was CORRECTLY calibrated: Claude first dismissed it as
non-diagnostic by computing σ = √33 — anchoring Poisson variance on the
THRESHOLD — when variance is set by the expected RATE. The 237-point curve
gives mean 53.6, so σ ≈ 7.3 and the threshold sits **2.8σ** below baseline,
firing by chance well under 1% of the time. Same error class as BL-028: a
constant treated as an expectation instead of reading the expectation off the
data.
Cause decomposed against difficulty, which was still in retention: D_k flat
(1.55e16→1.65e16, +6%) while D_z rose ~45% (9.6e15→1.45e16), ratio 0.62→0.87.
Difficulty explains roughly 58→45 of the decline. The trough is NOT
difficulty — 08-25 07:00 had D_z at its series MAXIMUM (1.653e16) with pace
healthy at 57, while 08-28 01:00 had LOWER D_z (1.502e16) at pace 32. A
variable cannot explain an effect it moves opposite to. Remaining ~45→32
(~29%) is capture-side, coincident with the outage + node-cutover + deploy
era, and was RESOLVED BY THAT WORK — not self-limiting (operator correction
to Claude's "self-recovered" inference, which was drawn from curve shape
alone). 08-29 20:00 = 51, inside the 51–60 baseline band, fully post-deploy
and post-cutover.
Also banked: the D_z/D_k ratio spans **0.62–1.106** over 225 hours,
including a period where zKAS difficulty EXCEEDED KAS. BL-028 called the
ratio unpinnable on four measurements; this is the same finding much louder,
and any Luck denominator reading a constant is wrong by up to 78%.
WATCH ITEM opened: `est_hashrate_z` appears BIMODAL ~18× apart — five
samples inside 28 minutes read 2.883e16, 3.014e16, 3.031e16, 3.041e16, then
**1.677e15**, with `d_z` flat at 1.468–1.491e16 and `hk` steady. A genuine
18× hashrate collapse at flat difficulty is not physical. The 1.677e15 value
matches a 08-27 reading, so it oscillates between regimes rather than
drifting. Sampler is reporting faithfully; the gauge is suspect. This is A3's
file, and A3 was scoped on a few-percent drift — 18× is a different
phenomenon under the same label. Ratio columns unaffected (difficulty-
derived). Read the distribution after a few hours of samples rather than
reasoning from five points.
**Lesson:** compute Poisson σ from the measured rate, never from the
threshold being tested. And do not infer a recovery MECHANISM from a
recovery's SHAPE — the operator knows what was done; the curve does not.
**BL-063 · 2026-08-28 · node — v1.0.6 adoption: four findings that outlive
the cutover**
Cutover record lives in NODE-CUTOVER-r1 + SCOPE r5 H7. The findings:
(1) **Config-key drop trap.** kaspad/zkas args assembly takes CLI-only for
five fields — `shielded-history`, `verify-shielded-history`,
`consensus-diag`, `shielded-anchor-overrides`, `externalip` — TOML values
parse then are SILENTLY DISCARDED (no `.or(defaults)`). Verified in source
at both tags; pre-existing. Corollary: `externalip` in zkas-node-758k.toml
(and plausibly kaspad's config) was dead the operation's whole life;
inbound peering works regardless. Consequence: those flags ride LAUNCHERS,
which are therefore load-bearing config (H2 must carry them verbatim).
(2) **No version banner.** `kaspad_env::version()` = workspace 2.0.1 on
every zkas release; no git hash. Sha-256 at download is the ONLY node
identity; BL-030's engine-prefix alarm is structurally blind to
zkas-version drift — the §3 contract re-review is the sole rail.
(3) **Forward-only boundaries live at COMMITS, not tags.** `e49ce61`
(08-06) changed the bincode scan-record layout mid-struct; only v1.0.6
(`5d94f7f`) reads a mixed archive. Our pre-cutover binary probed
pre-boundary (`ShieldedHistoryChunk` string absent). Cold-copy-before-
first-launch is now a STANDING upgrade step.
(4) **Release assets are re-clobbered post-tag** (v1.0.5 precedent; manual
`--clobber` recipe in upstream deploy.yaml). HEAD-probe + pin at download.
**Lesson:** a config file that parses clean can still be lying about five
of its keys; verify flag paths in source before trusting any config rail.

**BL-064 · 2026-08-28 · BL-032 addendum — deadlock candidate raised and
RETRACTED on reachability; class-level finding stands**
Upstream `6467cb3` documents an ingest deadlock (pruning-lock
blocking_write under the IBD flow's held read) whose symptom set matches
the 08-26 wedge exactly. Retracted as a BL-032 mechanism: the four defects
were each hidden behind the previous — in every SHIPPED build, defect #1
(missing chunk subscription) closes the connection before the deadlocking
acquisition is reachable; the only tagged builds where it is reachable
also contain the fix. Doubly confirmed: the code was absent from our
running binary entirely. What stands, class-level, for the upstream
report: RPC sessions starve behind the fair session RwLock with no fault
raised — a pending writer blocks all subsequent readers while p2p keeps
running. BL-032 root cause remains OPEN; memory pressure leads; H6 is the
only instrumentation path.
**Lesson:** reachability analysis before mechanism attribution — a
documented bug is not a candidate unless a shipped artifact could execute
it.

**BL-065 · 2026-08-30 · host/ops — the impostor night: a stale notes line
resurrected a legacy binary twice; the state-check button caught round two**
Sequence: a Session-1 disruption (~08-29/30, origin UNRESOLVED — forensics
open) killed the stack; the operator's CURRENT-STARTUP notes carried
`C:\zKAS\node\zkas-node.exe --configfile ...v106.toml` — a legacy-dir exe
(66D8296D…B4078E, era unknown), no launcher, no flag. Executed twice: the
impostor served production for hours (round 1), was killed and corrected,
then returned at 03:28 when the notes were re-run for the bridge relaunch
(round 2). check-kron.ps1 — minted that night, identity-pinning path +
sha + flag — caught round 2 on its first live run. Compounding find:
run-zkas-node.cmd was EMPTY on disk (truncation mystery open, timestamp
captured); rebuilt byte-identical from the CONVERSATION RAIL (law 16's
fourth rail, second recovery this month). Closure: legacy node exes
quarantined to C:\zkas\archive\legacy-node-dir\ so the wrong line now
fails loud; notes corrected; STARTUP-ORDER-r1 minted as the sole reference.
Cost: hours of unpinned-binary service (walletd scan-degradation watch
opened, BL-067), an evening of whack-a-mole.
**Lessons:** port-listening is not identity — state checks must pin path +
sha + flags; a start command living in personal notes is a defect (docs or
launchers only); an empty launcher is invisible until launch — integrity-
check launchers, not just processes (button r2).

**BL-066 · 2026-08-30 · analysis — the drought arc: no fault found, five
instruments minted**
00:00–08:27 EDT: ~4 solve events vs ~16 expected (event-luck ≈25%, ~10⁻³
for the window, ~2–4% of appearing somewhere in 34 days) immediately after
a 127% day. Pipeline exonerated by convergent instruments: near-misses
FLAT ~190/hr for 48h through onset, drought, rebuild, and sampler removal;
min-ratio table at the 1.000x% print threshold in every 6h bucket (no
classification floor); zero rejects; scrapes 12/12; difficulty/net-hash
flat (d 1.62e16→1.64e16). NetworkHistorySampler EXONERATED by data (flat
stream across its entire lifetime AND its removal) after being the
operator's suspect; disable ran as a controlled experiment.
Minted: (a) **the 100:1 law** — near-misses/hr ÷ 100 = expected solves/hr
(190/hr ⇒ 1.9/hr ⇒ 46/day; independently 15.3T/29.0P × 86400 = 45.6 —
two derivations, one number); (b) the 15-min boundary-bucket test — the
generic "did my change touch the pipeline" discriminator; (c) the
min-ratio classification check; (d) trailing-24h luck is LAGGING by
construction — since-midnight expected-vs-observed is the drought
instrument (the ~100% cards during the drought were arithmetically
correct); (e) the legs are ONE correlated solve process (72/74 doubles) —
per-leg Poisson intuition overstates events ~2×.
Standing trap documented with two live exhibits: **interventions self-
certify** (droughts end after anything) — the total rebuild did NOT end
the drought; the sampler disable "did" only in the way regression always
does. Dashboard's restart threshold (18 outcomes, 16 "effective") is this
trap wearing a UI. Verdict: cold window at the edge of plausible;
discriminators armed; a future 3σ cumulative drift WITH flat near-misses
is the escalation trigger (Prometheus rule queued).
**Lesson:** exonerate with mechanisms and proxies, never with the block
count — and apply the same standard to recoveries as to failures.
**Amended 08-30 ~10:15:** the 3σ trigger FIRED the same morning (~5 obs vs
~19 exp at 10h). Tail-histogram hunt ran per the escalation clause:
control window Pareto-perfect at every rung; drought window Pareto-perfect
through z≥30 (625/620 · 184/186 · 61/62), thin only in the extreme tail
(z≥60: 24 vs 31; z≥100: 9 vs 18.6, ~1.6% conditional); z≥100 == clears ==
submissions in BOTH windows — the evaluation bar is ACQUITTED share-by-
share. Deficit localized to genuine extreme-tail scarcity; no mechanism on
our side can thin above 30% while leaving below perfect. Per-rig tail
decomposition armed as the day-2 escalation; not fired (9 events too few
to decompose).

**BL-067 · 2026-08-30 · monitoring — task liveness, sampler census gap,
walletd degradation watch**
(1) `task=Running` is NOT liveness — Task Scheduler tracks the instance,
not the work; reporter truth = :9151 listening + log moving (button r2
rewrites check 7 accordingly; r1's 10-min log-age threshold also
miscalibrated for an event-paced log). (2) NetworkHistorySampler was
absent from the BL-039 census (created after, S14) — census-freshness is
a revision trigger; its console blink is structural (`-WindowStyle
Hidden` cannot suppress the console-host allocation frame for a task in
the interactive session; S4U principal is the fix, gated on a script read
that IS now done — sampler verified :3034-only, 2s lifetime, wedge-proof
by design). Oddity pinned: the census read its args WITHOUT the Hidden
flag at ~04:00; a direct read at ~04:30 showed the flag present — same
API, different answers, unresolved. (3) Walletd degradation watch OPEN:
post-impostor, `/api/wallet/history` polls timing out (07:58 WARN) and a
BEAT2 landed 38 min after its BEAT1 on a T+60s protocol — reporter
self-healed as designed (exact sompi delivered). First datapoint for the
scan-gap investigation; next block's dt is the probe.
**Amended 08-30 midday:** the watch ESCALATED live. Reporter went dark
~08:27 (no BEAT2/give-up for fb952a; :9151 False at ~10:5x) while the
bridge counted three doubles — the reconciliation alert's FIRST LIVE
CATCHES (09:02/10:04/10:52 cards), working exactly as designed. Recovery:
startup-replay-from-byte-0 + state-file dedup backfilled FIVE missed
blocks in six seconds (BEAT1+BEAT2, dt 0.3–3.7s, real txids) — the
timestamps show the wedged instance revived and fired the backfill just
before the task-tier restart cycled it: wedge-then-recover, not clean
death; cause unopened. Walletd measured healthy after (41/213/12 ms
balance; history sub-4s in the backfill). THREE give-up blocks stand at
provisional 45.24092998 pending exact-amount backfill: 7da0660e,
5740a96b, fb952a95. New PS5.1 trap banked: `-o $null` in an external
command expands to a bare `-o` — use `-o NUL`.
**Lesson:** read liveness off the work, not the wrapper — the same
principle that separated blocked-from-busy (BL-033) applies to tasks.

**BL-068 · 2026-08-30 · host — instant hard reset at 14:22:41, uncaptured;
hardware-class; operator's click coincident as trigger, not cause**
Timeline pinned by three independent clocks: block card 14:17 (full alert
path alive) · bridge log last write 14:22:41 MID-TABLE, uptime column
self-consistent to the second (03:30:31 + 10:52:10) · boot records
14:22:57/14:23:06 — a 16-SECOND death-to-boot turnaround. The 6008's
"previous shutdown 13:44:26" is DEBUNKED as the last-System-event floor
(bridge wrote 38 min past it) — that field is an estimate, never a death
time. Capture sweep EMPTY: no MEMORY.DMP, no minidump, no WHEA, no 1001
bugcheck, no 1074 command. An instantaneous reset Windows never saw
coming = hardware-class (DC brick/VRM transient under sustained all-core
load is the lead on a consumer mini-PC; window-activation iGPU spike as
the plausible last-straw — the operator's click on a red console at
~14:22:4x is trigger-coincident, not causal in any software sense; the
red console itself died unidentified with the session). Honest hypothesis
update: instant reset WEAKENS commit-exhaustion for THIS event (OOM is
sluggish, not instantaneous) while the reporter wedge keeps it alive —
two distinct fault classes may be in play. Gap only ~10 min (bridge back
14:32:03) — STARTUP-ORDER-r1's first live cold start. Banked property:
ZkasReporter AUTO-STARTS at boot (task fired itself; button showed its
log 18m fresh by 14:50). Forensics queue: PowerPanel PPPE_Db.db power-
event check for 14:22 · Reliability Monitor sweep · thermal/rail
instrumentation decision · the four-wordless-events pattern review
(session kill, launcher truncation, reporter wedge, this reset) now reads
as possibly TWO mechanisms, not one.
**Lesson:** date a death by the victim's own last write, never by the
event log's estimate — and an uncaptured instant reset is a hardware
conversation, not a software one.

**BL-069 · 2026-08-31 · zKAS chain-level — the reorg-resistance ladder is
three-tiered; 12 h is the irreversibility bar, NOT the 10-minute anchor**
Source-verified container-side against firecash/zkas-rusty tag `25be83a`
(the v1.0.6 pin), tarball read, not a GitHub UI read.
THE CONSTANT: `constants.rs:70` `FINALITY_DURATION = 43_200` (seconds);
`bps.rs:92` `finality_depth() = BPS * FINALITY_DURATION`. At `Bps::<1>` =
**43,200 blocks = 12 h**.
ENFORCEMENT, already-synced node: `virtual_processor/processor.rs:1703`,
`sink_search_algorithm` accepts a candidate sink ONLY if `finality_point`
is its chain ancestor; else the `warn!` at :1722 — "Finality Violation
Detected. Block … violates finality and is ignored from Virtual chain."
The node KEEPS ITS OWN CHAIN and does not follow. A reorg deeper than
finality_depth cannot move an already-synced node.
ENFORCEMENT, IBD: `protocol/flows/src/ibd/flow.rs:594` calls
`are_pruning_points_violating_finality` → peer disconnect on violation.
But that function's own comment (`processor.rs:2331`) states it detects
violations only at depth `2*finality_depth` and gives FALSE NEGATIVES
below that; and it tests against the node's OWN virtual finality point,
which a genesis-only node does not have. **A fresh sync inherits its
peer's view.** Same seam as BL-001/BL-002, approached from the hostile
direction rather than the benign one.
SECOND GATE, not predicted before the read: `post_pow_validation.rs:80`
`check_bounded_merge_depth` → `RuleError::ViolatingBoundedMergeDepth`.
`MERGE_DEPTH_DURATION = 3600` → merge_depth 3,600 blocks = 1 h at 1 BPS.
Its comment states the value is sized to roughly the DAA window duration
specifically to block low-difficulty side-chain merges. This is the
SHALLOWER gate and it decides whether a hidden chain can be reintroduced
at all — read it alongside finality, never instead of it.

THE LADDER at `Bps::<1>`:
  · < 1 h hidden — merge rules permit it; this is where a double-spend
    actually lives.
  · 1–12 h — reds can't be sneaked in, but a genuinely heavier chain can
    still take the sink. The real exposure window; costs sustained
    majority hashrate for hours and is visible in the difficulty gauges.
  · > 12 h — refused by every already-synced node. Only fresh-syncing
    nodes are takeable.

DEPTH TABLE (blocks / wall clock at `Bps::<1>`; every constant is
`BPS × duration-in-seconds`, so all wall-clock windows are RATE-INVARIANT):
  shielded_anchor_depth      600      / 10 min   (`600 * BPS`)
  merge_depth              3,600      / 1 h
  max_shielded_anchor_age 27,000      / 7.5 h    (`pruning_depth / 4`)
  finality_depth          43,200      / 12 h
  pruning_depth          108,000      / 30 h     (PRUNING_DURATION
    dominates; computed lower bound 63,398 at k=18, mergeset_limit 180)

OPERATIONAL LAW: `shielded_anchor_depth` is a SPENDABILITY gate — when a
note becomes provable — NOT a finality gate. Anything irreversible
against a counterparty, OTC settlement included, gates on 43,200 blocks
/ 12 h. Conflating the two numbers is expensive in exactly one direction.

THREAT CONTEXT (2026-08-31): zKAS ~30 PH/s. Kaspa live ~267 PH/s
(2Miners, ONE rail, unverified — other aggregators return garbage) vs
~1500 PH/s peak, so the threat surface is ~1200 PH/s of IDLE kHeavyHash
inventory, not diverted Kaspa hashrate. Merged mining drives an
attacker's MARGINAL cost to ~zero: keep full KAS revenue, redirect the
ZKMM commitment. Precedent: Coiledcoin, killed 2012 by a merged-mining
51% at no marginal cost. What 51% CANNOT do: mint. The turnstile
invariant, the per-bundle binding signature and the Halo 2 proofs are
not hashrate-defeatable — hashrate rewrites ORDER, never VALUE. The
live defense is economic, not cryptographic: no trading venue found for
firecash's ZKAS (search contaminated by the ZKasino ticker collision —
absence NOT established), and the actors capable of the attack are the
~73% of headers already earning ZKAS at zero marginal cost. That
alignment expires the day a liquid listing exists.
**Lesson:** a chain's irreversibility bar is a consensus constant, not a
block time — and the merge-depth rule, not the finality rule, is what
decides whether a hidden chain can come back at all.

**BL-070 · 2026-08-31 · docs/upstream — firecash's stated 1-BPS rationale
located; three of our nine rate arguments falsified against their source**
RAILS: `x.com/ZKas_X` → ROBOTS_DISALLOWED, zero coverage.
`discord.gg/3kp6SmPrD` → OG metadata only (title ZKas, 522 members);
message rail needs auth, zero coverage. Note zkas.info advertises a
DIFFERENT invite (`jysMS4XNFT`) and the launch announcement deep-links
guild `1521797800952729612`. The rationale was found on an unnamed
fourth rail: `zkas.info/whitepaper.html` §9. Rail-of-first-resort
discipline (law 16 amendment) applies to research targets too — the
named rails were the wrong ones.
STATED RATIONALE — shielded proof VERIFICATION is the gate. §9: a
shielded proof costs on the order of 100–700× a signature check, and at
one block per second naive per-Action verification would already
overwhelm a node under load. Named unlock: recursive per-chain-block
Halo 2 aggregation — one accumulator proof per chain block instead of
one per Action — described as the throughput lever that lets a private
ledger sustain higher block rates and as the primary post-launch
engineering objective. §15 Roadmap is BLANKED ("being revised"): there
is NO published plan to raise the rate. Absence of the commitment is
itself the datapoint.
POSITIONING: the whitepaper never benchmarks against Kaspa's 10 BPS
anywhere. The peer set is deliberately Zcash ~75 s and Monero ~120 s,
claimed at ~750× and ~1,200×. Their implicit reply to the critique is
that the critic chose the wrong comparison class.
OUR ARGUMENTS, FALSIFIED against their own source:
  (a) "emission is denominated at 1 BPS" — WRONG. The schedule is
      defined per second and divided by the block rate; per-second
      issuance is rate-invariant. Same construction Kaspa used at
      Crescendo. Rate is not economically load-bearing.
  (b) "time-based safety constants are expressed in blocks and scale
      wrong" — WRONG. Every depth is `BPS × duration-in-seconds`
      (`shielded_anchor_depth: 600 * BPS`); all wall-clock windows are
      rate-invariant by construction. Only memory/storage scales.
  (c) "per-block shielded state cost" — MOSTLY WRONG. §6.3 asserts no
      per-transaction cost depends on block rate: bundle subtrees built
      offline, chain-block subtrees in parallel with zero contention,
      one serialized append over a ~32-node frontier. The rate-invariance
      claim covers the TREE; the cost that does not scale away is
      per-Action proof verification (§9).
  (d) "1 BPS = 10× work per block = security" — arithmetically true but
      NOT their argument and in tension with it. §10: zKAS sets its own
      difficulty, its target is typically far easier than Kaspa's; §7:
      finality via a matured canonical anchor is the decisive reorg
      protection, not raw hashrate. DO NOT argue this publicly.
SURVIVING arguments: verification throughput (theirs, strongest) ·
unprunable nullifier-set growth, rate-bounded by a KIP-9 storage-mass
extension — our measured ~1.35 GB/day archival on Kron is the
operator-side reading of exactly this · node topology/count (ours,
unstated by them) · rate is the cheapest knob to hardfork later.
NEW LEAD: `params.rs:198`/`:230` cite "security audit F-04/F-05" as the
origin of `max_shielded_anchor_age`. We have no record that zKAS has
been audited and no report on any rail. Chase with firecash.
FOSSIL: the whitepaper appendix reads "Ticker: FC" — firecash lineage,
same family as the `h_fc`/FCMM fossils (repo map).
**Lesson:** test our own arguments against upstream's design document
before deploying them — four of nine died on contact, and the one that
survived was theirs, stated plainly in a section we had never read.

## 2026-08-31 — S16: reporter r3 on the money rail, the red solved, host verdicts

**BL-071 · 2026-08-31 · reporter — r1→r3 deployed (r2 VOID, never ran):
the provisional-zero defect closed fail-closed, walletd made visible,
BL-060's coupling fixed, and the error stream captured**
Defect found by reading the 08-30 boot in r1's own log: the reporter's
boot-start (proven 27s after power-on) races walletd's ~270s cold start;
`Update-ProvisionalAmount` on a failed poll leaves `ProvisionalAmt=0`, and
`Run-Beats` posts it — `14:23:26 provisional amount source: 0 zKAS` armed a
window where any found block ships amt(prov)=0 to the accounting rail (the
BL-052 zeroed-object class, on money). Corollary caught on the full source
read: the surplus guard is `-gt 0`-gated, so BEAT2 matching while provisional
is unknown runs WITHOUT the third-party-income filter and can cross-claim a
treasury row. r3 therefore gates ALL beats at the top of Run-Beats:
provisional <=0 → no BEAT1, no BEAT2, throttled defer warning (300s), blocks
queue losslessly; give-up clock only runs on b1-sent blocks, so a walletd
outage defers rather than degrades. Non-positive poll answers never arm the
value; the main loop polls walletd while provisional is unknown even with
zero pending beats, so a boot ahead of walletd converges in <=30s of it
waking.
Visibility (replacing the accidental walletd signal that r3's pump removes):
`zkas_reporter_walletd_poll_failures_total`,
`walletd_last_success_timestamp_seconds`, `provisional_known` — plus
Pump-Metrics at four beat-path sites so stalled polls can no longer starve
:9151 (BL-060's loop coupling, fixed). Alert-rule note pre-filed: naive
age-on-last_success is wrong (quiet nights age legitimately); gate on
failures increasing or age-while-pending.
ERRSTREAM (the r2→r3 delta, forced by an operator observation between cut
and deploy — sustained vs flash red): PowerShell's error stream is red on
console and invisible to Log() by construction; r3 flushes $Error to the log
per iteration (cap 5 + suppressed count). Proved itself pre-deploy on a live
exception (the DryRun port-9151 collision surfaced as an ERRSTREAM line
unprompted).
Deploy per house gates: source-identity F48D66D6…51DA4D matched mount and
Kron; r2 af977851…89dc4 VOID never deployed; r3
eb2b813d1c49bad11f46d077539c72cbbdaf78477541057245b99de80094753e (412 ln)
DryRun-accepted, swapped (.bak-pre-r3 = r1 bytes), verified at the RUNNING
artifact: r3 banner, six series, first two production blocks BEAT1→BEAT2
complete (78216a…, 10d7cb…, dt=0.9s both), Prometheus ingestion confirmed
against a known event to the second (last_success 1788165379 = the 04:36:19
BEAT2). Mount carries r3 only.
**Lessons:** a value only Michael's walletd can supply must never default to
a postable number — unknown is a state, zero is a claim. And an observation
arriving between cut and deploy is cheap to honor (r2→r3 was four anchored
edits) and expensive to ignore.

**BL-072 · 2026-08-31 · host/UI — the red console SOLVED: Windows Terminal
ignores -WindowStyle Hidden; red = focus-denial attention tint; crash-day
red and the click both exonerated; windowless goal STRUCK**
Identified at the command line: PID 1456, created 04:24:25 (the task start
to the second), running `-WindowStyle Hidden -File C:\zkas\zkas-reporter.ps1`
— WITH a visible tab. WT hosts task consoles as tabs regardless of the flag
(it targets classic conhost); both stack tasks carry the flag and both
surface anyway. All six stack consoles are tabs of ONE WindowsTerminal PID
(9220) — why window enumeration shows a single title.
The red: background-launched tabs are denied focus and tinted red-orange
UNTIL ACTIVATED. Sustained red = the reporter's long-lived tab (tonight
04:24→activation; crash day 11:02 wedge-recovery restart → the 14:20 click —
three hours of tint over a log clean 13:16→14:18). Flash red = the sampler's
transient tab every 5 min. Confirmed by operator test, not inference:
activation cleared the tint, content was nominal r3 startup + two blocks,
and — the control experiment 08-30 never had — clicking the red console on a
healthy machine caused NOTHING. Crash-day red carries zero diagnostic
weight; the click is double-exonerated (bridge-pen timeline + direct
repetition).
Theories retracted en route, all mine: ANSI escape bleed; active error-spew
(refuted by the clean log the instant it was read); stale scrollback;
"sampler flashing unsuppressed" (flag was already present); "hidden flag
means no console exists." The WINDOWLESS GOAL IS STRUCK from the plan: its
premise (console as crash trigger surface) died with the evidence, the
window is the operator's beats view, and r3's ERRSTREAM keeps its value
independently. Flags left in place, noted DORMANT: if the default terminal
ever reverts to conhost, both consoles silently vanish — pre-filed answer to
a future "where did the reporter window go."
**Lessons:** goals inherit the mortality of their premises — audit the task
list when a hypothesis dies, not just the verdicts. H2 service migration
gains its fourth motivation (tasks off the interactive desktop ends the
tint, the tabs, and the dormant-flag trap at once).

**BL-073 · 2026-08-31 · host — commit exhaustion ACQUITTED by continuous
curve; PowerPanel's first NEGATIVE verdict; two lookback misreads corrected;
BL-068 stands with zero pre-death observables**
The S15 unifying hypothesis (one resource curve crossing its ceiling
explains reporter wedge → red console → hang → reset) is DEAD on the
instrument installed 13 hours before the event: commit ratio 01:00→15:00
08-30 shows max 60.6% at 01:10 (pre-restart), then a POST-RESTART plateau
41.0–41.2% flat into the cutoff — 21 GB against a 50.9 GB limit, no ramp,
no stumble. Classic mimalloc step-plateau (BL-047-era MemLog shape),
nowhere near a ceiling. windows_exporter's first major verdict, 13h after
install. Independent corroboration of instrument accuracy: 60.6% @ 01:10 vs
60.5% measured by hand at 01:15.
PowerPanel, same event: Event Logs 08-29→31 contain ONLY the four rows of
the deliberate 00:16 plug-pull test — NO transfer at 14:2x. Premises power
ELIMINATED for the 08-30 event, by an instrument proven watching the right
unit (the test rows are the proof). BL-044's one-bit promise delivered on
the first post-install event — as a negative.
Corrections, mine: two Prometheus reads (a "6-min gap 14:27→14:33" and
"serving metrics at 14:25") were 5-minute-lookback smear over coarse steps —
carried-forward samples, not live ones. The bridge log's pen (14:22:41
mid-table, 16s to boot records) is authoritative and BL-068 already carried
it; container-timezone labels compounded the first read. Range-query gap
edges at step>=60s are ESTIMATES; the log pen is the clock.
Net: BL-068 unchanged and now fully fenced — instantaneous, uncaptured,
hardware-class, premises power out, commit out, thermal untestable (BL-075),
zero pre-death observables. The DC-path transient lead is strengthened by
elimination and by BL-076's headroom finding.
**Lesson:** an acquittal is worth as much as a conviction — this one killed
the only hypothesis that explained four events, and it cost one query
against data that existed BECAUSE the instrument went in the same night.

**BL-074 · 2026-08-31 · monitoring — KronHeartbeat deadman built and
DRILLED both directions; host death now has an off-box clock**
`C:\zkas\heartbeat-r1.ps1` (3F5CB4EC…5E2C7, 54 ln): one-shot on the sampler
pattern, pings healthchecks.io check `kron-deadman` (Period 5m / Grace 5m),
URL from `C:\zkas\heartbeat-url.txt`. PURE HOST LIVENESS by design — no
stack checks, so a bridge fault cannot page a false host-death; Prometheus
owns stack health and this exists precisely because Prometheus dies with the
host (BL-054's ~34-min asleep discovery cost). Failure-only logging: the
log's absence means the ping has never failed. Task 5-min repetition,
battery flags disarmed (BL-046), ExecutionTimeLimit 2m.
Verified at every layer: manual run exit=0 with the dashboard ping carrying
the WindowsPowerShell UA (ping #1's Chrome UA had proven only the receive
side); four unattended grid fires; then the DRILL — task disabled 09:33 UTC,
DOWN detected 09:40:02 (prediction ±2s), email DELIVERED to inbox (~05:40
EDT), re-enable → UP 09:45:02 with recovery email. Downtime 5:00 exact.
Both directions witnessed with nothing at stake — the BL-054/058 standard,
met before the instrument was a day old.
Recorded decision: the ping URL sits on the conversation rail — accepted for
its sensitivity class (possession enables fake pings, nothing else); the
same judgment that keeps wallet secrets off this rail absolutely.
**Lesson:** the drill IS the install. An alarm whose delivery has never been
witnessed is BL-044 with a subscription.

**BL-075 · 2026-08-31 · host — thermal is UNTESTABLE-BY-INSTRUMENT on this
board (scoped negative), not tested**
The thermalzone collector path resolved as: enable never applied (ImagePath
verified unchanged — six collectors + process filter intact throughout; the
step-2 probe queried an unmodified exporter), and then made moot one layer
down: `MSAcpi_ThermalZoneTemperature` answers **"Not supported"** at the WMI
layer itself — the exact class the collector reads. Board-scoped,
authoritative: the ACEMAGICIAN's ACPI exposes no thermal zones. Continuous
temperature trending is unavailable on this host; HWiNFO-under-load remains
the only open thermal path; the crash-series thermal hypothesis stays
UNTESTED, which BL-068's file must carry as distinct from ruled out.
**Lesson:** scope a negative before recording it — "collector found
nothing" and "board exposes nothing" are different claims, and one WMI read
promoted the first to the second.

**BL-076 · 2026-08-31 · host/power — brick label read: 65W, not 90W; the
box has run at ~87% of nameplate for months; replacement ordered; swap =
experiment start**
Label (AS0651-193402F): 19V / 3.42A / **64.98W** — the transcribed "9.0V"
falsified by the label's own arithmetic (64.98/3.42 = 19.0 exactly) and the
model string (…19**34**02 = 19V/3.4A). Every prior analysis assumed a
~90W-class unit loafing; reality: Kron+switch ~63W at the wall (BL-048,
UPS-corroborated) ≈ 55–57W DC against a 65W ceiling — **~85–88% sustained,
continuous, with essentially zero transient headroom**. A 5825U boost or
iGPU redraw spike atop that is exactly BL-068's "DC transient under
sustained load"; ceiling-limitation now joins degradation as the mechanism
(no capacitor aging required). DC barrel jack inspected by operator: no
play, no discoloration, no strain — CHECKED, narrowing the class to brick
(and residual board VRM only the swap can separate).
Replacement ordered: PERFEIDY 19V/6.3A/120W (B0GK119W8T) — fixed voltage,
5.5×2.5mm, center-positive STATED on listing, protection functions stated;
~2.1× headroom. SHNITPWR adjustable (incl. Pro) evaluated and REJECTED for
the permanent role: stepless 4–24V knob with an off-gear is a settable
power fault on a custody box (my tip-kit disqualifier was CORRECTED — the
native cable end is 5.5×2.5 center-positive; the knob argument stood alone
and sufficed). Install protocol: seat + wiggle-test any tip joint; DATE THE
SWAP — it starts the controlled experiment BL-040 prescribed, now running
unattended under PowerPanel + deadman + exporter. Resets stop → old brick
convicted (retained, labeled, as evidence and rollback). Reset on the 120W
unit → board-side fault, different conversation. Old-chat sequencing
("hold the order until discriminators run") is satisfied: they ran.
**Lesson:** read the label before sizing the theory — one worn digit hid a
25W assumption error that reframes the whole fault class.

**BL-077 · 2026-08-31 · process — the prior-work-sweep law minted; the
sitting's retraction set on the record**
Exhibits: the 19V brick was convicted, retracted, re-convicted, and refuted
across one sitting while BL-068's better synthesis sat committed; the 08-30
"instability" question was answered from live instruments before the S15
investigation of the SAME EVENT was read; two operator redirects ("we
already did this — go read it") were required. Same failure class as BL-057
one level up: closed-world conclusions from incomplete rail enumeration,
applied to FINDINGS instead of artifacts. Law filed to project instructions
(conduct tier, law 10): causal verdicts require a prior-work sweep —
conversation rail, ledger, then live instruments, in that order — and state
their evidence base; a verdict from a partial base is labeled provisional.
Sitting's full retraction set, for calibration: three red-console content
theories (BL-072); two Prometheus lookback misreads (BL-073); the
no-1001-means-no-bugcheck challenge (withdrawn when CrashDumpEnabled=3 was
read — S15's inference was sound); the SHNITPWR tip-kit claim (BL-076); the
windowless goal carried past its premise (BL-072); a drill-timing
prediction (09:18 down-call made before checking that ping #2 had reset the
window). Every reversal came from reading a rail nobody had read yet — the
lesson is the order of operations, not the humility.
**Lesson:** the model races to verdicts that lag the operation's own record
unless retrieval is forced FIRST; the operator's redirects are part of the
system, and the law exists to make them rarer.

## 2026-08-31→09-01 — S17: the accounting rail interrogated, A3 solved, first public filing

**BL-078 · 2026-08-31 · analysis — the Supabase rail answers six standing
questions in one sitting; the two-beat timestamps were an unread instrument**
Schema anchored from the webhook doc before any query (the mount's
"ZKAS_WEBHOOK_INTEGRATION_md.pdf" is actually a zip of markdown — noted;
readable either way). Key realization: upsert-on-hash means `created_at` is
BEAT1 and `updated_at` is BEAT2 — every row carries its own confirmation-
latency measurement, recorded since 08-21 and never read.
(1) BEAT2 LATENCY IS A WALLETD HEALTH INSTRUMENT, retroactively validated:
p50 sits 200–208s on every healthy day (found → chain index → next 30s poll);
the daily max reproduces the documented incident set EXACTLY — 08-27 883s
(premises outage), 08-28 929s (event eight), 08-29 3,132s (walletd wedge),
08-30 2,323s (reboot) — with five clean days 08-22→26 maxing ≤267s. Four
events, four spikes, zero false positives.
(2) 28 GIVE-UPS (amount frozen at provisional), ALL incident-era (7/4/9/8
across 08-27→30), zero in steady state. Per BL-069, chain blocks earn
mergeset fees atop subsidy, so any chain block among the 28 is UNDERCOUNTED
on the accounting rail. One-time reconciliation vs walletd history queued
POST-H8. Metric correction: gate on age>1h (a fresh block awaiting its
normal ~200s beat2 is not a give-up) and exclude 08-21.
(3) DROUGHT PERCENTILES, permanent calibration from 1,184 blocks: mean
0.503h · p50 0.34 · p90 1.16 · p99 2.50 · max 5.19h; gaps ≥3h occur ~7/1183
≈ twice a week. S16's opening anxiety (3.3h, 2 blocks) was a p99 event that
recurs every few days. Replaces per-incident Poisson arithmetic.
(4) FLEET ATTRIBUTION CLEAN over 25 days: KS7s ~352 blocks each vs KS0s
~32.5 each = 10.8:1 per-unit, matching the measured share-rate ratio;
within-class spreads inside ±1.7σ. No degraded-rig signal.
(5) Artifacts to read around: the 08-21 row (n=707, "latency" 4.10 days,
spread 187s) is the backfill's bulk insert + one bulk update pass — exclude
from latency stats; row-count drift between queries = live inserts, the rail
proving it's alive.
**Lesson:** Supabase is the operation's ONLY long-horizon series store
(Prometheus forgets at 15d; the ledger holds findings, not series). Interro-
gate stored side-effects before building new instruments — the best walletd
monitor of the month was two timestamp columns nobody had subtracted.

**BL-079 · 2026-08-31 · A3 CLOSED — the frozen hashrate gauge, end to end:
a stranded tip, a faithful estimator, and our own anchor choice**
Chain of custody, each link a read: 520-sample statistics (95.4% bit-
identical 1676882337221918, mid-band EMPTY between 5e15 and 2e16, distinct-
low-values = 1) → sampler EXONERATED (single series under the metric name —
no ghost to mis-select) → bridge source read at the running commit 1b63698:
ONE writer (`record_zkas_network_stats`), both arguments from the same
30s-tick response, error path skips both gauges — caller cannot mix stale
with live → node probed LIVE via gRPC (protos extracted from our own clone,
protowire-protos.zip 3f2d2531…62a76): UNANCHORED estimate = 3.101e16 then
3.118e16, distinct, ≈2×d_z as 1 BPS demands → ANCHORED on tipHashes[0]
reproduces the constant ON DEMAND → tipHashes[0] IDENTICAL across rounds
while virtualDaaScore advances and every other tip churns.
The anchor, dated: block e8dc1a034c0cfd99…555c12e, header 2026-07-31
11:26:54.799 UTC, daaScore 418,627, blueScore 415,031 — ~2.74M blocks behind
virtual; DAA-depth age (31.7d @ 1 BPS) matches header age (31.4d): a branch
that stranded at birth, blew past merge depth within half a day, and on an
ARCHIVAL node (no pruning) will never leave the tip set. Persistence
re-confirmed 4.6h later, still index 0.
Micro-mystery closed: Supabase's uniform …920 vs live …918 is the /metrics
text render truncating to 15 significant digits; gauge exact, text lossy.
THE DEFECT IS OURS: kaspaapi.rs:543 anchors the zkas estimate on
`tip_hashes.first()` — an unspecified-order list with a permanent squatter
at [0]; the same latent choice sits at :511 on the KAS leg, masked only by
10-BPS merge speed. Fix: `Some(tip_hash) → None` (virtual anchor) both legs.
**v2.0.1.6 SEED** via the standard CI + canary path — the gauge feeds
nothing load-bearing meanwhile; P2 sources from
rc:fleet_hashrate_delivered_hps regardless (measured 15.23 TH/s — the
dashboard's 14.2 nameplate understates Expected by ~7%).
Four intermediate verdicts reversed en route, each by the next rail in:
two-bridge-writers theory (killed by prom.rs), sampler-selection theory
(killed by the single-series read), a premature sampler exoneration, and a
node-side conviction (killed by the unanchored probe). The provisional-
verdict clause, exercised the day it was minted.
**Lesson:** an anchored estimator answers a question about its ANCHOR's era,
faithfully; the bug class is anchoring on unstable-ordered collections. And
the investigation cost one evening BECAUSE every prior layer (sampler
stats, source pins, gRPC access, proto extraction) already existed.

**BL-080 · 2026-09-01 · upstream — the batch cross-checked against venue
norms, cut 4→3, and the operation's first public filing landed**
Exemplar standard derived from kaspanet/silverscript's 20 open issues
(fingerprinted R/E/A/V/M/F + three full bodies): ONE defect per issue;
declarative first line; version pins; minimal paste-ready repro; observed vs
expected at the artifact level; measured numbers with METHOD credibility
(#218: "confirmed three independent ways… prediction matched a recompile to
the byte"); explicitly FENCED non-claims (#226: "no incorrect result or
covenant bypass has been demonstrated"); acceptance criteria; offer of
labor (#139). The two accepted outsider reports are written in this
operation's native dialect — the bar is form, not depth.
Batch re-formed 4→3: A3 SPLIT (bridge bug is OURS — fixed our side, never
filed upstream; observation + spec question UP; anchor-semantics docs note
queued) · finality ladder → docs PR, not an issue · 1-BPS material DELETED
from the batch on our own record's evidence (BL-070: four of nine arguments
died against upstream's design doc; the survivor was already theirs) ·
surplus/third-party-miner note → docs-gap report, pending txid data.
FILED: **firecash/zkas-rusty#6** (2026-09-01 UTC), body =
ISSUE-DRAFT-stranded-tip-r2.md (19bffbc3…3c941a, 94 ln), committed alongside
this entry; rendering verified by fetch at the far end — title, table,
three repro blocks, non-claims, questions, all byte-faithful. The
operation's first public artifact.
Corrections, mine: `open_issues: 0` misread as "empty tracker" — the issue
landing as #6 proves five CLOSED predecessors (a tracker that gets worked,
the better signal); and the first digest script shipped with a SyntaxError
from an unvetted f-string edit — law 5's dry-run discipline applies to
one-off analysis scripts exactly as much as to deliverables.
**Lesson:** grade the batch against the venue before filing — form-fit
first, and owning your own bug inside someone else's tracker ("that was our
bug, fixed on our side") is what separates a report that gets worked from
one that gets closed.

## 2026-09-01 — S18: the memory posture closed, and zkas-node found invisible

**BL-081 · 2026-09-01 · host/nodes — the 79% RAM reading is a CONFIGURED
EQUILIBRIUM, not drift; memory pressure ELIMINATED on the read/write split;
no config change on either node**
Opened on a Task Manager frame (21:02:26 EDT, pinned by the bridge log's own
console line): 24.8/31.4 GB in use (79%), commit 27.9/47.4, compression store
1.0 GB. The 79% sits one point under BL-032's ">80% RAM" wedge marker, on the
same instrument.
Uptime CONFIRMED not inferred: every stack process reads 54.6–54.7h,
`explorer`/`WindowsTerminal` at 54.7 bracketing the stack at 54.6 — one boot,
the 08-30 14:22:41 reset, nothing restarted since.
PER-PROCESS CURVES (windows_exporter `process` collector, scoped by include
regex to six stack binaries, running since ~01:00 08-30 — see BL-084(5)).
Two mechanisms, not one:
· **kaspad = step-plateau.** 1.43 → 10.89 GB. Twelve consecutive hourly
  samples at 9.51–9.54, then twelve at 10.13–10.16 — ±0.02 GB over half a
  day. Three step-ups, DECREASING: +1.17, +0.64, +0.21, then a creep. A
  bounded allocator on shelves.
· **zkas-node = smooth, shelf-free.** 0.44 → 5.34 GB, rate decaying by 12h
  block: 0.124 → 0.089 → 0.068 → 0.046 GB/h (ratio ≈0.72). Different
  mechanism; no `rocksdb-cache-size`, archival, defaults.
Attribution CLEAN: over the last 12h the two nodes grew +0.82 GB against
+0.72 GB of host growth. No third contributor. Growth leadership has FLIPPED
— zkas-node adds memory ~2× kaspad's rate at half the size.
Decay-fit equilibrium (a fit on four points of a derived quantity, treat
loosely): kaspad ~11 GB, zkas-node ~6.5–6.8, host commit ~28 GB, available
~6.5 GB, RAM ~79%. **The box is not drifting toward BL-032's marker; its
designed steady state IS the marker.** Reframes BL-032 rather than closing
it: not a leak crossing a line, a configuration whose equilibrium is the line.
**`rocksdb-cache-size = 8192` IS INERT.** `daemon.rs:243` computes a cache
budget only `if matches!(preset, RocksDbPreset::Hdd)`, `else { None }`;
`rocksdb_preset.rs:60-61` — `apply_default(opts, parallelism, mem_budget)`
never receives `cache_budget`. Under `rocksdb-preset = "default"` the value
is never read and the `info!("Custom RocksDB cache size…")` line never
prints. kaspad's 10.89 GB is `ram-scale = 2.0` alone, acting on the
consensus cache policies (`storage.rs:84`). Deleting the line changes no
behaviour; it is documentation.
`ram-scale` is therefore the SOLE memory lever on kaspad. Valid range
0.1–10.0 (`daemon.rs:107-111`). Help text targets 3.0–4.0 for a DEDICATED
64 GB node; Kron shares 31.4 GB across two nodes plus bridge, walletd,
Prometheus, Grafana.
PRESSURE TEST — the verdict. An initial read of
`rate(windows_memory_swap_page_operations_total[1h])` returned max 1221/s,
mean 252/s and was WRONGLY called non-zero-therefore-pressure: that counter
is perfmon `Memory\Pages/sec`, which counts hard faults of ANY kind
including file-backed I/O. The split is decisive:
· reads 100–300/s throughout, spikes at both ends of the range
· writes ZERO in 33 of 56 hours, max 18.6/s
· available memory fell 24.1 → 8.1 GB across the same window with the read
  rate FLAT — no correlation, therefore not eviction
· pagefile `PeakUsage` 300 MB of 16,384 (1.8%) corroborates independently
**VERDICT: no memory pressure exists. `ram-scale` stays at 2.0. No
configuration change on either node.** A prior "1.25–1.5" recommendation is
WITHDRAWN — it bought headroom the machine has no use for at a certain cost
in RPC latency, which is template freshness, which is revenue.
Banked for H6: `windows_memory_swap_pages_written_total` is the correct
pressure alert (near-zero baseline, a sustained climb is genuine eviction).
A RAM-percentage threshold is the WRONG rule — it would fire at 79% today
and mean nothing.
Also observed, no action: read spikes recur near 05:35/17:35, the same hours
as kaspad's allocation step-ups (compaction doing both at once); and the
final sample shows available jumping 8.12 → 11.55 GB with reads flat,
~3.4 GB released in an hour.
**Lesson:** occupancy is not pressure — on Windows the number that indicates
stress is paging, not how full RAM is. And a counter's AGGREGATE can be
uninformative where its component split is decisive: the same metric that
looked like a conviction at 252/s was an acquittal once read as reads-vs-
writes. Know what a counter counts before setting a threshold on it.

**BL-082 · 2026-09-01 · zkas-node/p2p — the node is STRUCTURALLY
UNDISCOVERABLE: zero inbound peers for the operation's life, open port and
all; BL-063's "inbound peering works regardless" falsified for this leg**
Measured: zkas-node **0 inbound, 10 outbound** against `outpeers = 16`.
kaspad on the same box, same gateway: **42/42 outbound plus 11 inbound**.
Everything local exonerated in sequence — both listeners bound `0.0.0.0`
(16811/PID 14824, 16111/PID 12904); both ports carry enabled inbound Allow
rules on Any profile (`zKAS Node`, `Kaspa P2P Inbound`); AT&T gateway
NAT/Gaming forwards 16811 → 192.168.1.96; and BOTH ports confirmed open from
outside via canyouseeme.org with **16111 as the positive control** (it must
be open — kaspad has 11 live inbound — which validates the instrument).
The port is reachable and nobody dials it.
MECHANISM, source-verified at the v1.0.6 pin:
(1) `flow_context.rs:907-917` — a peer's address enters an address manager
    in exactly two cases: `router.is_outbound()` (you dialled them), or
    `peer_version.address` (they SELF-REPORTED in the version message).
    **There is no path that records an inbound connection's source IP.**
(2) `addressmanager/src/lib.rs:116-128` — `local_addresses()` advertises
    `externalip` if publicly routable; otherwise falls back to enumerating
    local interfaces FILTERED to publicly routable. Kron has only
    192.168.1.96. The fallback yields nothing.
(3) `externalip` is a drop-trap field (BL-063) — parsed from the TOML then
    discarded.
Therefore the node advertises no address, and nothing on the network can
ever learn it exists. Compounding it: **zKAS mainnet is SEEDERLESS** —
`MAINNET_PARAMS.dns_seeders = &[]` (`params.rs:864`); only TESTNET carries
seeders. There is no crawler to find an unadvertised node, and peer
discovery is hardcoded `addpeer` plus gossip from there. (Gossip IS working:
8 of 10 outbound peers came from exchange, not the three bootstrap entries.)
kaspad is exempt not by configuration — its `externalip` is equally dead —
but because its address is already in the network's collective address books
from earlier in its life, where it propagates by gossip and persists across
restarts regardless of whether the node keeps advertising.
CONSEQUENCE for the 38-public-node figure: this operation is a member of the
population that figure cannot see. On a seederless network the reachable set
any crawler finds is bounded by what its bootstrap peers happened to know,
so two crawlers from different starting points can converge on different
totals. A DEFINITIVE public+private count is not obtainable — non-listening
nodes are unobservable by construction; the private population is estimable
only by capture-recapture across multiple listening observers, never counted.
CONSEQUENCE for `outpeers`: the setting was NEVER the lever on zKAS. The
node fills only 10 of 16 — the reachable pool binds first. Raising it is
inert. (The 32–42 diminishing-returns finding from July stands on its own
network: kaspad fills 42 of 42, so 42 is both achievable and evidenced there.
A recommendation to cut it to 16 was withdrawn on the operator's blocks-found
data.) `maxinpeers = 8` likewise exonerated as a constraint — nowhere near
binding at 0 inbound.
FIX, applied to the launcher by the operator and VERIFIED by direct read:
`--externalip=108.95.94.128:16811` on the `zkas-node.exe` line of
`C:\zkas\node-v106\run-zkas-node.cmd` (`require_equals(true)`, args.rs:477 —
the `=` is mandatory; `cd /d %~dp0` and `--shielded-history=on` both intact;
first byte 64, no BOM). Public IP re-confirmed unchanged at 108.95.94.128.
**STATE: ARMED, NOT ACTIVE.** PID 14824's argv carries only `--configfile`
and `--shielded-history=on`; the flag lands at the next node restart.
Acceptance gate: `External address is publicly routable 108.95.94.128:16811`
at INFO in the node log ("not publicly routable" = value rejected; no line =
flag never reached the binary). Then inbound peers in HOURS not minutes —
the address must enter a peer's addrman, propagate through the addresses
gossip flow, and be selected for a dial.
Standing fragility: a residential WAN IP change silently un-advertises the
node again, with no alert and no symptom but inbound quietly returning to
zero. `KASPAD_EXTERNALIP` exists as an alternative injection point if that
ever wants automating.
**Lesson:** a reachable port proves nothing about discoverability. On a
seederless network self-advertisement is the ONLY discovery path, which
makes `externalip` load-bearing for this leg in a way it is not for kaspad —
and a corollary proven on one node ("inbound peering works regardless") was
generalised one node too far.

**BL-083 · 2026-09-01 · kaspad/zkas args — config-surface audit at the
v1.0.6 pin: the drop trap is SIX fields, `perf-metrics` is honoured but
FILTERED, and one field is TOML-only**
Source-verified container-side against `firecash/zkas-rusty` at `25be83a`,
tarball read (BL-069's method), not a GitHub UI read.
(1) **The drop trap is SIX, not five.** BL-063 lists `shielded-history`,
`verify-shielded-history`, `consensus-diag`, `shielded-anchor-overrides`,
`externalip`. **`override-params-file` (`args.rs:655`) has the identical
shape** — `m.get_one::<String>(…).cloned()` with no `.or(defaults)`. The
header comment block inside `zkas-node-v106.toml` states five and needs
correcting.
(2) **`perf-metrics` is NOT in the trap.** `args.rs:635` uses
`arg_match_unwrap_or`, and `args.rs:673` guards it:
`.filter(|_| m.value_source(arg_id) != Some(DefaultValue))` falls through to
the TOML value when the flag is absent from the CLI. The key is honoured and
the monitor IS constructed and ticking. **Its output is discarded at the log
filter** — the callback at `daemon.rs:739-741` logs at `debug!`, compile-time
target `kaspad_lib` (the `[lib] name` of the kaspad crate, since the closure
lives in daemon.rs not in `kaspa-perf-monitor`), and the log runs at INFO.
Confirmed empirically: `matches=0` for `memory|resident|virtual|cpu usage`
across the whole 54h log. ~19,000 metric lines generated and thrown away this
boot. Fix if ever wanted: `--loglevel=info,kaspad_lib=debug` on the launcher.
Moot in practice — windows_exporter answers the same question with better
fidelity and no restart.
(3) **`block-template-cache-lifetime` is the INVERSE trap**: TOML-settable,
NOT CLI-exposed (`args.rs:638`, "currently used programmatically by
benchmarks and not exposed to CLI users"). Default 1000 ms. It looks like the
obvious mining knob and is NOT one: `cache.rs:62-67` clears the cache
whenever `VirtualStateApproxId` changes, so a stale template is never served
across a virtual-state change. Setting it to 0 buys nothing and costs
template rebuilds.
(4) **`deny_unknown_fields`** on the Args struct (`rename_all = "kebab-case"`)
means a misspelled TOML key is a HARD startup failure, not a silent ignore.
Both production TOMLs parse, so every key in them is a real key. This is the
opposite hazard to the drop trap and worth holding separately: unknown keys
fail loudly; known-but-CLI-only keys fail silently.
(5) Defaults banked: `outpeers` 8 · `maxinpeers` 128 · `rpcmaxclients` 128 ·
`async_threads` = num_cpus (16 on the 5825U, per node) · `ram_scale` 1.0 ·
`rocksdb_cache_size` None · `perf_metrics_interval_sec` 10 · zKAS mainnet
`default_p2p_port` = **16811** (`network.rs:246`; RPC+1 in the distinct "8"
block).
**Lesson:** read the arg-assembly line AND the consumer. A key can be
correctly named, correctly parsed, correctly plumbed all the way into the
running config, and still produce nothing — and the config file gives no hint
that its output dies two layers downstream.

**BL-084 · 2026-09-01 · process — command-shipping and reconstruction trap
catalog, second edition (five exhibits; all mine)**
(1) **PromQL label selectors do not survive `curl.exe` invoked from
PowerShell.** The Win32 command-line parser strips the inner double quotes,
so `{process=~"kaspad|zkas-node"}` arrived as `{process=~kaspad|zkas-node}`
→ `bad_data`, `parse error: unexpected identifier "kaspad" in label
matching, expected string`. Cost: four failed queries. Fix: omit the selector
and filter client-side (a scoped include regex caps the series count anyway),
or escape as `\"`. Sibling of BL-049(5).
(2) **A guard that gates on EQUALITY passes when both sides are absent.**
`if ($a.Count -eq $b.Count)` is true at 0/0, so a `status=error` response
flowed straight into the parse and crashed on null arrays. That crash was
then MISATTRIBUTED to a label-name guess, sending the diagnosis sideways for
two turns — the error text had been sitting in the response the whole time.
Gate on `status` FIRST, then on non-zero count.
(3) **`LastWriteTime` is not a liveness instrument on this box.** Measured
**41 hours stale** on `rusty-kaspa.log` while the owning process wrote to it
continuously (NTFS defers metadata updates for files held open with buffered
writes). It very nearly manufactured an incident: the stale timestamp
(08-31 04:28) sat inside the only anomaly in the 54h memory curve (the
04:35–05:35 excursion). Voids any "the log stopped at X" claim taken from a
directory listing — read the content's own timestamps. Applies equally to
the reporter, bridge and sampler logs.
(4) **`-AsByteStream` is PowerShell 7+**; Kron runs Windows PowerShell 5.1,
where the parameter does not exist. Use `-Encoding Byte`. **NODE-CUTOVER-r1
carries the same defect in its BOM-verification line**, which means that
verification has evidently never been executed on this host.
(5) **Four absence-or-state claims made from RECONSTRUCTIONS rather than
reads** — the expensive class:
   (a) "H6's per-process-RSS leg is open" — asserted from a metric-name query
       I had deliberately narrowed to `^windows_(memory|os)_`, which excluded
       the process collector BY CONSTRUCTION. The collector had been running,
       scoped to six binaries, since the exporter went in on 08-30. Several
       turns were then spent designing node-restart paths to obtain data
       already on disk.
   (b) The pagefile reboot-effectuation story — refuted by one query:
       `windows_memory_commit_limit` flat at 47.42 GB across 806/806 samples.
   (c) `externalip` as the cause of zero inbound — kaspad has the identical
       dead key and 11 inbound peers.
   (d) The launcher's contents reconstructed from NODE-CUTOVER-r1 and an edit
       proposed against the reconstruction. The operator had already applied
       the change manually. **The assert-count guard is the only reason that
       edit survived** — the script would have overwritten it.
**Lesson:** law 16 governs my own instruments and the runbooks, not only the
repo rails. A narrowed read establishes NOTHING about what it excluded, and a
runbook's description of an artifact is evidence about the runbook. Read the
file before proposing an edit to it — and keep the assert-count guard, which
paid for itself here by refusing to run.

**BL-085 · 2026-09-01 · corrections and housekeeping (four items)**
(1) **BL-073's commit-limit pin is FALSIFIED.** That entry cites "21 GB
against a 50.9 GB limit". Measured: `windows_memory_commit_limit` = **47.42
GB, flat, 806/806 samples**, 08-30 02:00 → 09-01 21:10. The two BL-073
figures are self-consistent with each other (21/50.9 = 41.3%) and both
inconsistent with the instrument: 41.2% of 47.42 is **19.5 GB**, and 21 GB
against 47.42 is 44.3%. The ratio is the likelier survivor (it presumably
came from a ratio query) but that is a guess; one query over 08-30
01:00–14:22 settles it. **BL-073's VERDICT IS UNAFFECTED** — 41% against a
47.42 GB ceiling is still nowhere near exhaustion, and commit exhaustion
stays acquitted for the 08-30 event. Only the pins are wrong.
(2) **Pagefile posture, first record on any rail.** `C:\pagefile.sys`,
`AllocatedBaseSize` 16,384 MB **fixed**, `AutomaticManagedPagefile = False`,
`PeakUsage` 300 MB. Reconciles the limit exactly: 31.4 GB usable + 16 GB =
47.42. Configured with Claude assistance at some earlier date and never
documented — a load-bearing host setting that existed on the conversation
rail alone until this entry.
(3) **H7's gRPC scoping was applied to ONE LEG OF A PAIR.** zkas-node's 16810
was scoped to the MacBook IP on 08-28 and the exposure recorded closed.
kaspad's 16110 still carries `RemoteAddress 192.168.1.0/255.255.255.0` under
a rule named `Kaspa gRPC LAN only` — the full subnet, i.e. the seven
unaudited-firmware rigs, against unauthenticated kaspad gRPC (which includes
`shutdown`, `ban`, `addPeer`). Losing the KAS leg takes merged mining with it,
since the AuxPoW parent comes from that node. Connection census: one Listen,
one loopback ESTABLISHED (the bridge), **zero LAN clients** — narrowing to
192.168.1.173 mirrors H7 with nothing at risk, and loopback is not filtered by
Windows Firewall so the bridge is unaffected. **NOT YET EXECUTED — no rail
carries a result.** BL-053's enumerate-the-whole-class lesson, recurring.
(4) **kaspad has no launcher and no witness rail.** It runs from a bare exe
path, `C:\rusty-kaspa-v2\target\release\kaspad.exe --configfile
"C:\Node-v2\config.toml"`, so BL-018 (PATH collision) and BL-019 (the next
local `cargo build` silently overwrites the production binary) are both live
simultaneously. And `nologfiles = true` means the largest process on the box
— 10.89 GB, 39% of commit — has NO log at all; its block-processing timing,
the line zkas-node prints every 10s, is invisible. This is why BL-032 was
unresolvable, and the fix went to the smaller node. **`nologfiles = false`
plus a versioned launcher is the highest-value remaining kaspad change**, and
neither is a tuning knob.
**Lesson:** a scoping fix applied to one member of a pair leaves the other at
its original scope while the record reads "closed" — enumerate the pair, and
verify the fix landed on both before writing the entry.

**BL-086 · 2026-09-02 · upstream/nodes — #6 closed CONFIRMED; the class was
6× bigger; Kron measured unhealed; and the field we should have read was in
the response all along**
Upstream closed `firecash/zkas-rusty#6` with a live confirmation: at the
first pruning-point advancement after their deploy (2026-09-01 11:41 UTC)
the archival cleanup fired on both public archival nodes and removed the
stranded tip **plus five more unmergeable side-branch tips** that had
accumulated unnoticed — `pruned 6 unmergeable side-branch tips`. Fix
`9a464d51` is on main, ships in the next tagged release; an upgraded
archival node self-heals at its first pruning-point advancement (~once per
finality interval, ≈12h at 1 BPS), no resync. Our report was explicitly
single-vantage ("we cannot say whether other nodes carry this tip"); the
close supplies the population. Accumulation is a steady-state property of
archival mode, not one 07-31 branch — `e8dc1a03…` was the one visible at
index 0, not the class.
**KRON IS UNHEALED — MEASURED, not inferred.** Probe 2026-09-02 from
`/Users/pearsonmw/zkas-lab/proto` (protowire-protos.zip pinned
`3f2d2531…62a76` at run — same proto set as the A3 probes, so directly
comparable): `tipHashes[0]` is still
`e8dc1a034c0cfd992c295703d775779fffe2ab467d9c7c7130c51b918555c12e`,
`virtualDaaScore` 3,287,534, depth behind virtual **2,868,907 ≈ 33.2 days**
@ 1 BPS. DAA advanced **116,939 (~32.5 h of chain)** since the 08-31
re-check at 3,170,595 and the squatter did not move. v1.0.6 predates
`9a464d51`; version ordering held, and the inference is now a reading.
**THE DISCRIMINATOR WAS AT THE API SURFACE THE WHOLE TIME.** The same
response carries `tipHashes` (3 entries) and `virtualParentHashes` (2) —
and the stranded block is in the former and NOT the latter:
`tipHashes` = e8dc1a03…, 2dec178a…, d42551f4… · `virtualParentHashes` =
2dec178a…, d42551f4… · `sink` = 2dec178a…. The node already knew the tip was
unmergeable and already excluded it from the list that matters. We anchored
on the wrong field — not on a field the node could not disambiguate.
Three consequences: (1) our question 2 has a better answer than a note about
`tipHashes` ordering — ordering is real but secondary, `tipHashes` is simply
NOT the anchor list; `sink` is the selected chain tip and is a single
specified value. `None` remains the v2.0.1.6 fix (nothing here argues for
re-anchoring), with `sink` named in a patch comment so the next reader does
not re-derive it. (2) **Strandedness is a one-line detector**:
`tipHashes` − `virtualParentHashes` = the unmergeable set, computable from a
call the bridge already makes on the 30s tick — a candidate metric, not a
520-sample investigation. (3) **The 08-30 WATCH item closes arithmetically
and was never a second phenomenon**: current `difficulty` 1.4994e16, so
2×d_z ≈ 2.999e16 against the frozen 1.676882337221918e15 = **17.9×** — the
"bimodal ~18×" oscillation is exactly the two anchors' difficulty eras, and
A3's root cause covers it with no residual.
Unchanged: v2.0.1.6 holds priority. The bridge fix is the ONLY remediation
on our rail until a tagged node release ships and we cut over, and the KAS
leg at `kaspaapi.rs:511` is a different codebase entirely — upstream's fix
touches nothing there, so that `first()` anchor stays live, masked only by
10-BPS merge speed. **NEW GATE**: watch for the tagged release carrying
`9a464d51`; the post-cutover acceptance criterion is free — the stranded
tips should leave `tipHashes` within one finality interval (~12h) with NO
resync, which is a cleaner check than anything in NODE-CUTOVER r1.
Corrections, mine, both in this sitting: (a) the first probe never reached
the node — `find ~/zkas` returned empty, `dirname ""` evaluates to `.`, so
`cd .` SUCCEEDED and grpcurl ran from the operator's current directory and
failed on a missing proto. An empty command substitution inside `dirname`
degrades to a valid path instead of erroring; any find-then-cd one-liner
needs the empty case guarded or it silently relocates the command. (b) I
carried the ledger tip as BL-080 while the mount read BL-085 — a five-entry
stale self-pin, corrected by reading the rail instead of the summary.
**Lesson:** read the WHOLE response, not the field you came for. A 46-hour
sampler, a 520-sample distribution, a source read at the running commit, and
a public issue all sat downstream of a single JSON body that contained its
own disambiguator two fields below the one we parsed — and the cheapest
instrument in the entire A3 chain was the one nobody ran: print the response
and look at it.

## 2026-09-02 — S19: event #9 fully witnessed, the power architecture decided, H8 rehearsed

**BL-087 · 2026-09-02 · host/power — series event #9: the first FULLY
WITNESSED event; premises acquitted twice over; money rail undamaged; brick
swapped and the experiment armed**
Timeline, all measured: last deadman ping ~13:50 EDT · death 13:53:17
(41+6008, **34s dark** — host self-recovered to the logon screen) · DOWN
email 14:00:02 (period+grace exact, instrument nominal) · operator home
~14:12, ATTESTED hard-off for the brick swap (its 14:12:51 41/6008 decodes
identically to the series — the attestation fence is the ONLY discriminator,
now demonstrated; without this line it becomes phantom event #10) · boot #2
on the 120W brick 14:12:49 · stack restart 14:14:35 · ping #677 ~14:15.
IPv6 interface-identifier flip corroborates the reboot boundary.
Both 41s decode BugcheckCode=0 / PowerButtonTimestamp=0 — seven decodable
events now, seven at 0/0.
PREMISES ACQUITTED TWICE: PowerPanel (proven watching the 1000VA by the
08-30 plug-pull rows) shows ZERO transfers; and the four rigs NOT moved in
the operator's UPS rebalance all read **~6d12h uptime — continuous through
13:53**, dating their last boot to the 08-27 premises-recovery, a
self-corroborating cross-check. (The three short uptimes are the attested
moves: w2m 35m, w5m 38m, w7m 21m.) Fault LOCALIZED downstream of the UPS
outlet: brick, barrel, or board.
DEADMAN SEMANTICS CORRECTED BY ITS FIRST LIVE EVENT: the host sat booted
and healthy at the logon screen for ~19 minutes, silent. Mechanism read,
not inferred: principal `LogonType: Interactive`, single time-trigger, NO
boot trigger; no heartbeat failure log exists → the task NEVER RAN (vs.
ran-and-failed). The instrument measures INTERACTIVE-SESSION liveness.
BL-074 amended accordingly; fix (boot trigger + non-interactive principal)
rides H2 with the sampler's.
H2's cost side, measured: host back in 34 seconds; PRODUCTION back in 24
minutes — and only because the operator happened to come home. That number
moves H2's service migration from hygiene to front-of-queue.
MONEY RAIL: ZERO DAMAGE — first incident in the series' history. Last
pre-death block (7628d0…, 17:16 UTC) fully settled before death; 20/20
blocks today refined, beat2 178–235s (max 210.6s post-recovery), pending 0,
post_failures 0. Contrast: the 08-29/30 incidents printed 3132s/2323s maxes
and 17 give-ups on the same instrument. r3's defer-gate fired IN PRODUCTION
for the first time (`provisional amount UNKNOWN … beats deferred` — r1
would have armed a zero), with ERRSTREAM capturing the walletd
connection-refused burst live.
Two chain observations rode the recovery window: a SUBSIDY STEP occurred
between 09-01 and 09-02 (45.24092998 → 42.70175169 — stepped emission,
fresh edge; all expected-value figures must roll) and the first OBSERVED
mergeset-fee delta: BEAT1 prov 42.70175169 → BEAT2 exact 42.94753769
(+0.24578600, a chain block earning fees — BL-069's mechanism, live, the
two-beat architecture visibly correcting money).
BRICK SWAP: PERFEIDY 19V/6.3A/120W in at 14:12:49 (operator attests ~14:15;
the boot pen is the measured edge). Wiggle-test done; old AS0651 labeled
and RETAINED as evidence. **Experiment armed with its falsifier stated:
quiet under the 120W brick is weak evidence (this series has shown 10+
quiet days); another 41/6008 with zero PowerPanel rows CONVICTS
barrel-or-board and acquits the brick.** Old-brick era closes at 8 prior
events plus #9 (~87% sustained loading, headroom-exhaustion mechanism per
BL-076).
**Lesson:** every instrument installed on 08-30/31 fired correctly at its
first live event — and the one that fired WRONG (the deadman's silence)
failed in a readable way that corrected its own spec. Nothing in this entry
is inferred; that is what the weekend bought.

**BL-088 · 2026-09-02 · power architecture — outlet map final, canary
designated, the router gap found, and the end-state decided and funded**
FINAL MAP (operator-executed rebalance; retires the H2 rider): 1000VA =
Kron + auxiliary, 52W, battery-backed (an ~11W drop from BL-048's ~63W
arrived with the new brick — unexplained, benign, noted). 1500VA-1 = KS7 +
2×KS0 battery-backed, 742W + **w8m (KS7) on SURGE-ONLY ← designated
premises canary**. 1500VA-2 = KS7 + 2×KS0 battery-backed, 752W. House
terminology fixed: **battery-backed** vs **surge-only** (CyberPower's own
labels); "pass-through" collides with UPS bypass-mode and is retired.
Canary trade stated deliberately: w8m is the fleet's top producer (31.1%
attribution) — correct as instrumentation (a reboot you'd notice), priced
as a real premises event costing its uptime + ramp.
SCOPING CORRECTION to every "zero PowerPanel rows" claim in the series:
PowerPanel watches the 1000VA ONLY; the 1500VAs retain no onboard history
and have no PC attached — they are DARK instruments. Correct scope (Kron is
the patient), but a rig-side-only premises sag is invisible everywhere
except rig uptime counters — the canary's real coverage.
ROUTER GAP, found by the operator: on premises loss the AT&T gateway (no
UPS) drops — both nodes lose all peers (templates freeze; the fleet hashes
a dead DAG) and the deadman's path dies, paging host-down for a healthy
host. FIX DECIDED: gateway → the 1000VA — fate-sharing with Kron is the
architecture (a separate upstairs UPS would create alive-but-blind mismatch
windows); ~15W on 52W is trivial. Gated on a first-floor→basement cable
drop; executes in the UPS-expansion physical window. Interim exposure:
known, detectable (PowerPanel row + port-census playbook), accepted.
END-STATE DECIDED: each KS7 on its own 1500VA with 1–2 KS0s (A: KS7+2×KS0
~83% — the one hot unit, four KS0s don't divide by three; B/C: KS7+1×KS0
~72%). All seven battery-backed; canary role RETIRES — succeeded by
NUT/witness-node detection + the gateway move (which becomes prerequisite,
not nicety). Third 1500VA **ORDERED**.
WITNESS NODE DECIDED AND ORDERED (~$100: Pi 4 2GB kit w/ case+PSU,
high-endurance SD bought separately, wired ethernet, powered from the
1000VA battery side): NUT multi-UPS monitoring (PowerPanel is structurally
one-UPS-per-PC; the 1500VAs get their first observer) → nut_exporter → the
existing Prometheus/Telegram rail, monitoring-only, drilled per BL-054
(three input-pulls, three witnessed alerts) before it counts. Identical
tri-unit discrimination risk flagged: serial-match in ups.conf, udev
port-path fallback if serials are blank. CHARTER beyond NUT, in order, one
at a time: (1) deadman successor/second-leg off-host (immune to the session
semantics BL-087 exposed), (2) rig-canary exporter (seven IceRiver UIs
polled to Prometheus — retires browser-tab forensics), then WAN-continuity
probe and an off-Kron backup landing zone. Firm NOs recorded: nothing
mining-critical, no node, no LAN-critical service, nothing inbound-exposed
— the box's value IS its independence, and drift surface is its rent.
Rig IPs still on no rail — KRON-HARDENING r3 carries them (twice now the
decisive test stalled on "where do I read rig uptime").
**Lesson:** the operator found the coverage gap the instruments could not —
the gateway sat outside every UPS while everything it enables sat inside.
Enumerate the DEPENDENCIES of protection, not only the protected.

**BL-089 · 2026-09-02 · upstream/H8 — BL-086's release gate is ALREADY
SATISFIED: v1.0.8 contains 9a464d51 (compare-verified); H8 re-briefed to
v1.0.8 with three corrections; the cutover class REHEARSED on the MacBook**
Reconciliation of BL-086's forward-looking gate: `gh api compare/
zkas-v1.0.8...9a464d51` → `status: behind, ahead_by: 0` — the fix is an
ANCESTOR of the tag. "Next tagged release" was v1.0.8 itself (tagged 09-01
10:07, hours after the fix landed). H8's acceptance inherits BL-086's clean
check: post-cutover, the six-strand class leaves `tipHashes` within one
finality interval (~12h), no resync. Maintainer's close also endorsed our
`None` anchor (index 0 "is not a semantic position") — the v2.0.1.6 fix is
now maintainer-recommended, and BL-086's `sink`/`virtualParentHashes`
discriminator goes in the patch comment.
v1.0.7 READ (the skipped version), three corrections banked: (1) the
v1.0.8 "shared chain tree reset" fix targets code our v1.0.5 walletd
PREDATES — the wedge class stays formally undiagnosed; the honest statement
is that walletd is TWO overhauls behind, and v1.0.6's "wallet lock no
longer held while serializing balance polls (10,507,750 → 185 bytes)" is
the plausible mechanism for our 10s-timeout bursts (acceptance metric:
poll_failures flatlines post-cutover). (2) Stranded-tip provenance =
the v1.0.2–1.0.4 disqualification-storm era — "known early-chain incident
window" decoded by release archaeology. (3) Skipping v1.0.7 was LUCKY:
v1.0.8's "v8 checkpoints load again" implies 1.0.7 broke checkpoint
loading; v1.0.5→v1.0.8 jumps the pothole.
v1.0.7 also brings: DNS seeder seed.zkas.info (peer pins droppable),
complete-history-on-every-node w/ frontier checkpoints (Kron's archival
stewardship stops being systemically load-bearing), and
`missing_history`/`history_complete` wire semantics — a NEW walletd refusal
state the reporter's fail-closed logic must recognize on the canary
(plausibly REPLACES the zeroed-object trap with an honest signal). p2p
default moved 16111 (1.0.7) → 16811 (1.0.8): launcher port audit rides the
cutover. Release binary naming changed (node ships as `kaspad`; new
`zkas-api`, `shielded-pay` binaries) — exporter include-regex and launchers
audit with it.
H8 DRESS REHEARSAL PASSED (MacBook, 09-01): the wallet app's bundled node —
a 13-era 2.8 GB datadir — upgraded IN PLACE to v1.0.8 and caught up at
~41k daa/minute to Kron's tip ±1 block, zero intervention. Same operation
class as Kron's cutover, executed first on the low-stakes machine.
**Lesson:** a gate written as "watch for X" must be re-checked against
evidence ALREADY HELD — this chat had verified containment fourteen hours
before BL-086 restated the gate as open. Cross-session merges reconcile
forward-looking claims, not just facts.

**BL-090 · 2026-09-02 · custody — zkas-wallet 13→29 on the MacBook: the
/reveal-era surface retired, the dead embedded engine explained, balance
re-derived sompi-exact, and the app joins the recorded stack**
The desktop wallet (self-custody + Covenants++ REST harness,
`info.zkas.wallet`, zkas-api :8500 loopback) was found SIXTEEN releases
behind — on no rail, watched by nothing; that it took a review prompt to
discover is itself the finding. 99-commit delta read
(~/zkas-lab/zkas-wallet-13-29-delta.txt): v1.0.13 carried the CUSTODIAL-ERA
surface removed at 1.0.19 (`/api/wallet/create`, `/import`, **`/reveal`**,
and a silent seed-fetch fallback in resolveDeviceSeed) plus plaintext key
fallbacks sealed at 1.0.27; the desktop connection chooser LIED until
1.0.19–21 (unlock silently reverted to the embedded daemon while the UI
showed the remote choice — `25d608d`). That commit EXPLAINS the operator's
observed history verbatim: a local node went offline, and every unlock
re-pointed at the dead backend regardless of selection. LIVE CONFIRMATION
pre-upgrade: zkas-api running with `--rpc-server=127.0.0.1:16810` and
NOTHING listening there — 1.0.27's "dead embedded engine" report observed
on this machine. Consequence recorded: pre-upgrade Covenants++ test
provenance (which node validated) is uncertain; balance figures of that era
were cache-or-remote, indeterminate by construction.
Upgrade per custody protocol: hex-key backup CONFIRMED offline first →
clean quit (sidecar killed by PID) → cold copy
`wallet-backup-pre-1.0.29-20260901-165823` (454 files / 3.5 GB, manifest
454, includes the node datadir — a full state snapshot) → DMG
`ed98e2dd6a2370280195d909ce42bcd303a7109117bc38e4d8e635cd3846e14f` (13 MB,
authenticated gh download; no published checksums — our sha at download is
the identity) → install → About **1.0.29** ("New in 1.0.17" popup = unseen
backlog, benign) → both wallets AUTO-REGISTERED (44d1f34's audit path; also
evidence the 13-era at-rest storage was readable unsealed — App Lock then
enabled, sealing into the 1.0.27 path) → embedded node updated in-app to
v1.0.8, synced (BL-089's rehearsal) → **balance gate PASSED sompi-exact:
200,451,802,596,746 sompi / 7 notes**, first trustworthy derivation since
the engine died — retroactively validating the dead-engine-era readings.
App Lock scope characterized: ONE challenge spans both wallets — no
per-wallet isolation; app-scoped seal. Correct per design; strengthens the
cold-storage-sweep case (with "custody component aged 16 releases
unwatched" as the second strengthener this week). New habit filed: wallet
releases are CI-stamped with no notes — the commit range is the changelog;
a release check joins session-open hygiene.
**Lesson:** custody components age silently — nothing in the monitoring
universe watches an app version. Put every key-touching binary on a rail
with a version pin, or it drifts sixteen releases with `/reveal` exposed.

**BL-091 · 2026-09-02 · process — cross-session merging PROVEN on the
rails; the header trap's third occurrence fixed; two laws minted; and the
sweep catching a live BL-collision**
The PRF0 session (memory posture, p2p discoverability — BL-081..085) ran
parallel to this one; merge executed by READING THE RAILS, not the chats:
mount ledger → full S18 ingest → boards reconciled → this session's content
renumbered around it. Defect found live: S18's merge shipped with the
self-header still reading BL-080 — THIRD header incident — fixed at
`3270a7c` (count invariant 2305, sha the only discriminator: law 15's own
exhibit). Two laws minted and filed to project instructions: **17
(prior-work sweep before verdicts, evidence base stated, partial-base
verdicts provisional)** and **2g (a merge is complete only when the merged
file's self-header reads back equal to the append's final BL id, before
commit)**. Both earned same-day: the pre-cut re-pin for THIS append found
the base moved 2305→2372 with BL-086 freshly banked by yet another session
— a blind cut would have collided at BL-086; the sweep renumbered S19 to
BL-087+ and dovetailed BL-086's open gate (BL-089) instead of duplicating
it. Three sessions, one record, zero bytes lost.
**Lesson:** the rails ARE the merge instrument — and they only work if
every session re-pins the base at cut time and reads what landed since.
The two new laws are the mechanization of exactly that.

**BL-092 · 2026-09-03 · upstream/process — BL-086's detector claim TESTED
(n=20, holds within stated scope); #6 addendum FILED and API-verified;
detector promoted to H8 acceptance instrument; three forward corrections;
laws evaluated 17→5**
**II.4 strengthening of BL-086** (amends BL-086 §(2), which asserted
`tipHashes − virtualParentHashes = the unmergeable set` flatly from a single
frame). Tested 2026-09-03 on Kron (v1.0.6, un-upgraded): 20 frames at 15 s
cadence, span 274 DAA (3,334,551→3,334,825), tip count 2–5. Difference set
was exactly `{e8dc1a03…}` in **20/20**; every non-stranded tip was in
`virtualParentHashes` on every read, including three frames at 5 tips / 4
parents (parent bound ≥4 observed). Log: `stranded-detector-zkas-2026-09-03.log`
20 ln `7191ddafd5ce4b51484b61a1f564247979c23eb10d01f8e74596962dfef72cdf`
(committed 49390b9). **Scope stated, not glossed:** untested at tip counts
above the virtual parent bound (never exceeded 5 here — 1 BPS zKAS will not
produce it), blind to transients shorter than the 15 s interval (bridge's
30 s tick is blinder), untested on the 10-BPS KAS leg where both modes
should show if they exist. The metric as defined: a tip persisting in the
difference across K consecutive `getBlockDagInfo` reads is stranded; K=10 on
the 30 s tick costs five minutes and is robust to anything the sampler could
have missed. "Zero new probes" in BL-086 was also overclaimed — persistence
needs no extra RPC, but *depth* (the discriminator that actually did the
work in A3) is a `getBlock` per member. Depth now 2,916,198 behind virtual.
**#6 addendum FILED** 2026-09-03T04:30:38Z as
`firecash/zkas-rusty#6#issuecomment-5520371274`. Published text pinned as
`ISSUE-6-ADDENDUM-r3.md` 72 ln
`348bb708e8febdb75dc736de4d14b5ed544471dc608b533613b0dace1873e2ca` (pulled
from the API body); r2 (73 ln 67fee2d9…, committed 49390b9) is the DRAFT —
two operator edits at post time (`not`→`NOT a reopen`; `Happy to`→`I can`);
r1 (44 ln 5a451740…) VOID, never posted, never committed. Rendering
verified via `GET /repos/.../issues/comments/5520371274`, not the HTML: the
logged-out issue page renders NO timeline (not even the 09-01 close) and
shows the issue as Open — a fetch-scoped absence that would have read as
"comment missing" under a single-rail read. `<details>` intact, 20 log lines
intact, JSON fence intact, host-leak grep 0. Adversary pass (named: a
maintainer who knows rusty-kaspa parent selection) surfaced four objections;
r2 answered three inline and DELETED the fourth — a counterfactual about
their six pruned tips, n=0 — rather than defending it. Issue open/closed
state unresolved from this rail (API rate-limited unauthenticated).
**Detector promoted to H8 acceptance instrument** (dovetails BL-089, which
found the release gate already satisfied by v1.0.8). The 20/20 log is the
pre-cutover baseline; the pass condition after v1.0.8 cutover is `diff=-`
on the identical sampler within one finality interval (~12 h, first
pruning-point advancement). Same command, no new tooling. Kron NOT cut over
as of this entry — operator-confirmed after a misread of "the fix has
fired" (upstream's 09-01 event, not ours); a run2 was shipped and correctly
withdrawn once the referent was resolved.
**Three forward corrections (IV.8):** (a) commit 49390b9 says "BL-087 owes
the II.4 strengthening" — BL-087..BL-091 landed from S19 (c01a095, evening
09-02) between this session's e4b518d and 49390b9; correct id is THIS entry.
Not amended: a forward id reference is corrected forward, not force-pushed.
(b) This session's recollection of the ledger tip was stale by exactly five
entries TWICE (BL-080 vs rail BL-085 at open; BL-086 vs rail BL-091 at the
addendum commit), both times because a parallel session advanced the repo.
Mechanism, not accident: the conversation rail is per-session, so any
session's memory of the tip is wrong the moment another session commits.
Id minted only from a rail read at cut time. (c) d288da5 was amended via
`--force-with-lease` from 7eba684 to replace a literal `<sha from step 3>`
that reached the pushed message — the IN pin of a 2f statement was a
placeholder; the remedy was `$(shasum …)` substitution, which the original
step should have used.
**Incident line (law 9), one shape, three instances:** constructions that
stay syntactically valid while losing their meaning. `dirname ""`→`.` ran
grpcurl from the wrong cwd instead of failing (empty substitution guard
absent); an angle-bracket token survived into a pushed commit because the
token lived in a fence and the fill-in lived in prose; a commit-message
excerpt shipped in a fence was executed as a command (parse error, no
state change) because under law 11 a fence IS the instruction signal. Laws
candidates: substitution over operator fill-in; fences deliver, backticks
name, quoted artifacts never fenced; empty substitutions fail loud.
**Skeleton sweep** (e4b518d): 8 lab-side ledger-append skeletons removed
after `missing=0` on 37 BL ids (SWEEP-MANIFEST-2026-09-02.txt, 8 ln);
BL-086's repo-committed skeleton removed (this session had introduced it —
every prior append lived lab-side). II.5 caveat stated with the gate:
`missing=0` proved header presence, NOT content identity; the manifest shas
are now unverifiable against anything. Norm: skeletons are not committed;
the append's cut sha rides the commit message beside OUT/IN. This entry's
skeleton follows that norm.
**Laws evaluation** (in-conversation, cut as SESSION-CONDUCT-LAWS-v2 queued):
17 laws + 5 floating amendments → 5 clusters (identity is content · claims
carry instrument, scope, and count · commands are deliverables · record is
append-only, synchronized, corrected forward · external artifacts pass an
adversary). Full 30-row disposition, nothing dropped; law 8 and the compiler
half of law 4 move to Part E per law 10's own tiering. Tested against this
sitting's 14 incidents: old set governed 3 (all compliance, not coverage);
new set governs 13 — the one it cannot govern is the trigger, which is
structural (a complete-feeling frame emits no signal). Highest-value new
provision: general or causal claims carry N and span inline. Rail-spelling
finding from 0715e6d (panel normalizes dots to underscores) belongs in
cluster I: filename equality across rails is not guaranteed; sha is the
only identity.
**Lesson:** the value-verification laws all passed on BL-086 — correct shas,
clean dedup, verified header — because the defect was a universal quantifier
over n=1, and no law asked for n. A claim's sample count is a value like any
other; written into the sentence, it lets an outside reader find the weak
joint without reconstructing the frame that produced it.

**BL-093 · 2026-09-03 · H8 EXECUTED — node + walletd to v1.0.8 in one window,
every gate passed, money rail verified against a canary; H8 stays OPEN on the
§13 tail (pruning point unmoved at close)**
**Timeline (Kron local, EDT):** walletd v1.0.5 stopped 01:24 · wallet dir cold
copies 01:25 (rollback + canary, 138.75 MB, 0 differing hashes across three dirs)
· node v1.0.6 stopped ~01:27 · datadir robocopy 01:27→01:34:11 (7m07s, 6554
files, 38.480 GiB) · v1.0.8 node launched 01:38:39, stopped, relaunched
**01:42:52** (PID 9448) after the 9b gate was re-read · node accepted via relay
by 01:39, tip-following through the window · canary walletd 03:35:21→~04:05 (PID
8460, port 8502, `--no-auto-consolidate --no-custodial`) · money-rail walletd
**04:09:32** (PID 17348), cache complete 04:25:24 · check-kron-r3 ALL 8 UP 04:27.
**Identities (all container-side pins matched on Kron, two independent downloads):**
zip `334cec3c31754318bca3832aab86fbcd75b9ae341cdcf2df825c2c9c9c7ebf40` 66,954,365 B
· node `45687E24E925C4ED777290C58B3A74B68339C8FA6C84F4E303533FC04652236D` (kaspad.exe
= zkas-node.exe copy) · walletd `B5B1DDA9093D1FB55A92A76D4D7CFE1ECDC67D57BCFCB0701FBFBAC7EF8C932C`.
Rollback identities: node v1.0.6 `1B49D1FA5416130A6CB82A166E5941E778EE1266E8BD5ACB23EA810B01DC97D2`
(pinned in check-kron r2 on 08-30 — **corrects NODE-CUTOVER-v1.0.8-r1's "never
pinned on any rail": rail-scoped absence stated as absolute**), walletd v1.0.5
`BDCBE0673C800720EF33D73EB68A4C6FBEBB10B3CA472E0822B8FDE08063713C` (launcher guard).
Config-key trap re-verified at `args.rs @ zkas-v1.0.8` (793 ln): same five
CLI-only keys; TOML copied verbatim (Compare-Object = 2 header lines);
`listen 0.0.0.0:16811` was already the v1.0.8 default → port move no-op;
`--externalip=108.95.94.128:16811` carried; node logged it publicly routable.
`perf-metrics=false` confirmed DECIDED (PRF0 09-01 Human turn; BL-081), header
comment was the stale line, not the key.
**Money-rail gate (§12), like-for-like on `/api/wallet/balance`:** baseline
(v1.0.5, live, 01:2x) `balance_sompi=6795485797267 note_count=481
scanned=3338868 history_total=1279 last_txid=17d8df16…`. Canary (v1.0.8, copy):
`6812566497943 / 485 / scanned 3348843 / missing_history=False` = base + 4
coinbases × 4,270,175,169 exactly, notes 481+4. Money rail (v1.0.8, real dir):
`6812566497943 / 485 / scanned 3349163 / missing_history=False` — **byte-identical
to the canary**. Two independent full rescans of the same file agreeing is the
strongest identity the wallet can give. Reporter `provisional_known 1`; the
window's deferred BEAT2s (c690635e4081, d7193e20a2d3, 5ef12d29976e, …) sent
with txids. v1.0.5 has no `/status`; v1.0.8 has none either — `note_count` and
`missing_history` ride the balance body. `notes[]` array (v1.0.5) is gone.
**Measurements (n=2, canary + money rail):** first v1.0.8 start on a v1.0.5
wallet dir is a **mandatory full rescan** — v1.0.5 checkpoints are not loaded;
"v8 checkpoints load again" applies to v8-written ones. Scan ~6.64M tree leaves
at 24.6–27.8 µs/leaf single-threaded, 99% of cost in tree build; subtree cache
**141.3 s / 147.5 s** (v1.0.5: ~270 s). Launch→warm **~14 min / 15m52s**.
**API SERVES HISTORY DURING THE RESCAN** ("off the wallet lock"): reporter's poll
succeeded 6 s after launch and cleared deferred beats; v1.0.5 answered nothing
for ~270 s. NODE-CONTRACT-v1.0.8 §6 owes this. Node: IBD of a 10-min gap in <60 s;
`Querying DNS seeder seed.zkas.info` (first use, peer pins droppable); `Network
mismatch` WARN from a 16111 Kaspa peer, benign; `shielded history … +0 records …
reached genesis after 1 rounds` — **benign**, settled by canary `missing_history=False`.
Banner reads `kaspad v2.0.1`; sha is the only identity, as NODE-CONTRACT §2 says.
**Detector, II.4 scope widened:** v1.0.8 pre-advancement 20/20 `diff=e8dc1a03`
(`stranded-detector-zkas-v108-pre-pp.log` 20 ln
`0df68368e3baec4b988a2448264cf0e4a139a645713d9835926e10c9fce72c4b`, DAA
3,350,191→3,350,393), including one frame `tips=8 vpar=7` — **parent bound ≥7
observed, difference still exactly one**. Cumulative: 40/40 across two binaries,
tip count 2–8, no counterexample. Stranded tip survives the binary upgrade;
pruning point `f864b7a2…` unmoved at close → §13 not yet readable.
**Rollback degraded to UNTESTED (node half):** 9a gated on "listeners = 0",
which proves ports closed, not process exited; RocksDB was still flushing when
robocopy ran. Backup 6554 files / 41,318,036,812 B vs source at read 6550 /
41,324,344,541 — four SSTs copied then compacted away, log grew after copy.
Source has since been written by v1.0.8; the copy cannot be retaken. RocksDB
WAL/MANIFEST recovery should open it; "should" is the word. **Rehearsal queued:**
v1.0.6 (`1B49D1FA…`) on a copy of the backup in a separate appdir/ports.
Walletd half clean (hash-verified triple copy, quiescent).
**Consumer found in-window:** `check-kron.ps1` r2 pinned v106 path + `1B49D1FA`
and would have remedied the cutover as an IMPOSTOR — its printed fix was the
forbidden v1.0.6-on-v1.0.8-data path. **r3 minted** (`C:\zkas\check-kron-r3.ps1`,
5535 B): node pins → v1.0.8, walletd remedy → v1.0.8-r1 launcher, **walletd
gains the same path+sha impostor check the node has**. ALL 8 UP at close.
**Runbook r2 owes (IV.7, one shape each):** token entry — `Read-Host` echoes,
`-AsSecureString` refuses paste, clipboard held a command (102 chars); guard
`$tok.Length -eq 17` on every block that uses it · 9a gates on `Get-Process`
absent · 9b identity read of the source AFTER the copy · poll exits on
`balance_sompi` present, not on a non-null body (a 400 satisfied the loop) ·
`-Filter *.log -Recurse` under node-data matched a RocksDB WAL (PRF0 trap #2) →
fixed log path · kill and launch from the same integrity level (canary launched
elevated, `Stop-Process` from non-elevated = Access denied; `taskkill` elevated) ·
canary dir was removed while the daemon still ran (non-terminating error let
the block continue; outcome correct) · check-kron listed as a cutover consumer.
**Artifacts on Kron only, owed to the docs rail:** `check-kron-r3.ps1`,
`start-walletd-v1.0.8-r1.ps1`, `start-walletd-v1.0.8-canary-r1.ps1`,
`zkas-node-v108.toml`, `node-v108\run-zkas-node.cmd`. Transfer path unrecorded.
Forward correction: `cc186c8` says "Mount ADD rides" for the laws doc — no mount
copy exists by design (governing tier = project instructions; repo = archive).
**H8 status: OPEN on §13 only.** Close = pruning point ≠ `f864b7a2…` → 20 frames
`diff=-` → node log `pruned N unmergeable side-branch tips`.
**Lesson:** the canary earned its cost twice — once as the gate it was designed
to be, and once as the thing that measured the rescan so the money-rail outage
was a known 16 minutes instead of an unknown one. And the checker's IMPOSTOR
line was the highest-value output of the night: an instrument that pins identity
will, by construction, call a correct upgrade an intruder — which is exactly
when you want it to speak, and exactly when you must not obey it.

## 2026-09-03 — S20 (this rail): H8's tail closed, the calendar cleared, the registry born

**BL-094 · 2026-09-03 · H8 §13 tail CLOSED (stranded tip gone ≤13h) and the
Button's impostor arc — dovetails BL-093's firsthand execution record**
BL-093 (the executing session) closed with H8 OPEN on its §13 tail: pruning
point unmoved, stranded tip still present at 04:27. CLOSED HERE: probe
09-03 ~14:45 EDT — **e8dc1a034c… ABSENT from tipHashes at daa 3,377,490**,
tips=3, all live/churning. First pruning-point advancement landed between
04:27 and 14:45 (~13h post-cutover, inside BL-086's one-finality-interval
gate); BL-092's detector pass condition (difference set {e8dc1a03…} 20/20
pre-cutover → empty) satisfied from our vantage. **The A3/#6 arc is CLOSED
end-to-end: gauge anomaly → 520-sample statistics → #6 filed → root-caused
→ fixed 9a464d51 → shipped v1.0.8 → cut over → healed on Kron. ~66 hours
discovery-to-cure.**
THE BUTTON, midday arc: `C:\zkas\check-kron.ps1` run ~12:30 EDT FAILed the
v1.0.8 node as IMPOSTOR — its contract pinned v106 path+sha+flag, so a
legitimate upgrade MUST fail it; that is the feature. Remedy NOT followed
blind (law 17): the process read (cmdline, node-v108 layout, launcher
provenance comment) showed a deliberate house-pattern cutover, so the
CONTRACT was updated instead: anchored patch v106→v108 (anchors 4/1/1;
backup `check-kron.ps1.bak-v106-pin`; binary pin 45687E24E925C4ED777290C5
8B3A74B68339C8FA6C84F4E303533FC04652236D; script now 7C477761365CA88AB8DC
E023D3865B5C4F7C216DBF9C14C56BE091BB76C6D3A6) → **8/8 PASS, "v1.0.8
pinned, flag on."**
RECONCILE OPEN (one ls settles it): BL-093 records "check-kron-r3 ALL 8 UP
04:27" — if r3 shipped as a separate file, TWO checkers now exist
(the patched original + r3); law 1d wants one live copy. Owed: enumerate
`C:\zkas\check-kron*`, consolidate, and fold R-6's walletd version pin
into the survivor (walletd passed its own 1.0.5→1.0.8 swap silently —
liveness-only check).
H8 residual watches (first days): beat2-latency p50 ~200s · poll-failures
flat · give-ups 0 · externalip "publicly routable" log line.
**Lesson:** FAIL → read the process → recognize deliberate change → update
the contract → re-arm is the impostor detector's whole lifecycle, executed
correctly on its first live trigger — and two sessions closing each
other's tails (their execution, our acceptance) is the rails working.

**BL-095 · 2026-09-03 · D2 PASSED (early call, exact-scoped) · SCOPE
retired · D1 executed — the calendar's coupled pair closes**
D2: `max_over_time(scrape_duration_seconds{job="rc_merged_bridge"}[135h])`
= **0.0491s** vs the 5s gate, **8,031 samples**, window exact-scoped
post-deploy and INCLUDING event #9's death-and-restart. Called at 5.65/7
days by operator decision, recorded as such (55s pin aged out 09-02; the
restart stress-sample outweighs two quiet remainder days). Cadence
correction en route: 8,031/135h = the bridge job scrapes at ~60s, not the
5-min assumption — prediction wrong, verdict 4× stronger. BL-050 CLOSED;
**v2.0.1.6 formally OPEN** (changelist in the registry).
SCOPE-v2.0.1.5 retired at 0715e6d (archive- rename; mount copy removed; r5
375 ln 864f9956… verified at the gate). Side-finding, unreproducible now:
the file was TRACKED but sparse-unmaterialized — absent from ls, clean
status, materialized by a cone re-apply; plus the rail-spelling exhibit
(panel normalizes dots→underscores; one artifact, two names — BL-092
cluster-I, second instance).
D1: five .bak-v2014 artifacts (4 source + the 15.2 MB rollback exe)
discovered by full-drive recurse, sha'd per file (identity record on this
rail), deleted; `remaining: 0`. Command defect owned en route: a stateful
$bak pair split across two fences nulled on standalone execution — III.6's
class; re-shipped self-contained. Coupling rule proven: D1 rode D2's
sitting, never standalone.
**Lesson:** an early gate call is legitimate when it is MORE rigorous than
waiting — exact-scoping beat the calendar's lazy [7d], and the stress event
inside the window was worth more than the quiet days outside it.

**BL-096 · 2026-09-03 · registry born · wallet 29→31 evaluated (BATCH) ·
corrections, one live collision caught, and archaeology**
ITEM-REGISTRY-r1 (129 ln 629ce4d4…, commit 4126f28, mounted): SCOPE's
retirement orphaned the D/H/P definitions — referenced 24× in the ledger,
defined nowhere living. The registry extracts them into a doc that does
NOT retire with a version; adds R-1..R-6 and T-1..T-5 as first-class
items; codes never reused. "What is H2" is now a mount read.
WALLET 29→31: notes era resumed AT v1.0.31 (the on-device "Run on this
phone" line); v1.0.30 is a CI stamp, content read via the 22-commit range.
VERDICT per the pre-committed rule: **BATCH — zero desktop custody/key/
money-display commits.** Desktop-adjacent: 9f55c10 (desktop gains
Config.build_shared_tree + node_socks_proxy) and bb5390e (macOS ad-hoc
signing — next DMG won't read "damaged"). UI: Consolidate renamed
**"Manage notes"** + split mode (1.0.30). P6 gains: view-key polish + a
phone-native watch-only option maturing. The release-check habit's first
full cycle: both releases evaluated within hours, against a rule fixed in
advance — the 16-release drift class is structurally dead.
INSTRUMENT LESSON (V.3/II.1): the logged-out GitHub releases INDEX served
a ~4-week-stale CDN render (v1.0.1 as "Latest") — presented as current it
would have "disappeared" 17 releases. Authenticated `gh` is the instrument
of record for release state; refetching the same surface is not a second
instrument.
ARCHAEOLOGY (three primary sources banked from that stale render's early
notes): (1) `<wallet>.scan.bak` + **`zkas-walletd --graft`** is the
documented restore for a broken rescan — "never rescan twice" now has its
vendor text AND recovery command railed; (2) the pre-1.0.6 poisoned-node
mechanism verbatim ("disqualifies every chain block above its pruning
point… while still looking alive"); (3) force-quit app-managed nodes →
RocksDB lock restart loop, handled since 1.0.2-pre4 — relevant to the
kill-the-sidecar upgrade step.
LIVE COLLISION CAUGHT (this train): step 2's base gate read 2776/11f285ad
with BL-093 present (the executor's entry, landed between sweep and merge)
against an expected 2684 — the instruction said stop; the merge ran
anyway; grep=2 exposed the duplicate BL-093 pre-commit; `git checkout --`
reverted; this append is the r2 renumbered around the firsthand record.
Nothing reached commit or push. Two notes: II.2's "any session's tip
memory is wrong the moment another commits" now has a same-DAY exhibit;
and the executor's push had not yet mount-synced (2f's ride pending on
that rail) — this train's sync brings the mount current for both.
CORRECTIONS (IV.8): BL-085(3)'s kaspad-gRPC gap was found ALREADY EXECUTED
(rule 'Kaspa gRPC MacBook only' @ .173; recorded pre-fix name obsolete;
executing session unidentified); dead disabled 'ZKas gRPC LAN only' /24
rule removed 09-02; program-scoped Any/Any finding railed at
KRON-HARDENING §6.8 / R-4. Hygiene: an r2-era KRON-HARDENING copy in
~/Downloads rode a mv into the r3 train, caught by the sha gate
(repo/mount untouched); norm: Downloads cleared of operation artifacts at
each landing.
**Lesson:** the registry and the release-check habit are the same repair
to the same wound — definitions and versions drifting on rails nobody
reads. Both now have a reader. And the gates only protect a train that
stops when they fire.

**BL-097 · 2026-09-03 · post-mortem — the August orchestrator (StartKron):
never ledgered, never completed a boot, killed in the act; H2 reshaped from
its five findings**
Prior-work sweep (law 17) on "we tried run-as-a-service and it didn't work":
the record corrects the memory — NO service was ever installed (the
mystery-kaspad-era census found zero services touching our binaries). What
was built 08-11 and killed 08-17 was a SYSTEM-context scheduled-task
orchestrator: `\StartKron` (BootTrigger, UserId S-1-5-18, HighestAvailable)
running `C:\Prometheus\tools\start-kron.ps1` — tiered startup, port gates,
env-baking via run-rc-merged.cmd, idempotent, self-logging. Its v2
(principal→inmyh, logon trigger) was designed and never shipped.
THE EVIDENCE IS ITS OWN LOG (start-kron.log, 2,732 B, read 09-03):
**v1 never completed a real boot — 4 for 4 truncated.** The only clean
end-to-end run was the 16:00 08-11 dry run (all SKIPs). Every actual boot
(08-11 16:04; the reset-era boots 08-15 10:53, 08-16 08:34, 08-17 00:51)
traces identically: `begin → START kaspad → START zkas-node →` SILENCE —
no gate line, no FATAL, no end banner. The script died seconds in, before
walletd/bridge/monitoring: after each reset the fleet pointed at a dead
stratum while two SYSTEM-owned, console-less nodes ran invisibly. The
"mystery kaspad" was not a side effect — it was the orchestrator's
half-finished work, four times. Death timeline now exact: the 08-17
00:51:00 truncated run spawned the final mystery kaspad; the task XML's
LastWriteTime (StartKron-task-ARCHIVED.xml, 01:01:35) shows it archived
TEN MINUTES LATER — caught in the act, killed same sitting.
WHY it died mid-run is unknowable from the record, because it could not
say: no try/catch, no finally, no exit capture — the log has no vocabulary
for its own death. Best mechanical suspect (hypothesis, labeled): the
Test-NetConnection gate throwing unhandled at T+~40s post-boot before the
network stack answers — consistent with dry-run-passes / real-boot-dies.
The evidence's verdict is only: died unobserved, four consecutive times
over six days, failures sitting unread in its own log.
FIVE FINDINGS → v2 REQUIREMENTS:
F1 SYSTEM principal made children invisible and console-less → principal
   is inmyh, never SYSTEM.
F2 The boot mechanism had no observer (survived its own presumed death;
   failed 4× silently) → **the orchestrator gets a deadman**: ping a
   healthcheck at successful END; a truncated run pages in minutes.
F3 Boot-start and crash-supervision were conflated, then abandoned
   together → separate items; prove boot-start first, supervision later.
F4 Consoles were the only runtime view; SYSTEM deleted them with no
   replacement → **visibility contract per process BEFORE migration**:
   log file + Prometheus metric + Button line + viewer command; consoles
   become optional, not load-bearing (the September groundwork —
   reporter/ERRSTREAM pattern, versioned launchers, identity-pinned
   Button, kaspad nologfiles=false queued — is most of this already).
F5 The tool itself was un-instrumented → lifecycle scripts wrap
   try/finally with failure WRITTEN; LastTaskResult is a checked value.
FINDING #0: none of this was ever ledgered — the failure lived on
conversation rails only, and ITEM-REGISTRY r1's H2 ("→ Windows services")
was written in ignorance of a three-week-old failure in the same
neighborhood. This entry is the repair; registry r2 rewrites H2 to the
staged shape (visibility contract → inmyh boot-start with its own deadman,
proven across a deliberate reboot → supervision as its own later item).
WHAT SURVIVES, validated: tiered order · port-gates-not-sleeps (plus a
network-readiness pre-gate) · idempotency-by-name · env-baking via
run-rc-merged.cmd · file logging. v1's architecture was right; its
principal, error handling, and observability were wrong — the three
things the operation now does well.
**Lesson:** a failure that is not banked is a failure the operation is
condemned to redesign toward. The sweep cost one search and rewrote H2's
premise; the log had been holding the whole answer, unread, since August.

**BL-098 · 2026-09-03 · H8 CLOSED — five acceptance gates, five passes; the
walletd hypothesis confirmed by its own counter; the deferred queue proven
lossless at scale**
Gates (read ~19h post-cutover): (1) beat2-latency p50 **198.9s** across 39
blocks — dead-center of the healthy band. (2) ERRSTREAM quiet since the
cutover's own walletd start-window (last hits 04:09:09–28, walletd up
04:09:32). (3) walletd poll-failures **FLAT — increase()=0.0 over 6h**;
lifetime 1,501 is cutover archaeology in full (~2¾h of connection-refused
at retry pace, 01:24–04:09) — **BL-089's mechanism hypothesis CONFIRMED:
the v1.0.5 wallet-lock serialization was the timeout source; v1.0.8's
185-byte polls ended it.** (4) Discoverability proven BEHAVIORALLY,
exceeding the planned log-line gate: **three non-LAN peers established on
16811** (138.88.31.204, 169.155.235.243, 85.215.243.87) — the node that
was structurally undiscoverable ten days ago (BL-082) now found and held
by the open internet. (5) Give-ups **0 since steady state**; two in-window
rows (found 06:02/06:46 UTC, walletd down) identified by the
insert-default signature `updated_at ≤ created_at` = BEAT1-only, never
BEAT2-updated — same class as the standing 28 give-up rows; ANNEXED to
that queued reconciliation (now 30 rows, all incident/outage-era).
The beat2 max (2,049.9s) resolves as the flip side of gate 5: a block
found deep in the outage, refined the instant the subtree cache completed
— **r3's deferred queue proven lossless across a 2¾-hour walletd outage**,
its strongest exercise to date.
NEW FINDING → H2(a) broadened: the node writes NO log file (`nologfiles`
rides zkas-node's config too, not only kaspad's) — the externalip
activation line went to a console nobody kept, which is why gate 4 needed
the behavioral read. The visibility-contract rider now names BOTH nodes.
Registry r3 flips H8 to CLOSED alongside this entry. Post-H8 unblocks are
live: the (now 30-row) give-up reconciliation · P6 · R-1's
missing_history-aware semantics.
**Lesson:** behavioral gates beat declarative ones — a log line would have
said "I believe I am routable"; three strangers holding connections says
the internet agrees. Prefer the read that cannot be sincere-but-wrong.

## 2026-09-04 — S21: Telegram block cards shipped (r4→r5), the dashboard sitting closed

**BL-099 · 2026-09-04 · reporter r4→r5 — Telegram block cards live via the
08-25 architecture; the fail-open law passed its first live test; two wire
findings; dashboard work order 5/5 with cross-check**
DESIGN (the 08-25 record's own plan executed): the reporter — holder of
hash, worker, exact amount, txid — sends the block cards; the Alertmanager
card demotes to independent backstop. One card per block: born at BEAT1
(~T+5s, provisional amount, faster than the alert path's structural
T+37–127s), EDITED IN PLACE at BEAT2 (exact amount + explorer
/transactions/<txid> link; editMessageText keyed by message_id **persisted
on the block state**, so edits survive restarts; give-up edits the card to
"provisional stands"). Alert-path alternative (hash-labeled gauges, 08-11)
REJECTED: series-per-block is BL-024's churn class and BL-025 says never
reapply the birth idiom. LAW stated at design time: the TG leg is strictly
FAIL-OPEN — any Telegram failure logs and drops; beats, queue, money rail
untouched. Token = Alertmanager's own file (BL-014 lane); chat 8180943473.
r4 (1DF8C061…) deployed 00:00:49 by operator. FIRST LIVE BLOCK
(4c2233d675f6, 00:01:50) = the fail-open law's FIRST LIVE PASS and a
finding: both beats clean, money rail untouched, ERRSTREAM caught TG 400
"text must be encoded in UTF-8" — **PS 5.1 re-encodes STRING request
bodies as Latin-1**; the ASCII smoke test passed while the card glyphs
died. (The script-side twin was caught pre-ship: PS 5.1 reads BOM-less
scripts as ANSI — r4 shipped with UTF-8 BOM.) r5 (9B4628AB…, 479 ln) fixes
the wire: body as UTF-8 BYTES + charset in Content-Type. Second finding at
swap: r5 lost the :9151 bind RACE to r4's HttpListener teardown
(Stop-ScheduledTask returned before the port freed) — dedup'd
kill-and-restart; single instance PID-verified, metrics rebound. r5 LIVE
00:16:04, all pre-block gates green; **first-card witness OPEN** (next
block is the test).
DASHBOARD SITTING CLOSED (KDSM/Netlify, Bolt lane, display-only): work
order 5/5 — (1) projection verified NO-OP: card is pace extrapolation
(count/hours×24), no hashrate denominator — immune to the endpoint error,
noisy-at-small-N by construction; future card noted: "Expected (network)"
from network_history avg × fleet = BL-066's 100:1 law in UI, pace-vs-
expected as the drought instrument; (2) BOTH hashrate cards re-derived
from network_history (now + 24h avg), old kaspaApi path REMOVED —
**cross-checked: dashboard 28.4 vs Prometheus avg_over_time 28.9 PH/s,
1.7%, two samplers one quantity — the 08-25 mis-derivation class closed**;
(3) interval mixed-window labeled (lifetime avg + 24h line); (4)
restart-efficacy machinery DELETED (BL-066's self-certification trap
un-wore its UI); (5) nameplate 14.2 labeled "capture-efficiency KPI"
(BL-016(b) by design) + code comment pointing delivered at the reporter
feed. P2 RESOLVED HONESTLY: a hardcoded 15.23 would violate BL-028
(constant = measurement minus timestamp); Netlify-remote means no LAN
Prometheus fetch ever; delivered arrives when the r-series posts the
gauge — r5 is now the natural vehicle (r6 candidate with the difficulty
feed). Dashboard's role railed: the ORIGINAL remote deadman ("are we
okay?" from anywhere), zero-inbound by architecture; four instruments,
four silences (dashboard=money flowing · TG=events · deadman=host ·
Button=exactly right).
PROCESS: two IV.2 violations called by the operator (instructions
referencing fences in scrollback instead of carrying them) — the clause's
cost mode is exactly this week's r2-in-Downloads ride; correction: every
execution request carries its fence verbatim, including closings.
**Lesson:** ASCII smoke tests certify the pipe, not the payload — test
with the bytes production will send. And a port is not free just because
its owner was told to stop.
