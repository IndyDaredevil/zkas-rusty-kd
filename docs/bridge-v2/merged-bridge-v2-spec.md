# Specification: Merged-Mining Stratum Bridge v2
### Target: zkas-rusty in-tree bridge (v2.0.1 RKStratum lineage) · Draft 4 · 2026-08-06
### Draft 2 decisions: fate tracker KEPT (WS3-FT); WS4 plain-parent degradation KEPT as spec'd (flagged, default-on after V6)
### Draft 3 (post-V5 revision): V5 BANKED on reference bridge with day-one production stats; invariant 7 (magic bytes) added; WS3 warm-up fix; zkas-address handling first-class; WS5 upstream collision flagged; stale pre-757k framing scrubbed
### Draft 4 (post-build revision, 2026-08-05/06 session): WS1+WS2 BUILT and live; WS3 built beyond spec and out of order. **Invariants 3 and 4 CORRECTED — Draft 3's text contradicted the shipped, correct implementation.** Invariant 8 (accounting law) added. Windows "structural immunity" claim RETRACTED (bug #1 has three vectors). V2 banked via telemetry rather than debug logging. Attach lifecycle, commitment wire format, and display semantics promoted from commit messages into the spec. §8 records the rollout deviation (full-fleet cutover ahead of WS4/soak) as accepted risk rather than absorbing it silently. Stale difficulty constants replaced with relations.

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
multi-port ADR-0022), not a cross-version forward-port. The port's value is
lockfile-drift immunity (in-workspace build), reduced surface, WS2–WS8, and
upstreamability. Success = one bridge process that (a) matches or beats the
operator's tuned RKStratum on KAS solo capture, (b) earns zKAS at zero
marginal hashrate cost, (c) is operable: observable, degradation-tolerant,
and loud about the right things.

**Field results — day one of reference-bridge production (v0.3.3-win,
through 23:59 ET 2026-08-04):**
- **21 zKAS blocks** settled and paid (~1,130 ZKAS) in a ~9.1h earning
  window — expected ~15.8 at 11 TH/s vs ~22.9 PH/s network → **luck ≈ 133%**
  (hot; no luck figure is trusted before a 48h window).
- **15 merged-parent KAS blocks** paid (~36.7 KAS), spanning a longer window
  that includes pre-fix FCMM rejections — so the observed full-clear rate
  (15/21 ≈ 71%) is upper-biased; consistent with the 55–66% difficulty-ratio
  prediction at n=21 (±2 binomial). Re-derive on clean post-fix windows.
- First confirmed **double** (one solve, both chains paid) 17:07 ET.
- **Combined KAS day total: 40 blocks** — at the ~38/day expectation for
  14.2 TH/s on a cutover day full of restarts. The zKAS revenue line was
  added at measurably zero KAS cost: the zero-marginal-cost thesis,
  demonstrated at day-ledger granularity.
- KAS solo baseline unharmed throughout.

**Build results — v2 RC (2026-08-05/06 session).** WS1 (c.1–c.6) and WS2
complete; WS3 c.7–c.14 delivered. The RC ran live against both production
nodes, then the full fleet (7 rigs, 14.23 TH/s, instances :5755/:5765).
Three KAS blocks found by the RC (00:00:34 explorer-verified; 02:26:21;
03:30:14 accepted + blue in 2s) — **all KAS-solo**, as the merged env vars
were unset in those windows (see §7). WS2 verified live: node-push cadence
3.2 j/s single-rig, 22.4 j/s fleet — 10 BPS coalesced by the 250ms limiter,
unreachable by ticker polling. Merged mode verified live: `zk=ok`, 100% of
templates committed, per-leg near-miss tracking active. **No dual settled
yet** — V5-on-our-bridge remains open.

These figures are the quantitative bar for V7: same luck-per-TH on both
chains, same zero-red record, at equal or better stale%.

## 2. Scope

**In:** the eight workstreams below, applied to `zkas-rusty/bridge` in the
operator's fork, upstreamable as PRs.
**Out (v2):** multi-node template failover; any web dashboard beyond what the
bridge already ships; pool/payout accounting; non-Windows service tooling.

## 3. Architecture (contract; §3.1–3.3 are as-built additions)

