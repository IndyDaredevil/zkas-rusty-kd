# Specification: Merged-Mining Stratum Bridge v2
### Target: zkas-rusty in-tree bridge (v2.0.1 RKStratum lineage) · Draft 3 · 2026-08-04
### Draft 2 decisions: fate tracker KEPT (WS3-FT); WS4 plain-parent degradation KEPT as spec'd (flagged, default-on after V6)
### Draft 3 (post-V5 revision): V5 BANKED on reference bridge with day-one production stats; invariant 7 (magic bytes) added; WS3 warm-up fix; zkas-address handling first-class; WS5 upstream collision (d79bf68) flagged; stale pre-757k framing scrubbed

## 1. Purpose

Upgrade the modern in-tree bridge to a production merged-mining bridge for
KAS + zKAS, replacing the vendored zkas-pool bridge currently used for
production. **Lineage correction (2026-08-05, verified from the pool
bridge's UPSTREAM.md and a direct diff):** the pool bridge was re-vendored
2026-06-13 from RKStratum v1.1.0 to **v2.0.0**, and upstream's bridge/ is
byte-identical between v2.0.0 and v2.0.1 — so the reference bridge and the
port target share the SAME stratum core. The port is therefore an
extraction of katpool's merged-mining layer onto a clean identical core
(shedding pool-only baggage: event bus, session DB, PROXY protocol,
multi-port ADR-0022), not a cross-version forward-port. There is no
performance difference between the cores; the port's value is lockfile-
drift immunity (in-workspace build), reduced surface, WS2–WS8, and
upstreamability. Success = one bridge process that (a) matches or beats the
operator's tuned RKStratum on KAS solo capture, (b) earns zKAS at zero
marginal hashrate cost, (c) is operable: observable, degradation-tolerant, and
loud about the right things.

Everything here is informed by the 2026-08-02/03 live soak and the 2026-08-04
production cutover of the reference (pool) bridge — now **v0.3.3-win**
(`IndyDaredevil/zkas-pool-kd` @ d5c3e29, post-ZKMM-lockfile-fix), serving
6 rigs / ~11 TH/s with w7 held out on RKStratum as control. The pool bridge
is the reference implementation and the live diff target for behavior.

**Field results — day one of production (v0.3.3-win, through 23:59 ET
2026-08-04):** first zKAS block 14:51 ET (during final testing); full
production go-live ~16:00 ET after testing/deployment/troubleshooting.
- **21 zKAS blocks** settled and paid (~1,130 ZKAS) in a ~9.1h earning
  window (14:51→23:59) — expected ~15.8 at 11 TH/s vs ~22.9 PH/s network
  → **luck ≈ 133%** (hot; per spec discipline, no luck figure is trusted
  before a 48h window).
- **15 merged-parent KAS blocks** paid (~36.7 KAS). NOTE: the KAS count
  spans a longer window — first merged-KAS at 11:59 ET, pre-fix, when the
  FCMM bug was rejecting every aux submission. Pre-14:51 KAS finds are
  therefore lost zKAS doubles, and the observed full-clear rate
  (15/21 ≈ 71%) is upper-biased by the window mismatch. Reading: ~71%
  observed, consistent with the 55–66% difficulty-ratio prediction at
  n=21 (±2 blocks binomial noise); re-derive on clean post-fix windows.
- First confirmed **double** (one hash, both chains paid) 17:07 ET.
- **Combined KAS day total: 40 blocks** (merged bridge + RKStratum) — on
  a cutover day full of testing, restarts, and fleet churn, KAS capture
  held at the ~38/day statistical expectation for 14.2 TH/s. The zKAS
  revenue line was added at measurably zero KAS cost: the zero-marginal-
  cost thesis, demonstrated at day-ledger granularity.
- KAS solo baseline (RKStratum + control rig) unharmed throughout.

These figures are the quantitative bar the v2 bridge must meet in V7:
same luck-per-TH on both chains, same zero-red record, at equal or better
stale%.

## 2. Scope

**In:** the eight workstreams below, applied to `zkas-rusty/bridge` in the
operator's fork, upstreamable as PRs.
**Out (v2):** multi-node template failover; any web dashboard beyond what the
bridge already ships; pool/payout accounting; non-Windows service tooling.

## 3. Architecture (unchanged, restated as contract)

**GOVERNING PRINCIPLE (2026-08-05, operator-set): KAS-primary inversion.**
The reference (zkas-pool lineage) bridge is zKAS-primary: its main client
is the zKAS node and Kaspa is the optional add-on. The port INVERTS this:
the primary client is the Kaspa node — the unmodified, production-proven
RKStratum path — and the zKAS commitment is the optional enhancement
(`zkas_client: Option`). Consequences: KAS-ONLY is the BASE state, not a
degraded one (the bridge with no zKAS node is simply RKStratum); MERGED
is the enhanced state; startup order is irrelevant (KAS mining begins the
moment the Kaspa node is reachable, the commitment attaches whenever a
zKAS template source appears); and commitment attach is NON-BLOCKING —
a zKAS hiccup means the next job goes out plain rather than late, never
delaying KAS job cadence. Protecting KAS mining at all costs is the
primary reason the port targets the RKStratum lineage; this principle is
that reason expressed as architecture. It is also the most upstreamable
shape: to rusty-kaspa, this is RKStratum plus optional AuxPoW child-chain
support — a strict superset, zero behavior change when unconfigured.

```
IceRiver ASICs ──stratum──▶ bridge ──gRPC──▶ zKAS node (127.0.0.1:16810)  [templates, zKAS submits]
   (per-rig ports)            │
                              └────gRPC──▶ Kaspa node (127.0.0.1:16110)  [parent templates, KAS submits]
```

- Job = real Kaspa parent template whose coinbase `extra_data` embeds
  `"ZKMM" + H_fc` (H_fc = hash of the pending zKAS block).
- Every share is checked against BOTH targets: ≤ zKAS target → assemble
  AuxPow and submit zKAS block; ≤ Kaspa target → submit parent to Kaspa
  (and it is also a zKAS block: KAS target ⊂ zKAS target at current ratio).
- KAS leg rides the operator's production Kaspa node — identical network
  surface to solo mining (verified: parent bits ≡ RKStratum's d=1.73e16).

## 4. Workstreams

### WS1 — Merged mining port (from pool bridge)
Port per Port Map v2 components 1–9: `merged.rs` (AuxPoW mechanics,
MergedPending LRU), KaspaApi merged state + init, parent template plumbing
(fetch/TTL cache/semaphore/listener), `get_block_template` wrap hook,
`submit_block` aux-reassembly hook, dual-target + dual-settlement logic in
the share handler, prom counters, main wiring, full test suite including
`merged_derisk.rs` and the late-parent regression.

**Acceptance:** V2 target-decode gate passes (network_target ≙ live zKAS d,
independent of parent bits); all ported tests green; synthetic-parent mode
mines on a devnet/testnet; settlement path diff-matches the pool bridge's
observed behavior once V5 (first live settlement) is banked on the reference.

### WS2 — Notification-driven templates (kill the polling fallback)
The pool bridge shipped with the concrete-API wiring dropped (trait-object),
silently downgrading to polling where `block_wait_time` becomes job cadence.
Requirements: (a) stratum server receives the concrete API so node
`NewBlockTemplate` notifications drive zKAS jobs; (b) Kaspa parent listener
likewise notification-driven; (c) polling retained strictly as fallback with
a WARN on activation and a `bridge_template_mode` gauge; (d) each fresh
zKAS template triggers parent refresh + job push within 50ms internal budget,
subject to WS8 coalescing.

**Acceptance:** normal operation shows zero polling-mode warnings;
disconnect test shows fallback activate + recover with mode transitions
logged once each; job latency measured from node notification to stratum
notify ≤ 100ms p99 on the Kron K1.

### WS3 — Performance analytics
Prometheus per instance AND per chain. Series (minimum): shares
accepted/stale/invalid; blocks {zkas, kas_full_clear}; near-miss counter
(existing ratio semantics); parent fetches + failures; aux submissions +
rejections **by rejectDetail reason**; template age at job send (histogram,
per chain); template-source state; effective hashrate; per-chain luck%
(blocks found ÷ expected from share work). Grafana-ready; no new UI.
Existing stats table stays, gaining a per-chain Blocks split.

Warm-up MUST zero-init **all** per-worker series at startup, including the
merged counters (`ks_merged_parent_submit_total` class) — not just
`ks_blocks_mined`. (Field bug: the first double's alerts never fired
because `increase()` cannot see a brand-new series' first increment; the
PromQL birth-idiom workaround is deployed but the correct fix is here.)

**Acceptance:** a Grafana board can answer, without logs: "what is each
chain's luck%, stale%, and template freshness for any rig over any window"
— i.e., the A/B-vs-RKStratum comparison becomes a dashboard read. AND: a
fresh bridge start followed by a first-ever event on any series fires the
corresponding alert with plain `increase()` rules, no birth-idiom needed.

#### WS3-FT — Own-block fate tracker
The bridge subscribes to virtual-chain-changed notifications on BOTH nodes
and tracks every submitted block through its lifecycle:
`submitted → accepted-by-node → blue (paid) / red (merged, unpaid) /
evicted (reorged out)`. Emitted as `bridge_blocks_fate{chain,worker,fate}`
plus one INFO line per final verdict (hash, worker, job age at submit,
time-to-verdict). Red/evicted verdicts additionally fire a WS7 webhook —
a red block is page-worthy on a pipeline whose baseline is months of zero.

**Acceptance:** (a) cutover certification — during V7 the tracker
independently confirms the KAS-leg zero-red baseline is preserved under
the new bridge, per-block rather than statistically; (b) zKAS visibility —
every zKAS find gets a definitive chain-kept-it verdict, distinct from
node-accepted-it (the wedge-era failure mode where those diverge);
(c) latency-regime early warning — if either chain re-enters the
low-difficulty racing regime, red-rate is the leading indicator and
surfaces same-day.

### WS4 — Graceful degradation + log hygiene
**OPERATOR REQUIREMENT (2026-08-05, elevates WS4 into the initial build
alongside WS2):** either node may be shut down at any time and the bridge
MUST NOT fail — it elegantly continues mining whichever chain's node is
alive, recovering automatically. Formalized as a four-state mode machine:

- **MERGED** (both nodes healthy): Kaspa parents committing to H_fc;
  dual-target, dual-settlement.
- **ZKAS-ONLY** (Kaspa node down): local synthetic parents; zKAS
  submissions continue; KAS-clearing shares counted but unsubmittable.
- **KAS-ONLY** (zKAS node down): PLAIN Kaspa parents (no ZKMM commit) —
  the bridge temporarily behaves as vanilla RKStratum; KAS submissions
  continue paying the configured kaspa address; zKAS earning paused.
- **ISLANDED** (both down): stratum listeners and rig connections stay
  alive; no work or last-good work served; rig-level pool-#2 failover
  covers extended outages (RKStratum during transition; post-cutover,
  the operator-configured failover — recommended second instance of this
  bridge); bridge never exits and recovers to the best available state.

Transitions are driven by per-client health state exposed by the WS2
NotificationHub relay (built once, shared); each transition emits ONE
INFO line and updates a `bridge_mode` gauge; recovery is automatic; rigs
are never disconnected by a mode change. Plain-parent mode ships
default-ON with a config off-switch (`degradation.plain_parent: true`)
as the V6 escape hatch. This generalizes invariant 6 into full bilateral
node-failure independence.

Original incident-derived requirements retained: (a) on zKAS template
refusal (e.g. transitional IBD), serve last-good template up to
`stale_template_grace` (default 5s) before entering KAS-ONLY; log ONCE
(rate-limited summary "template source degraded: N failures over Ts").
(b) Kaspa-source loss → zKAS-aux-only mode (existing behavior), same
logging discipline. (c) No per-client retry cascades in logs; per-client
errors collapse into the summary.

**Acceptance:** simulated 60s outage of each node (V6 drill, both
directions) shows: single degrade + single recover log line per event,
rigs never disconnected, correct mode metrics, and — for zKAS-node loss —
KAS blocks still submittable during the window.

### WS5 — Configuration done right
**PAYOUT MODEL — DECIDED (a), 2026-08-05, operator-set:** per-worker
**kaspa:** addresses via stratum authorize (unmodified RKStratum
semantics: username = kaspa address + worker suffix), plus ONE global
zKAS treasury address in yaml (`merged.zkas_pay_address`), validated at
startup (zkas: prefix + checksum; kaspa-addresses knows both prefixes
since 7372571). Consequences: (1) the authorize/wallet path is literally
untouched RKStratum — clean_wallet's kaspa-only regex is now CORRECT
as-is, dissolving the bug-#7 zkas-username fallback question entirely;
(2) rig cutover = port change only — rigs keep the SAME kaspa-address
username they use on RKStratum today, so during migration pool #1 (new
bridge) and pool #2 (RKStratum, transitional) share credentials and the
authorize-once stale-wallet trap cannot recur; post-cutover, pool #2
targets whatever failover the operator configures (recommended: second
instance of this bridge) with the same shared-credential property; (3) per-worker zKAS attribution
remains via worker-name metric labels while all zKAS pays the single
treasury, matching actual operating practice; (4) WS5(e) shrinks to a
yaml-field checksum validation test.
(a) Yaml fields for merged mining that are actually read:
`merged.enabled`, `merged.kaspa_node`, `merged.kaspa_pay_address`,
`merged.coinbase_tag`; env vars (`ZKAS_MERGED_MINING` etc.) override yaml
for service-definition compatibility. (b) Startup validation, fail-fast:
address prefix/checksum, node reachability probes (warn + degrade, not
crash), stratum/prom/dashboard port-collision detection before bind.
(c) The two ENABLED lines remain the operator contract, plus one line
echoing effective config source (yaml vs env) per setting.
(d) Ship the operator's proven defaults: per-rig fixed-diff instances,
`block_wait_time: 200` (fallback-only semantics per WS2), vardiff off.
(e) **`zkas:` addresses are first-class:** stratum-username parsing and
wallet validation explicitly accept and checksum-verify the `zkas:` prefix
— no fallback path, no `POOL_FALLBACK_ADDRESS` semantics. (Reference
bridge's `clean_wallet()`/`WALLET_REGEX` matches only `kaspa:`; zkas
usernames succeed via an untraced path — works, but incidental.)
(f) **Authorize-once wallet capture is documented contract:** the payout
address is captured at stratum authorize and a config-page save on
IceRiver does NOT re-authorize — the bridge logs the captured address per
worker at authorize so a stale wallet is visible in one log line. (Field
cost: 2 blocks paid to an old wallet.)

**UPSTREAM STATUS (verified against repos 2026-08-05):** the "wire merged
mining YAML into runtime" work previously cited as zkas-rusty d79bf68 is
REAL but lives in firecash/solo-dual-mode as **c2cd7e1** (hash in prior
notes was wrong; description was right): a `new_with_merged()` constructor
reading `configured_kaspa_node`/`configured_kaspa_pay` from config.yaml,
env vars retained as precedence overrides. WS5(a) therefore ports from
their implementation rather than being designed fresh. Their vendored
bridge-src is otherwise byte-identical to the soaked pool bridge (7 of 9
core files zero-diff, incl. merged.rs/share_handler.rs), pins
kaspa-consensus-core to e3589f7 (verified post-ZKMM-rename), and fixes
the misleading "Kaspa is not synced" log to name the actual ZKas endpoint
— take their kaspaapi.rs/main.rs as the WS1 port source. zkas-rusty
upstream separately: 80e8e2b/e5bcf55 was packaging moving to the
solo-dual-mode repo; 7372571 added the `kaspa` mainnet prefix to
kaspa-addresses (shrinks (e) to a validation test).

**Acceptance:** the day-one failure classes are unreachable: dead yaml
fields gone; typo'd env → loud fail; port collision → named error, not
bind panic; aux-only silent fallback impossible without a WARN; a zkas:
username with a bad checksum is rejected at authorize with a named error.

### WS6 — Submission forensics (rejectDetail)
Both chains' submit paths log and count the v1.0.5 `rejectDetail` (zKAS)
and Kaspa's rejection reasons verbatim. A rejected block is a page-worthy
event: one ERROR line with chain, worker, hash, reason, template age.

**Acceptance:** deliberately corrupted submission in test surfaces the
node's reason string in the bridge log and increments the reason-labeled
counter.

### WS7 — Block-found notifications
Generic webhook (`notifications.webhook_url`, JSON POST): events
`block_found` {chain, worker, hash, full_clear, height/daa, explorer_url}
and `mode_change` {degraded/recovered states}. Discord-compatible payload
option for KDSM-style pings. Fire-and-forget with retry ×3; never blocks
the share path.

**Acceptance:** live block produces a Discord ping ≤ 5s after settlement;
webhook endpoint down → zero impact on mining, one rate-limited WARN.

### WS8 — Job pacing & rig compatibility (carry-over hardening)
Preserve the pool bridge's proven pieces as explicit spec: per-miner-profile
notify coalescing (legacy ASIC 1000ms / common 500ms floors), IceRiver
pre-authorize extranonce handshake, `pow2_clamp`, extranonce_size 2,
anti-abuse limits with ASIC-sane floors. New: coalescing floor configurable
per instance (`notify_min_interval_ms`) for future firmware.

**Acceptance:** all seven current rigs (KS0 Ultra ×4, KS7 Lite ×3)
connect, hash, and hold SPM on profile for 24h with stale% ≤ RKStratum
baseline (≤0.1% observed) on the KAS leg.

## 5. Invariants (consensus-critical — each gets a test)

1. **Verbatim coinbase:** the zKAS template's coinbase bytes flow untouched
   into H_fc and the assembled aux block. (Dev-fee accrual post-NU1 changes
   coinbase content; verbatim handling makes the bridge agnostic to it.)
2. **Raw-header submission:** zKAS submits via the RpcRawBlock conversion
   that carries `aux_pow`; a unit test asserts aux_pow survives conversion.
3. **Block gate = merged_fc_target** (zKAS target), never parent bits;
   `pow_passed`/share validity remain distinct. (Field-verified pattern.)
4. **Late-parent ordering:** parent submit precedes zKAS claim; a parent
   solving after a failed/lost zKAS claim is still submitted to Kaspa.
   (Port the regression test unchanged.)
5. **One-shot claims:** an H_fc settles at most once (MergedPending
   semantics); duplicate parent nonces for one H_fc handled per reference.
6. **KAS-leg independence:** no zKAS-side state may gate Kaspa submission
   of a Kaspa-target-clearing share (WS4 degradation included).
7. **Commitment magic:** the bridge's embedded commitment magic equals the
   node consensus crate's `MERGE_MINE_MAGIC` (`*b"ZKMM"`), asserted by a
   test that decodes `embed_commitment()` output against the consensus
   constant itself — never a string literal. (From the FCMM/ZKMM lockfile
   bug: a stale Cargo.lock pinned pre-rename crates and produced 100%
   deterministic aux rejection with a healthy-looking KAS leg masking it.
   In-workspace builds make this class structurally impossible — bridge
   and node compile the constant from the same crate — but the test stays
   as the tripwire for any future rename.)

## 6. Test & validation plan

**Unit/integration (CI):** ported suite + invariant tests + WS-specific
acceptance tests above; Linux preflight `cargo check` retained; Windows
build in-workspace (structurally immune to the dependency-cdylib LNK class).

**Live gates (from Port Map v2, procedures proven in the field):**
- V2 target-decode via `RUST_LOG=...share_handler=debug` — network_target
  ≙ explorer/node d for zKAS; drifts independently of parent bits.
- V3 near-miss frequency ≈ share_rate ÷ (kaspa_target_ratio/1024).
- V4 parent bits decode ≡ production Kaspa bridge `d=`.
- V5 first live settlement: settlement log cluster; zKAS block on explorer
  + wallet note maturing; Blocks column increments correctly (historical
  display-bug regression); for full_clear cases, Kaspa block paying
  configured address.
- V6 degradation drills both directions (per WS4).
- V7 A/B: ≥48h, merged bridge vs RKStratum baseline — KAS stale% and
  per-TH luck% within noise; then staged cutover. **End state (operator
  decision 2026-08-05): ONE bridge — ours. RKStratum runs only as the
  transitional baseline and failover during V7 and the confidence soak,
  then is retired; its proven code path persists as the new bridge's
  KAS-ONLY ground state.**

**Environment prerequisites (post-NU1 era, in effect):** the network is
past DAA 757,000; the production node is snapshot-bootstrapped (archival
mandatory — the snapshot came from an archival node) on official v1.0.5
binaries; pins are obsolete. Fresh-sync-from-genesis validation becomes
possible once the pruning point clears 757k. Peer topology constants are
operator-validated (Crescendo/Toccata sweeps on the target hardware):
KAS 42 out / 8 in, zKAS 16 out / 8 in; re-sweep outbound after full
merged-stack cutover, since the knee is load-dependent.

## 7. Build & release

Built in-workspace in the operator's zkas-rusty fork; add
`--bin stratum-bridge` to deploy.yaml's Windows job (+ the same zip
packaging pattern as the pool-fork workflow: sample yaml, setup txt,
launcher cmd). Launcher hygiene baked into the shipped .cmd: `cd /d %~dp0`
as the first line (prevents PATH picking up a foreign stratum-bridge.exe
when invoked from another cwd — field incident) and every env line
commented with the `set VAR=value` syntax requirement (a dropped `VAR=`
silently degrades to aux-only; fails safe but earns nothing). Fork stays
synced to firecash main; node binaries in production remain official
releases — the fork pipeline exists for the bridge and for building ahead
of upstream when needed.

