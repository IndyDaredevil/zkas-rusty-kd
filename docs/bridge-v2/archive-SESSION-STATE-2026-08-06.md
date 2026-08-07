# SESSION STATE — 2026-08-06 05:30 ET (FINAL, session close)
### Merged-mining bridge v2 · zkas-rusty-kd · Operator: Michael (IndyDaredevil)
### Supersedes SESSION-STATE-2026-08-05.md + addendum. Read this one first.

---

## 0. ONE-PARAGRAPH STATE

The v2.1 release candidate is **live on the full fleet** (7 rigs, 14.23 TH/s, both
instances :5755/:5765) mining KAS with merged zKAS armed (`zk=ok … 100%`). Three
KAS blocks found during the session (all KAS-solo; merged env was unset in those
windows). WS1 (merged port) and WS2 (notification hub) are COMPLETE and merged to
`main` or sitting on `merged-ws1-port` under PR #3. WS3 (observability) is
substantially built. No production incident occurred at any point; RKStratum
remained the failover target throughout.

---

## 1. WHERE EVERYTHING IS

### Branch / PR state at close
- `main` @ **8b19f69** + PR #2 merge — contains NotificationHub (WS2), address-fixture
  fix, clippy fixes, Windows test unblocking.
- `merged-ws1-port` @ **fa272e1** — 22 commits ahead; PR #3 OPEN (ready-for-review,
  title "Release candidate v2.1: full merged-mining pipeline (WS1 c.1–c.6, KAS-primary)").
- PR #1 MERGED (NotificationHub). PR #2 MERGED (WS2 complete + Windows).
- Branch protection: `main` requires PR + status check context **`bridge`** (the JOB
  name inside bridge-check.yaml — NOT "Tests", NOT "bridge-check").

### Commit ledger on merged-ws1-port (oldest→newest, session work)
```
c0310fc/74d1111  WS1 c.1  merged.rs + merged_derisk.rs verbatim from solo-dual-mode
6f8beba          lock      regenerate Cargo.lock for kaspa-merkle dep (local, not hand-patched)
b5c3898/ceded67  WS1 c.2  invariants.rs magic tripwire (+ ASCII-hex format correction)
e79fa38          WS1 c.3  KAS-primary plumbing: zkas_client/hub/pay_address as Option
f09ad43          WS1 c.3.5 background attach, retry forever (OPERATOR DESIGN REVIEW)
35cf123          WS1 c.4  template decoration via extra_data + MergedPending home
1f9bf99          WS1 c.5  dual-target share gate + dual settlement
62c93ae          WS1 c.6  env wiring ZKAS_MERGED_NODE + ZKAS_TREASURY_ADDRESS
d0232e1          fix      consensus network.rs accepts 'kaspa-' prefix (UPSTREAMABLE)
e3b4072          obs      V2-gate debug lines (later superseded/removed by c.12)
3d81b1c          WS3 c.7  console observability: K/Z/D counters, zk state, j/s, latencies
01261da          WS3 c.8  prom metrics export
e96ca5d          WS3 c.9  dashboard merged panel + API fields
170ce6f          WS3 c.10 timestamped per-event gauges (Z + doubles)
5bc379b          WS3 c.12 near-miss telemetry both legs (c.11 epoch fix folded in here)
ce6ebdb          WS3 c.13 dashboard solve-based accounting (dual = 1 row, 2 hashes)
cfba811          fix      remove committed rebase-conflict markers
63f4667          WS3 c.14 forensic hex-dump for implausible near-miss ratios
fa711c4          fix      MergedPending eviction-window bug + escape dual-hash cells
60b9685          fix      doubles carry both hashes E2E + launcher .cmd + .gitignore
dfedfc6          fix      missed ShareOutcome test call site
243ef14/fa272e1  fix      example lib-link for risc0 stub (bug #1 THIRD vector)
```
NOTE: **c.11's label was lost** during a rebase (folded into c.12); its content
(zk_age_secs epoch fix + test) is verified present in merged_obs.rs.