**GOVERNING PRINCIPLE (2026-08-05, operator-set): KAS-primary inversion.**
The reference (zkas-pool lineage) bridge is zKAS-primary: its main client is
the zKAS node and Kaspa is the optional add-on. The port INVERTS this: the
primary client is the Kaspa node — the unmodified, production-proven
RKStratum path — and the zKAS commitment is the optional enhancement
(`zkas: Option<Arc<ZkasLeg>>`). Consequences: KAS-ONLY is the BASE state,
not a degraded one (the bridge with no zKAS node is simply RKStratum);
MERGED is the enhanced state; startup order is irrelevant; commitment attach
is NON-BLOCKING — a zKAS hiccup means the next job goes out plain rather
than late, never delaying KAS job cadence. Protecting KAS mining at all
costs is the primary reason the port targets the RKStratum lineage; this
principle is that reason expressed as architecture. It is also the most
upstreamable shape: to rusty-kaspa this is RKStratum plus optional AuxPoW
child-chain support — a strict superset, zero behavior change when
unconfigured.

```
                            ┌─gRPC─▶ Kaspa node :16110   PRIMARY
IceRiver ASICs ──stratum──▶ │        (parent templates, KAS submits, node-push
   (per-rig ports)          │         NewBlockTemplate → NotificationHub)
                            └─gRPC─▶ zKAS node :16810    OPTIONAL ENHANCEMENT
                                     (zKAS templates, aux submits; attached in
                                      background — absent = plain RKStratum)
```

- Job = real Kaspa parent template whose coinbase `extra_data` embeds the
  commitment (§3.2). The parent pays the **connecting worker's** kaspa
  address (payout model (a), WS5); only the zKAS side pays the treasury.
- When the job carries a commitment, every share is checked against BOTH
  targets: ≤ zKAS target → assemble AuxPow and submit zKAS block; ≤ Kaspa
  target → submit parent to Kaspa (and it is also a zKAS block, since
  KAS target ⊂ zKAS target at current ratio).
- KAS leg rides the operator's production Kaspa node — identical network
  surface to solo mining. **Verification is a relation, not a constant:**
  the RC's parent bits must decode to the same `d=` the production bridge
  reports concurrently (observed together at 1.52e16–1.56e16 on 08-06;
  difficulty moves, so never assert a literal).

### 3.1 Attach lifecycle (as built, c.3.5 — operator design review)

The zKAS leg lives in a mutable slot filled by a **background task that
retries forever** (capped 30s backoff, log-quieted to 1-in-10 after the
first three attempts). The constructor never awaits it. Therefore:

- The bridge boots and mines KAS with no zKAS node present, entering MERGED
  whenever the node appears — **startup order is irrelevant by
  construction, not by policy.**
- Accessors (`has_zkas`, `zkas_leg`, `zkas_hub`) read per-use and return
  cloned `Arc`s, so a caller's view is stable for its own duration even if
  the slot changes underneath.
- Observed live: `Merged mining ACTIVE after 1 connect attempt(s)` 7ms after
  boot; and a `zk=PLAIN → zk=ok` status transition on a later boot — the
  KAS-primary guarantee made legible in one status field.
- **Not yet built:** detach on loss-after-attach, and re-attach after
  detach. Both are WS4 scope; this slot is the shape WS4 extends rather
  than replaces.

### 3.2 Commitment wire format (consensus-adjacent; field-verified)

```
extra_data = coinbase_tag || MERGE_MINE_MAGIC || hex(H_fc) || suffix
             (MERGE_MINE_MAGIC = *b"ZKMM", consensus/core/src/auxpow.rs)
```

`H_fc` is embedded as **ASCII lowercase hex, 64 characters**
(`COMMITMENT_HEX_LEN`), *not* 32 raw bytes — because `extra_data` rides
`GetBlockTemplateRequest`'s protobuf **UTF-8 string** field. This is why the
FCMM-era forensics decoded rejected coinbases as readable text. An early
invariant test asserted raw bytes and was wrong; the corrected test
(`bridge/tests/invariants.rs`) asserts the magic appears exactly once,
followed immediately by the hex form, against the consensus constant itself.

### 3.3 Non-blocking template budget (as built, c.4)

`current_zkas_template()` is cache-first (`ZKAS_TEMPLATE_TTL` 500ms),
single-flight (`Semaphore(1)`), and hard-bounded (`ZKAS_FETCH_BUDGET`
250ms). **Any** miss — no leg, stale cache, gate race, timeout, RPC error —
yields a plain job. All instances and workers share one cache, so the zKAS
RPC rate stays ~1/s against a ~22/s job rate. The stash insert happens
**once per distinct template** (at fetch success), never per request:
inserting per request churned `MergedPending`'s 4096-entry ring down to a
~3-minute eviction window and could silently lose the zKAS leg of a dual
whose parent solved from slightly older work (found by self-critique
2026-08-06, fixed in `fa711c4`).

## 4. Workstreams

Status legend: **DONE** (built, tested, live) · **PARTIAL** · **TODO**.

