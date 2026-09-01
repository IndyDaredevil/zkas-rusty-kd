# ENGINEERING LEDGER — zKAS/KAS Merged Mining Operation
### Standing, append-only record of bugs fixed, major corrections, and lessons learned.
### Convention: new entries appended at session close with the next BL-### id.
### Session-state docs reference this file; do not duplicate its content there.
### Last entry: BL-080 (2026-09-01)

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