## 8. Rollout

- **Phase A (now → network ready):** WS1 components 1,2,4,5 + WS2 + WS5.
  Compiles; synthetic mode + V2 pass on a local net.
- **Phase B:** WS1 components 3,6 (real dual-chain) + WS4. V3/V4 pass.
- **Phase C:** WS3, WS6, WS7, WS8 + full test suite.
- **Phase D:** V5–V7 on the live fleet; upstream PRs (bridge port,
  deploy.yaml, and the WS4/WS6 operability pieces as separable commits).

Gate discipline: Phase D's cutover requires V5 banked on the *reference*
pool bridge first, so the new bridge's settlement behavior has a live diff
target — not just source. **STATUS: V5 BANKED 2026-08-04** (20 zKAS + 12
merged-KAS blocks + confirmed double, day one of v0.3.3-win production).
Remaining unmet gates: V6, V7.

Operating pattern for remaining phases: the reference bridge and
RKStratum keep production earning during development; the v2 bridge
validates on the control-rig holdout pattern before A/B; RKStratum serves
as failover THROUGH the transition only. **After cutover the fleet
consolidates on the single new bridge** (reference bridge and RKStratum
both retired). Post-retirement failover recommendation, operator to
confirm: rigs' pool #2 points to a second instance of OUR bridge — a
KAS-only-configured process on offset ports — covering bridge-process
failure with the same binary; node failure is already covered in-process
by the WS4 mode machine.

## 9. Risks

| Risk | Mitigation |
|---|---|
| ~~Settlement path unexercised until V5~~ RETIRED: V5 banked 08-04 with on-model stats | Reference bridge is now the live per-block diff target |
| Upstream implements WS5 in parallel (d79bf68) | Re-diff before porting; adopt their schema, contribute validation on top |
| Post-NU1 consensus-crate drift vs bridge assumptions | In-workspace build tracks main; invariants 1 & 7 isolate coinbase/magic changes |
| WS4 plain-parent degradation is novel (not in reference) | Ship behind config flag, default on only after V6 passes both drills |
| Young-chain incidents recur mid-rollout | KAS leg independence (invariant 6) + RKStratum failover cap the blast radius |
| Upstream API churn in zkas-rusty bridge | Small PRs early (deploy.yaml, WS6) to establish the channel |

## 10. Effort

Port Map v2 estimate carried: **4–6 focused days** for WS1+WS2 core;
WS3–WS8 add ~2–3 days, largely parallelizable and individually shippable
(**6–9 days total**). With the reference bridge in production, there is no
chain-imposed deadline — the port proceeds at the pace production
operations allow, and WS5's scope should be re-priced after the d79bf68
re-diff.