### WS1 — Merged mining port — **DONE (c.1–c.6)**
Port per Port Map v2 components 1–9: `merged.rs` (AuxPoW mechanics,
MergedPending), KaspaApi merged state + init, parent template plumbing
(fetch/TTL cache/semaphore), `get_block_template` wrap, `submit_block`
aux-reassembly, dual-target + dual-settlement in the share handler, prom
counters, main wiring, tests including `merged_derisk.rs`.

**As built:** `merged.rs` + `merged_derisk.rs` ported byte-identical from
firecash/solo-dual-mode bridge-src (which carried three embedded unit tests
of its own, incl. one-shot claims and a merkle-branch property test). The
only dependency gap was `kaspa-merkle`. The template hook, dual gate, and
settlement were **re-derived rather than ported**, because the reference's
hooks are zKAS-primary and the inversion changes their shape. The helper
trio (`merged_fc_target`, `merged_chain_hash`, `claim_network_solution`)
ports flag-free — gating on the **job's** commitment rather than a static
`merged_mining` flag, which is what makes in-flight jobs behave correctly
across background attach/detach.

**Acceptance:** V2 gate passes; ported tests green; settlement diff-matches
the reference once V5 is banked there. **STATUS: V2 banked 08-06 (§6);
settlement diff pending a live dual on our bridge.**

### WS2 — Notification-driven templates — **DONE, live-verified**
The pool bridge shipped with the concrete-API wiring dropped (trait-object),
silently downgrading to polling where `block_wait_time` becomes job cadence.
Requirements: (a) stratum server receives the concrete API so node
`NewBlockTemplate` notifications drive jobs; (b) parent listener likewise;
(c) polling retained strictly as fallback; (d) fresh template → job push
within budget, subject to WS8 coalescing.

**As built:** a `NotificationHub` per **client** (never per scope — all
scopes multiplex one gRPC channel; two readers steal from each other)
demuxes by variant into per-scope broadcast channels, exposes a watch-based
`ClientHealth` (the WS4 mode-machine input), and treats `Lagged` as a
forced-resync edge rather than silent loss. Both template listeners became
thin wrappers over one free, unit-testable `run_template_listener`. The
`is_first_instance` gate is **deleted** — all instances subscribe.
Deliberately removed: the vestigial per-iteration `wait_for_sync` preamble
and the no-op `restart_channel` flag (under fan-out they would have
multiplied redundant sync RPCs by instance count).

**Acceptance:** zero polling-mode warnings in normal operation; disconnect
test shows fallback activate + recover. **STATUS: PASSED.** Live evidence:
job counter 6→621 in 194s ≈ 3.2 j/s single-rig; 22.4 j/s at full fleet.
Nine-subscriber fan-out is an automated acceptance test.

### WS3 — Performance analytics — **PARTIAL (c.7–c.14 delivered)**
Prometheus per instance AND per chain. Series (minimum): shares
accepted/stale/invalid; blocks {zkas, kas_full_clear}; near-miss; parent
fetches + failures; aux submissions + rejections **by rejectDetail reason**;
template age at job send; template-source state; effective hashrate;
per-chain luck%. Grafana-ready; no new UI. Existing stats table gains a
per-chain Blocks split.

Warm-up MUST zero-init **all** per-worker series at startup, including the
merged counters — not just `ks_blocks_mined`. (Field bug: the first double's
alerts never fired because `increase()` cannot see a brand-new series' first
increment.)

**Delivered (c.7–c.14):** console K/Z/D counters and status-line telemetry;
prom export incl. `zk_state`/rpc/jobs/submit gauges; dashboard merged panel
and API fields; timestamped per-event gauges for zKAS and doubles; per-leg
near-miss **session-best** tracking; solve-based display accounting
(WS3-DA); forensic hex-dump instrumentation for implausible near-miss
ratios. **Still TODO:** template-age histogram, per-chain luck%,
rejectDetail-labeled rejection counters (needs WS6), warm-up zero-init,
Grafana board.

**Telemetry contract (Draft 4):** the production instrument is the status
line plus state-change INFO lines — **not** debug logging, which is
bring-up-only by operating rule. Status fields and their meaning:
`j=` jobs/sec · `rpc k=/z=` per-chain RPC latency (ms) · `sub=avg/max`
submit latency · `near k=/z=` per-leg session-best near-miss · `zk=` merged
state (`PLAIN` unconfigured/unattached, `ok`, `stale`) with template age and
the percentage of templates going out committed. Any future gate procedure
should be expressible in these fields; if it is not, the gap is in the
telemetry, not a reason to reach for debug.

