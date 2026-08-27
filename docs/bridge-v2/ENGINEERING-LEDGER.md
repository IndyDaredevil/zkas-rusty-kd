# ENGINEERING LEDGER — zKAS/KAS Merged Mining Operation
### Standing, append-only record of bugs fixed, major corrections, and lessons learned.
### Convention: new entries appended at session close with the next BL-### id.
### Session-state docs reference this file; do not duplicate its content there.
### Last entry: BL-044 (2026-08-27)

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