### Local files on Michael's machine
- Repo: `C:\Users\inmyh\zkas-rusty-kd` (cargo 1.95, the ONLY machine that can compile).
- Launchers (repo root, both work): `start-rc-bridge.cmd` (tracked, from this session)
  and `run-rc-merged.cmd` (untracked, Michael's). Both bake env + `--node-mode external`.
- Config: `rc-v2-smoke.yaml` (untracked) — evolved to 2 instances (:5755 diff 1024,
  :5765 diff 4096), prom/dashboard/health disabled, `kaspad_address: 127.0.0.1:16110`.
- Untracked residue now gitignored: `*.bundle`, `notes-before.json`, `notes-after.json`.

---

## 2. ARCHITECTURE AS BUILT (the decisions that matter)

**KAS-primary inversion (governing principle).** The primary gRPC client IS the Kaspa
node — the unmodified RKStratum path. zKAS is an optional enhancement in a mutable slot
`Arc<RwLock<Option<Arc<ZkasLeg>>>>`. With env unset the binary is byte-for-byte
RKStratum. Consequences: rig cutover is a port change only; per-worker kaspa addresses
keep working unchanged (payout model (a)); bug #7's stale-wallet class is structurally
impossible.

**Background attach, retry forever** (operator design review, upgraded from bounded):
constructor NEVER waits on zKAS; capped 30s backoff; log-quieted after 3 attempts
(then 1-in-10). Boot with no zkas node → mines KAS immediately → MERGED activates
whenever the node appears. Startup order irrelevant BY CONSTRUCTION. Observed live:
`Merged mining ACTIVE after 1 connect attempt(s)` 7ms after boot; also observed the
`zk=PLAIN → zk=ok` transition on a later boot, which is this guarantee made visible.

**Non-blocking commitment.** `current_zkas_template()`: cache-first (TTL 500ms),
single-flight `Semaphore(1)`, hard 250ms budget. ANY miss (no leg, stale, gate race,
timeout, error) ⇒ plain job. The job is never late because of the enhancement.

**Wire format (field-verified).** Commitment = `prefix || ZKMM || hex(H_fc) || suffix`,
ASCII lowercase hex 64 chars (`COMMITMENT_HEX_LEN`), because it rides
`GetBlockTemplateRequest.extra_data`, a protobuf UTF-8 string. This is why FCMM
forensics decoded as readable text. An early test asserting raw bytes was WRONG.

**Dual gate + settlement.** Block gate = `meets_network_target || clears_zkas`
(`merged_fc_target` from the stashed committed block's bits). zKAS settlement spawns
FIRST (claim → stash → assemble_aux_block → submit); KAS arm runs iff its own target
is met and is NEVER gated by the claim (invariants 4/5/6). Gating is on the JOB's
commitment, not a static flag — correct across attach/detach.

**A DOUBLE = 1 solve, 2 chain-hashes.** One winning nonce → parent hash lands on KAS,
`H_fc` lands on ZKAS (aux rides outside the header hash). Display law:
`solves = kas + zkas − doubles`; Recent Blocks = one row per solve; a dual row carries
BOTH labeled hashes. `ShareOutcome` is the rendezvous — each arm deposits its hash at
accept time so the second finisher emits both.

---

## 3. LIVE-RUN FACTS (hard-won, non-obvious)

- **Kaspa node: gRPC on 16110; wRPC-Borsh on 17110.** Legacy RKStratum used 17110
  (the remembered "efficiency" — Borsh is lighter than protobuf; also why it paired
  with `tcp_no_delay`). v2 bridge speaks gRPC → must use 16110. Symptom key:
  wrong-protocol port = accept-then-abort (BrokenPipe/ConnectionAborted);
  dead port = ConnectionRefused.
- `tcp_no_delay` is NOT in the v2 config schema and is SILENTLY IGNORED if added
  (no `deny_unknown_fields`). tonic defaults TCP_NODELAY on; observed latencies
  (`rpc k=0.6 z=0.5`, `sub=0.2/0.4ms`) confirm no Nagle problem.
- **`--node-mode external` is REQUIRED** (default is `Inprocess` → hits the Windows
  stub and exits with a clear error). Baked into both launcher .cmd files.
- **WS2 verified live**: job counter 6→621 in 194s ≈ 3.2 j/s at 1 rig; 22.4 j/s at
  full fleet — 10 BPS push coalesced by the 250ms limiter. A ticker alone cannot
  produce this.
- **V2 gate: BANKED** via the status line rather than debug logs — `zk=ok <age> 100%`
  (100% of templates committed) plus `near k=… z=…` (per-chain near-miss rates can
  only exist if `merged_fc_target` resolves per share).
- Env vars are PER-POWERSHELL-WINDOW and were lost 3× across restarts (each loss =
  a KAS-solo window). Fixed permanently by the launcher .cmd files.
- Prom balance fetch fails by design (mining wallet not utxo-synced) → gate it off.

---

## 4. BLOCKS FOUND THIS SESSION (all KAS-solo; merged env unset in those windows)

| Time (ET) | Evidence | Notes |
|---|---|---|
| 08-06 00:00:34 | kaspa.stream explorer | RC's first block, ~21 min into first shift |
| 08-06 02:26:21 | dashboard Recent Blocks | bluescore 503852615, w9m |
| 08-06 03:30:14 | console + dashboard | bluescore 503891349, w9m, ACCEPTED + BLUE in 2s |

No dual/zKAS block yet. At 14.23 TH/s with `zk=ok 100%`, KAS expects ~1 block/33min
and **every KAS full-clear under merged mode is also a zKAS block** (KAS target ⊂ zKAS
target), so the first `D` row is imminent whenever the fleet runs with env set.

---

## 5. BUG LEDGER ADDITIONS (this session)

1. **Bug #1 (Windows risc0/cdylib) — THREE vectors, all now closed:**
   (a) `kaspad → wrpc-server` cdylib → `kaspad` dep gated `cfg(not(windows))`,
   in-process-node module stubbed on Windows, integration tests gated;
   (b) `consensus-core → shielded-core → risc0` (ungateable, load-bearing) →
   `cfg(windows)` `no_mangle` host stub for `sys_alloc_aligned` in lib.rs;
   (c) **example binaries** don't auto-link the parent lib → `use kaspa_stratum_bridge as _;`
   in `examples/c7_preflight.rs`. Release builds always survived via LTO dead-strip —
   which is why `-win` tags built while `cargo test` never could.
2. **Rebrand-sed casualties — now FOUR documented** (this fork's signature bug class):
   FCMM Cargo.lock pin; legacy address fixtures; the `NetworkIdError` message
   ("legacy 'zkas'"); and the network-name parser rejecting `kaspa-mainnet`.
3. **`MergedPending` eviction-window bug (mine, found by self-critique):** insert ran
   per template REQUEST (~21/s) instead of per distinct template (~1/s), shrinking the
   4096-entry ring to a ~3-minute window; a stale-but-valid solve could lose its zKAS
   leg silently. Fixed: insert moved into the fetch-success branch.
4. **Committed rebase-conflict markers** shipped in `share_handler.rs` (build break) —
   root cause: `git add` during a rebase staged the markers verbatim, and the tree was
   pushed without a local compile.

---

## 6. OPEN ITEMS (next session, priority order)

1. **Check for the first DUAL** — dashboard `D` row (both hashes), Blocks column
   `k/z/d`, treasury `zkas:px7ggt9l…` for a ~53.8 ZKAS coinbase.
2. **Near-miss ratio discrepancy** — console session-best vs dashboard disagreed
   (5.41e-1% vs 1.22e+1%), and ~19% figures are implausible per c.12's own math.
   c.14's forensic hex-dump is armed to catch it; read the log after the next spike.
3. **Status-line `blk=` anomaly** — showed 1,103,374 at 05:10 vs 1,479,661 at 03:36
   on the same synced node. Display/units artifact; mining unaffected.
4. **Merge PR #3** (green `bridge` gate), then `deploy.yaml` needs `--bin stratum-bridge`
   + zip packaging (web-UI edit — token has no Workflows scope), then tag
   **v2.1.0-rc1-win**.
5. **Token**: expires 08-12. Revoke + remint per discipline (Read-only dropdown trap
   hit BOTH mints — confirmation modal must read "Read and write").
6. **Upstream PRs** to firecash/zkas-rusty: `cb632f7` (address fixtures) and `d0232e1`
   (kaspa- network prefix) — both self-contained, both fix THEIR CI too.
7. **Punch list**: prom balance fetch gate-off; flip `Inprocess` default or document;
   restore `-D warnings` on bridge-check clippy (bridge crate is now clippy-clean);
   server-side `BlockInfo` optional `kas_hash`/`zkas_hash` fields (c.13's client-side
   nonce-join is the current mechanism, now with the lossless pair as primary);
   delete merged branch `ws2-notification-hub`; dedupe the two launcher .cmd files.
8. **Spec Draft 4**: §7 asterisk (in-workspace immunity covers the LOCKFILE class only;
   cdylib class had three vectors); WS2 marked BUILT; WS1 c.1–c.6 + WS3 c.7–c.14 status;
   solve-based display semantics as a WS3 requirement.
9. **Gates remaining**: V3 (near-miss rate — instrument exists, needs a clean read),
   V4/V5 (dual settlement on a real find), V6 (failure drills), V7 (≥48h A/B vs
   RKStratum) before retiring the reference bridge and RKStratum.

---

## 7. PROCESS RULES (earned; bind future sessions)

- **LOCAL-FIRST**: every command destined for CI runs on Michael's Windows machine
  first. The sandbox cannot compile. Never hand-patch `Cargo.lock`.
- **BUILD AFTER EVERY MERGE/REBASE, BEFORE PUSHING.** Two build breaks this session
  came from trees no compiler had seen.
- **Conflict resolution means EDITING THE FILE** — deleting the `<<<<<<< / ======= /
  >>>>>>>` markers and choosing content. `git add` during a rebase stages whatever is
  there, markers included. After any rebase: `grep "^<<<<<<<"` before building.
- GitHub required-check contexts are JOB names. Workflow files go through the web UI.
- Verify beats infer: ports (16110 vs 17110), wire formats (hex vs bytes), rebuild-time
  heuristics — every inference that lost this session lost to a one-command check.
- Verbatim-ported files stay verbatim; new assertions live in `tests/` beside them.
- Debug-level logging is for bring-up only; **state-change INFO + status-line telemetry
  is the production instrument** (this session's telemetry made debug unnecessary).
- Test contracts before assertions: two tests this session encoded WRONG contracts
  (hub `Closed` semantics; raw-bytes commitment) and both cost a CI round.

---

## 8. INVENTORY (unchanged but restated for cold starts)

- **Host**: ACEMAGICIAN Kron K1 (Ryzen 7 5825U, 32GB, 1TB NVMe).
- **Fleet**: w1,w2,w5,w6 = KS0 Ultra (~100–430 GH/s each); w7,w8,w9 = KS7 Lite
  (~3.6–4.9 TH/s each). Total ~14.2 TH/s. Merged-side suffix "m".
- **Nodes**: kaspad v2.0.1 (gRPC 16110, wRPC 17110, p2p 16111); zkas node v1.0.5
  (gRPC 16810, p2p 16811) + walletd :8501.
- **Payouts**: KAS → per-worker `kaspa:qznq82nszz…` via stratum authorize;
  ZKAS → single treasury `zkas:px7ggt9l6kh45k2nffc63mpclvz92mln4z6cvt2dcnewxa7c8950dgtl8lhyk3nqvdqyw8qc5r3fxrn`.
- **Ports in play**: 5555–5563 RKStratum (production), 5655–5663 reference merged
  bridge, **5755/5765 the RC**, dashboards 3030/3033, prom 2114–2122/2214–2222.
- **Session close state**: RC running unattended with the full fleet; RKStratum
  available as each rig's pool #2 failover.