**Near-miss design note (c.12):** a fixed percentage threshold is useless at
these magnitudes — at diff 4096 against d≈1.5e16, P(a share reaches even 1%
of network target) ≈ 2.7e-11. The instrument is therefore **session-best**
tracking (closest any share has come this process run), announced only on
genuine improvement. **OPEN ANOMALY:** console session-best and dashboard
readings have disagreed (5.41e-1% vs 1.22e+1% in one session), and observed
values near 19% are implausible under the same math that motivated the
design. c.14's forensics are armed; **V3 is not claimable until the
instrument agrees with itself.**

#### WS3-DA — Display / accounting semantics (NEW in Draft 4)
A **solve** is one winning nonce. A **double** is one solve that lands on
both chains as two blocks with **two different hashes** (parent hash on KAS;
`H_fc` on zKAS, because the aux proof rides outside the header hash).
Therefore:

- `solves = kas + zkas − doubles` (invariant 8).
- Counters and any tile/chart whose unit is "a find" count **solves**;
  chain-block counts are reported separately, never summed into a single
  "blocks" number.
- Recent-blocks tables render **one row per solve**, with a chain badge
  K | Z | D; a D row carries **both** labeled hashes, each linking to its
  own chain's explorer. A dual mid-confirmation may render as its first
  confirmed leg and upgrade to D on the next refresh — an honest transient,
  not an error.
- The per-event double feed publishes both hashes (`kasHash|zkasHash`);
  `ShareOutcome` is the rendezvous — each settlement arm deposits its hash
  at accept time so whichever finishes second emits the complete pair.

Rationale: the first implementation concatenated the K/Z/D feeds and
rendered a dual as **three rows** while the tile double-counted it. Display
correctness is not cosmetic — these numbers are the operator's only view of
settlement behavior, and a wrong one masks exactly the failures WS3 exists
to surface.

#### WS3-FT — Own-block fate tracker — **TODO**
The bridge subscribes to virtual-chain-changed notifications on BOTH nodes
and tracks every submitted block: `submitted → accepted-by-node → blue
(paid) / red (merged, unpaid) / evicted (reorged out)`. Emitted as
`bridge_blocks_fate{chain,worker,fate}` plus one INFO line per final
verdict. Red/evicted verdicts fire a WS7 webhook — a red block is
page-worthy on a pipeline whose baseline is months of zero. The
NotificationHub already supports the required `VirtualChainChanged` scope
with no modification.

**Acceptance:** (a) cutover certification — during V7 the tracker
independently confirms the KAS-leg zero-red baseline per-block rather than
statistically; (b) zKAS visibility — every find gets a definitive
chain-kept-it verdict, distinct from node-accepted-it; (c) latency-regime
early warning via red-rate.

### WS4 — Graceful degradation + log hygiene — **PARTIAL / RE-SCOPED**
**OPERATOR REQUIREMENT (2026-08-05):** either node may be shut down at any
time and the bridge MUST NOT fail — it continues mining whichever chain's
node is alive, recovering automatically. Four-state mode machine:

- **MERGED** (both healthy): committing parents, dual-target, dual-settlement.
- **ZKAS-ONLY** (Kaspa node down): local synthetic parents; zKAS submissions
  continue; KAS-clearing shares counted but unsubmittable.
- **KAS-ONLY** (zKAS node down): PLAIN Kaspa parents — vanilla RKStratum
  behavior; KAS submissions continue paying the worker's address.
- **ISLANDED** (both down): listeners and rig connections stay alive; bridge
  never exits and recovers to the best available state.

**Already satisfied by the as-built design (§3.1/3.3):** KAS-ONLY as the
ground state; automatic entry into MERGED whenever the zKAS node appears;
plain-parent service on every zKAS miss, per job, with no flag; and the
observability half — `zk=PLAIN|ok|stale` is already on the status line.

**Still owed by WS4:** (a) **detach on loss-after-attach** — today a zKAS
node that dies mid-session leaves the leg attached and every template
quietly misses its budget; correct behavior is an explicit transition, one
log line, and a resumed background re-attach; (b) **ZKAS-ONLY** — synthetic
parents when the Kaspa node is down; (c) **ISLANDED** as an explicit state
with its own test; (d) a `bridge_mode` gauge and one-line-per-transition
discipline; (e) the V6 drills. Plain-parent mode ships default-ON with a
config off-switch (`degradation.plain_parent: true`).

Original incident-derived requirements retained: (a) on zKAS template
refusal, serve last-good template up to `stale_template_grace` (default 5s)
before entering KAS-ONLY; log ONCE (rate-limited summary). (b) Kaspa-source
loss → zKAS-aux-only mode, same discipline. (c) No per-client retry cascades
in logs.

**Acceptance:** simulated 60s outage of each node (V6 drill, both
directions) shows single degrade + single recover log line per event, rigs
never disconnected, correct mode metrics, and — for zKAS-node loss — KAS
blocks still submittable during the window.

### WS5 — Configuration done right — **PARTIAL (env path shipped; yaml owed)**
**PAYOUT MODEL — DECIDED (a), 2026-08-05, operator-set:** per-worker
**kaspa:** addresses via stratum authorize (unmodified RKStratum semantics),
plus ONE global zKAS treasury address, validated at startup. Consequences:
(1) the authorize/wallet path is literally untouched RKStratum —
`clean_wallet`'s kaspa-only regex is now CORRECT as-is, dissolving the
bug-#7 zkas-username question; (2) rig cutover = port change only, so
pool #1 and pool #2 share credentials and the authorize-once stale-wallet
trap cannot recur; (3) per-worker zKAS attribution rides worker-name labels
while all zKAS pays the single treasury; (4) WS5(e) shrinks to a
checksum-validation test.

**Semantic note (Draft 4):** the rig's username string is **load-bearing on
KAS** (it is the coinbase payout address the node pays) and an **identity
label on zKAS** (attribution only; the treasury is paid from config). One
string, two roles — worth stating because it explains why a stale wallet
cost real KAS but would be cosmetic on a zKAS-only bridge.

**As built (c.6):** env-var path only — `ZKAS_MERGED_NODE` and
`ZKAS_TREASURY_ADDRESS`, **both required**, whitespace-trimmed; one alone
warns loudly and runs plain; neither set = byte-for-byte RKStratum. The two
`MERGED MINING ENABLED` lines are the operator contract and the one-glance
check that a restart kept merged mode.

**Still owed:** (a) yaml fields (`merged.enabled`, `merged.zkas_node`,
`merged.zkas_pay_address`, `merged.coinbase_tag`) with env as override,
ported from solo-dual-mode's `new_with_merged` schema; (b) startup
validation: address prefix/checksum, node reachability probes (warn +
degrade, not crash), **port-collision detection before bind**, and
`deny_unknown_fields`; (c) a line echoing effective config source (yaml vs
env) per setting; (d) ship the operator's proven defaults; (e) `zkas:`
addresses first-class in username parsing/validation; (f) authorize-time
logging of the captured payout address per worker.

**Config facts that must survive into the yaml work (field-earned):**
- The Kaspa node's **gRPC is :16110**; **:17110 is wRPC-Borsh** (what legacy
  RKStratum used — the remembered "efficiency": Borsh over protobuf, and why
  it paired with `tcp_no_delay`). This bridge speaks gRPC. Symptom key:
  wrong-protocol port = accept-then-abort (BrokenPipe/ConnectionAborted);
  dead port = ConnectionRefused.
- `tcp_no_delay` is **not** in the v2 schema and is **silently ignored** if
  present (no `deny_unknown_fields`) — tonic enables TCP_NODELAY by default.
  Silently-inert config is worse than a rejected key; hence (b).
- An empty-string port disables that listener entirely — this is what lets
  three bridges coexist on one host during A/B.

**UPSTREAM STATUS (verified 2026-08-05):** the "wire merged mining YAML"
work lives in firecash/solo-dual-mode as **c2cd7e1** — a `new_with_merged()`
constructor reading config with env overrides. WS5(a) ports from their
implementation. Their vendored bridge-src is otherwise byte-identical to the
soaked pool bridge (7 of 9 core files zero-diff). zkas-rusty upstream
separately: 7372571 added the `kaspa` mainnet prefix to kaspa-addresses;
**our `d0232e1` is its missing twin** — the network-*name* parser still
rejected `kaspa-mainnet`, which no one hit until a KAS-primary bridge
pointed at a real Kaspa node. Both `cb632f7` (legacy address fixtures) and
`d0232e1` are upstreamable as-is and fix upstream's own CI.

**Acceptance:** the day-one failure classes are unreachable: dead yaml fields
gone; typo'd env → loud fail; port collision → named error, not bind panic;
aux-only silent fallback impossible without a WARN; a zkas: username with a
bad checksum rejected at authorize with a named error.

### WS6 — Submission forensics (rejectDetail) — **TODO**
Both chains' submit paths log and count the v1.0.5 `rejectDetail` (zKAS) and
Kaspa's rejection reasons verbatim. A rejected block is page-worthy: one
ERROR line with chain, worker, hash, reason, template age.

**Acceptance:** deliberately corrupted submission in test surfaces the node's
reason string in the bridge log and increments the reason-labeled counter.

### WS7 — Block-found notifications — **TODO**
Generic webhook (`notifications.webhook_url`, JSON POST): events
`block_found` {chain, worker, hash, full_clear, height/daa, explorer_url}
and `mode_change` {degraded/recovered states}. Discord-compatible payload
option for KDSM-style pings. Fire-and-forget with retry ×3; never blocks the
share path. **Draft 4:** `block_found` for a double must carry BOTH hashes
per WS3-DA, not one plus a lookup.

**Acceptance:** live block produces a Discord ping ≤ 5s after settlement;
webhook endpoint down → zero impact on mining, one rate-limited WARN.

### WS8 — Job pacing & rig compatibility — **PARTIAL (inherited, unmodified)**
Preserve the pool bridge's proven pieces as explicit spec: per-miner-profile
notify coalescing (legacy ASIC 1000ms / common 500ms floors), IceRiver
pre-authorize extranonce handshake, `pow2_clamp`, extranonce_size 2,
anti-abuse limits with ASIC-sane floors. New: coalescing floor configurable
per instance (`notify_min_interval_ms`).

**Draft 4 note:** the 250ms per-client limiter is now load-bearing in a way
Draft 3 did not anticipate — it is what converts 10 BPS node-push into the
observed 22.4 j/s job rate. Any change to it is a job-cadence change and
must be measured as one.

**Acceptance:** all seven rigs connect, hash, and hold SPM on profile for 24h
with stale% ≤ RKStratum baseline (≤0.1% observed) on the KAS leg. **Partial
evidence:** 255 accepted / 0 stale / 0 invalid across all seven rigs in the
first four minutes of full-fleet operation; 24h read pending.

## 5. Invariants (consensus-critical — each gets a test)

1. **Verbatim coinbase:** the zKAS template's coinbase bytes flow untouched
   into H_fc and the assembled aux block. (Dev-fee accrual post-NU1 changes
   coinbase content; verbatim handling makes the bridge agnostic to it.)
2. **Raw-header submission:** zKAS submits via the RpcRawBlock conversion
   that carries `aux_pow`; a unit test asserts aux_pow survives conversion.
3. **Dual gate, per job** *(CORRECTED in Draft 4)*: a share clears the block
   gate if it meets **either** chain's target — `meets_network_target ||
   clears_zkas` — where `clears_zkas` is evaluated against `merged_fc_target`
   **only when the job carries a commitment**. A plain job gates on parent
   bits alone and is byte-identically single-chain RKStratum. The zKAS target
   is never omitted when a commitment exists. `pow_passed` / share validity
   remain distinct from the block gate. *(Draft 3 read "never parent bits",
   true only in the zKAS-primary reference where every job carried a
   commitment; taken literally under the inversion it would break the KAS
   leg.)*
4. **KAS-leg submission independence** *(CORRECTED in Draft 4)*: Kaspa
   submission of a Kaspa-target-clearing share is never gated by, ordered
   after, or conditioned on the zKAS claim's outcome. A duplicate `H_fc`
   whose claim fails is still a distinct, reward-bearing Kaspa candidate and
   MUST be submitted. Conversely a zKAS-only clear must never reach the Kaspa
   node (it would be low-PoW rejected and miscounted). *(Draft 3 stated this
   as "parent submit precedes zKAS claim." As built the zKAS arm spawns first
   and the two are independent; the ordering was incidental to the
   reference's structure — the independence is the actual property.)*
5. **One-shot claims:** an H_fc settles at most once (MergedPending
   semantics); duplicate parent nonces for one H_fc handled per reference.
   Corollary (Draft 4): the stash is inserted once per distinct template,
   never per template request — see §3.3.
6. **KAS-leg independence (state):** no zKAS-side state — attach status,
   template freshness, hub health — may gate Kaspa job service or submission
   (WS4 degradation included).
7. **Commitment magic:** the bridge's embedded commitment magic equals the
   node consensus crate's `MERGE_MINE_MAGIC` (`*b"ZKMM"`), asserted by a test
   that decodes `embed_commitment()` output against the consensus constant
   itself — never a string literal. In-workspace builds make the
   **lockfile-drift** form of this bug structurally impossible (bridge and
   node compile the constant from the same file), but the test stays as the
   tripwire for any future rename — and the **format** half (§3.2) is not
   covered by that structural argument: the first version of this test
   asserted raw bytes and was wrong.
8. **Accounting law** *(NEW in Draft 4)*: `solves = kas + zkas − doubles`.
   One solve is one winning nonce; a double is one solve with two chain
   hashes. No counter, table, export, or webhook may represent a double as
   two solves, nor render it with fewer than both hashes. (Violated by the
   first display implementation; see WS3-DA.)

## 6. Test & validation plan

**Unit/integration (CI):** ported suite + invariant tests + WS acceptance
tests; Linux preflight `cargo check`; **Windows `cargo test` in-workspace,
which is now genuinely runnable** — see §7. *(Draft 3 claimed in-workspace
builds were "structurally immune to the dependency-cdylib LNK class."
**RETRACTED:** that immunity covers the lockfile-drift class only. The
cdylib/risc0 link class lives inside the workspace and required three
separate fixes.)*

**Live gates:**
- **V2 — target decode. BANKED 2026-08-06.** *Instrument changed:* Draft 3
  specified `RUST_LOG=...share_handler=debug`; WS3 telemetry supersedes it
  and debug logging is bring-up-only by operating rule. The banked evidence
  is the status line — `zk=ok <age> 100%` (100% of templates committed)
  together with per-leg near-miss readings, which cannot be produced unless
  `merged_fc_target` resolves per share against a live zKAS target while the
  parent gates on Kaspa bits.
- **V3 — near-miss frequency.** OPEN. The naive prediction
  (share_rate ÷ (kaspa_target_ratio/1024)) must be restated against c.12's
  session-best semantics, and the gate is blocked on the ratio anomaly
  (§4/WS3) until the instrument agrees with itself.
- **V4 — parent bits ≡ production bridge's concurrent `d=`.** Assert the
  relation between the two bridges read at the same moment; never a literal.
- **V5 — first live settlement on OUR bridge:** settlement log cluster; zKAS
  block on explorer + wallet note maturing; **the Blocks column and dashboard
  render it as ONE solve with two hashes** (invariant 8, and the historical
  display-bug regression); for full_clear cases, Kaspa block paying the
  worker's address. Banked on the *reference* bridge 08-04; **not yet on
  ours** — the RC's three KAS blocks were all KAS-solo.
- **V6 — degradation drills both directions.** Blocked on WS4 detach and
  ZKAS-ONLY.
- **V7 — A/B, ≥48h**, merged bridge vs RKStratum baseline: KAS stale% and
  per-TH luck% within noise; then staged cutover. **End state: ONE bridge —
  ours.** RKStratum runs as transitional baseline and failover through V7 and
  the confidence soak, then retires; its proven code path persists as the new
  bridge's KAS-ONLY ground state.

**Environment prerequisites (post-NU1 era, in effect):** the network is past
DAA 757,000; the production node is snapshot-bootstrapped (archival
mandatory) on official v1.0.5 binaries; pins are obsolete. Peer topology
constants are operator-validated: KAS 42 out / 8 in, zKAS 16 out / 8 in;
re-sweep outbound after full merged-stack cutover, since the knee is
load-dependent.

## 7. Build & release

Built in-workspace in the operator's zkas-rusty fork; add
`--bin stratum-bridge` to deploy.yaml's Windows job (+ the zip packaging
pattern from the pool-fork workflow: sample yaml, setup txt, launcher cmd).
**Workflow files are edited through the web UI** — the project's fine-grained
PAT deliberately lacks Workflows scope. CI gate for `main` is the **`bridge`
job** inside `bridge-check.yaml` (a required status-check context is a JOB
name, not a workflow name); the full-workspace `ci.yaml` carries an inherited
upstream noise floor (Lints, Check no_std) that must not be made a gate.

**Windows link class (bug ledger #1) — THREE vectors, all closed:**
1. `kaspad → kaspa-wrpc-server` declares a `cdylib` crate-type; cargo builds
   the DLL even as a transitive dep, and risc0's guest-only
   `sys_alloc_aligned` cannot resolve on the host. Fixed by gating the
   `kaspad` dependency `cfg(not(windows))`, stubbing the in-process-node
   module on Windows, and gating the integration tests that embed a node.
2. `kaspa-consensus-core → kaspa-shielded-core → risc0` — **ungateable**, the
   shielded proof system is load-bearing. Fixed with a `cfg(windows)`
   `no_mangle` host stub for `sys_alloc_aligned` in the bridge lib
   (unreachable in practice; aborts loudly if that ever changes).
3. **Example binaries** do not automatically link the parent lib, so they
   never receive the stub. Fixed with `use kaspa_stratum_bridge as _;`.

Release builds always survived via LTO dead-stripping — exactly why the
`-win` tags built while `cargo test` on Windows never could. **Any new
artifact kind (example, bench, extra bin) must be checked for vector 3.**

**Runtime requirements the launcher must satisfy:**
- `--node-mode external` is **required** (the default `Inprocess` hits the
  Windows stub and exits). Either flip the default when merged config is
  present, or keep it baked into the launcher.
- The merged env vars are **per-PowerShell-window** and were lost three times
  across restarts during the build session — each loss silently produced a
  KAS-only window and cost zKAS earnings. The shipped launcher
  (`start-rc-bridge.cmd`) bakes `ZKAS_MERGED_NODE`, `ZKAS_TREASURY_ADDRESS`,
  `RUST_LOG`, the config path, and `--node-mode external`. **Restarting by
  any other means is a spec violation**; the two ENABLED banner lines are the
  one-glance check.
- Launcher hygiene: `cd /d %~dp0` as the first line (prevents PATH picking up
  a foreign `stratum-bridge.exe` — field incident); every env line commented
  with the `set VAR=value` syntax requirement.

## 8. Rollout

- **Phase A:** WS1 core + WS2 + WS5. Compiles; V2 passes. **DONE.**
- **Phase B:** real dual-chain + WS4. V3/V4 pass. **PARTIAL** — dual-chain
  built and live; WS4 outstanding; V3 blocked; V4 pending.
- **Phase C:** WS3, WS6, WS7, WS8 + full test suite. **PARTIAL** — WS3
  substantially delivered; WS6/WS7 outstanding.
- **Phase D:** V5–V7 on the live fleet; upstream PRs. **NOT STARTED.**

Gate discipline: Phase D's cutover requires V5 banked on the *reference* pool
bridge first, so the new bridge's settlement behavior has a live diff target.
**STATUS: V5 BANKED 2026-08-04** on the reference. Remaining unmet gates: V3
(blocked), V4, V5-on-ours, V6 (blocked on WS4), V7.

**DEVIATION RECORDED (2026-08-06, operator decision).** Draft 3 required WS4
and a soak before fleet cutover. In practice the full fleet (7 rigs, 14.23
TH/s) was cut over to the RC within hours of the code being written, ahead of
WS4, V6, and any soak. Accepted rationale: the KAS leg is invariant-protected
and byte-identically RKStratum in the plain path; every rig retains pool-#2
failover to RKStratum; and the RC had run clean (three blocks, zero stale,
zero invalid). **Residual risk carried knowingly:** a zKAS-node death
mid-session has no detach path (WS4(a)) and would degrade silently to plain
jobs — earning KAS normally, earning no zKAS, with only the `zk=` status
field to reveal it. This is the strongest argument for prioritizing WS4(a)
next.

Operating pattern for remaining phases: RKStratum keeps failover duty; the
reference bridge remains the settlement diff target until V5 is banked on
ours. **After cutover the fleet consolidates on the single new bridge.**
Post-retirement failover: rigs' pool #2 points to a second instance of OUR
bridge — a KAS-only-configured process on offset ports — covering
bridge-process failure with the same binary; node failure is covered
in-process by the WS4 mode machine.

## 9. Risks

| Risk | Mitigation |
|---|---|
| ~~Settlement path unexercised until V5~~ RETIRED: V5 banked 08-04 on the reference | Reference bridge is the live per-block diff target |
| **Silent zKAS earning loss** — leg attached but node dead; no detach today | WS4(a) is the next priority; `zk=` status field is the only current tell; treasury reconciliation catches it after the fact |
| **Fleet cut over ahead of WS4/soak** (§8 deviation) | Invariant-protected KAS leg + pool-#2 failover + clean RC record; accepted knowingly |
| Near-miss ratio anomaly undermines V3 | c.14 forensics armed; V3 not claimed until the instrument agrees with itself |
| Post-NU1 consensus-crate drift vs bridge assumptions | In-workspace build tracks main; invariants 1 & 7 isolate coinbase/magic changes |
| New Windows artifact kinds reintroduce the LNK class | §7 vector-3 rule: every new example/bench/bin gets a Windows test build |
| Upstream implements WS5 in parallel | Adopt solo-dual-mode's schema (c2cd7e1); contribute validation on top |
| Upstream API churn in zkas-rusty bridge | Small PRs early (cb632f7, d0232e1) to establish the channel |

## 10. Effort

Draft 3 carried 6–9 focused days for WS1–WS8. **Actual: WS1 + WS2 + most of
WS3 landed in a single extended session (2026-08-05/06), roughly 20
commits** — against an estimate of 4–6 days for WS1+WS2 alone. Two caveats
against reading that as repeatable velocity: the port had an exceptionally
good source (a field-proven, byte-identical-core reference), and the session
ran with continuous operator review, which caught two design errors (bounded
vs forever attach; solve-based accounting) before they reached production.

**Remaining, re-priced:** WS4 detach / ZKAS-ONLY / ISLANDED + V6 drills ≈ 1
focused session; WS5 yaml + validation ≈ 1; WS6 + WS7 ≈ 1 together; WS3
remainder (histogram, luck%, warm-up zero-init, Grafana) ≈ 1; WS3-FT ≈ 1.
Gates V5-on-ours and V7 are elapsed time, not effort. With production earning
on both the reference bridge and the RC, there is no chain-imposed deadline —
the port proceeds at the pace production operations allow.
