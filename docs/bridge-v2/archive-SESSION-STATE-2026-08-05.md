# SESSION STATE — 2026-08-05 (end of day)
### Merged-mining bridge v2 port · continuation of SESSION-STATE-2026-08-04
### Production: v0.3.3-win (zkas-pool-kd @ d5c3e29) serving 6 rigs ~11 TH/s; w7 control on RKStratum

## 1. Production status (unchanged today; dev day)
- Day-one v0.3.3 record now canonical in spec: **21 zKAS (~1,130 ZKAS, luck ≈133% over
  9.1h window from 14:51 ET) + 15 merged-KAS (~36.7 KAS)**; combined KAS day total
  **40 blocks** (merged + RKStratum) ≈ the 38/day statistical expectation at 14.2 TH/s
  despite cutover churn — zero-marginal-cost thesis demonstrated at day-ledger level.
  Full-clear 15/21 ≈ 71% is upper-biased (pre-fix KAS finds = lost doubles); band 55–66%.
- First 24h checkpoint numbers NOT yet collected (Luck cards, A/B stale%, w7 decision).
- Wallet items open: confirm blocks 1–2 in phone wallet; sweep p9f9d2d → px7ggt9l decision.

## 2. Repos & infrastructure (all verified today)
- **IndyDaredevil/zkas-rusty-kd** (fork of firecash/zkas-rusty — NOTE the -kd; the
  08-04 state doc's "zkas-rusty" was a transcription error): THE port target.
  - main protected by ACTIVE ruleset: refs/heads/main only; PR required (0 approvals);
    required status check = context **`bridge`** (the job inside bridge-check.yaml —
    NOT "Tests", NOT "bridge-check"); force-push/deletion blocked; no bypass actors.
  - bridge-check.yaml: Protoc step required (arduino/setup-protoc@v3); clippy is
    ADVISORY (continue-on-error) pending inherited-lint triage — bridge crate itself
    is now clippy-clean after the for_kv_map fixes, so -D can likely return; cache
    key suffixed -v2; push trigger only (ws2-*/merged-*), no pull_request (dedup).
  - Inherited CI noise floor (upstream-wide, ignore): Lints (fmt), Check no_std.
    Full Tests suite now runs PAST test 4 (address fix) — final full-suite verdict
    not yet checked; more latent failures may lurk ("dam opened" caveat).
- **firecash provenance corrections** (spec updated): pool bridge is v2.0.0-core
  (re-vendored 2026-06-13 per its UPSTREAM.md), NOT v1.1.0; upstream rusty-kaspa
  v2.0.0 and v2.0.1 bridge/ are byte-identical → reference and port target share
  the SAME stratum core; the "d79bf68 wire merged YAML" commit is REAL in
  firecash/zkas-pool (identical content to solo-dual-mode c2cd7e1 = WS5(a) port
  source); upstream zkas-pool Cargo.lock STILL pins broken 424b7036 (64 refs) —
  FCMM disclosure reclassified opportunistic (solo-dual-mode v1.0.6 is the official
  working path); 80e8e2b revert = packaging moved to solo-dual-mode repo (ronnie
  question answered, not asked).
- **Token**: fine-grained PAT on zkas-rusty-kd only, Contents+PR write (second mint;
  first was revoked, second needed a permission edit — the Read-only dropdown trap
  hit BOTH times). Expiry 2026-08-12. REVOKE + REMINT next session per discipline.

## 3. Code landed on main (via PR #1, merged 8b19f69 + PR #2 — CHECK PR #2 merge
   status next session; it was open, green-gated, mergeable at session end)
- **NotificationHub** (bridge/src/notification_hub.rs): per-CLIENT relay demuxing by
  variant into per-scope broadcast channels (NewBlockTemplate, VirtualChainChanged);
  watch-based ClientHealth (Receiving/Disconnected) = WS4 mode-machine input;
  Lagged = forced-resync semantics; capacity 256. 5 unit tests incl. 9-subscriber
  fan-out and MPMC-trap demux isolation. KEY DESIGN LAW: one hub per client, never
  per scope — all scopes multiplex into one gRPC channel; two readers steal.
- **addresses fix** (cb632f7, upstreamable): 8837354 rebrand sed relabeled legacy
  fixtures firecash:→zkas: keeping old checksums → BadChecksum fail-fasted the whole
  1448-test suite at #4 forever. Same pathology class as FCMM (rename outrunning
  derived artifacts) — firecash struck twice, both found by this project.
- **for_kv_map clippy fixes** in share_handler (inherited code, mechanical).

## 4. Branch ws2-kaspaapi-restructure → PR #2 (green `bridge` check, unstable=mergeable)
- KaspaApi: take()-once mpsc → hub field; both template listeners are thin wrappers
  over free run_template_listener (push-primary, ticker fallback, burst-drain kept,
  Lagged resync, shutdown arm); vestigial wait_for_sync preamble + restart_channel
  no-op REMOVED (kept wait_for_sync under allow(dead_code) as WS4 health probe).
- main.rs: is_first_instance gate DELETED — all 9 instances subscribe (WS2 acceptance
  test: all_nine_instances_receive_push_notifications, passing).
- **Windows test builds unblocked — bug ledger #1 has TWO vectors** (spec §7 needs
  asterisk): (1) kaspad→wrpc-server cdylib — gated: kaspad dep now
  [target.'cfg(not(windows))'], inprocess_node real module non-Windows + Windows
  stub erroring politely, integration tests (embed node) cfg'd non-Windows;
  (2) consensus-core→shielded-core→risc0 (UNGATEABLE, load-bearing) — cfg(windows)
  no_mangle host stub for sys_alloc_aligned in lib.rs (guest-only syscall,
  aborts loudly if ever hit). Release builds always survived via LTO dead-strip —
  why v0.3.x-win tags built while cargo test never could. Windows now runs the
  FULL suite: 130→135 tests incl. 119 bin tests runnable on Windows first time ever.

## 5. Branch merged-ws1-port → PR #3 (DRAFT) — Release 1 foundation, 4 commits, all
   verified 135-green on Windows
- c.1 (c0310fc+74d1111): merged.rs + merged_derisk.rs byte-identical from
  solo-dual-mode bridge-src; brought 3 embedded tests (incl. invariant 5 one-shot
  claims, merkle-branch property test). Only dep gap was kaspa-merkle. Lock
  regenerated LOCALLY (hand-patch lesson applied). MERGE_MINE_MAGIC=*b"ZKMM"
  confirmed in-workspace at consensus/core/src/auxpow.rs — bridge & node compile
  the constant from the same file.
- c.2 (b5c3898+ceded67): invariants.rs — magic asserted vs consensus constant;
  WIRE FORMAT FACT: commitment = prefix||MAGIC||hex(H_fc)||suffix, ASCII lowercase
  hex 64 chars (COMMITMENT_HEX_LEN), BECAUSE extraData is a protobuf UTF-8 string.
  First test version wrongly assumed raw bytes against our own FCMM forensics.
- c.3/c.3.5 (e79fa38+f09ad43): KAS-primary inversion — primary client = Kaspa node
  (production RKStratum semantics preserved; his yaml needs only ADDITIONS);
  ZKas leg = Arc<RwLock<Option<Arc<ZkasLeg{client,hub,pay_address}>>>> filled by a
  BACKGROUND attach task retrying forever (capped 30s backoff, log-quieted 1/10);
  constructor never waits — boot with no zkas node, MERGED activates on appearance
  (operator design review upgraded bounded→forever; startup order irrelevant BY
  CONSTRUCTION). Primitives: has_zkas/zkas_leg/zkas_hub/get_zkas_block_template
  (single treasury, payout model (a))/submit_zkas_block ((&block).into() carries
  aux_pow — invariant 2). new() delegates with None → bridge IS RKStratum.
  Loss-after-attach (detach/re-attach) = WS4 scope.

## 6. NEXT SESSION — start here
1. Revoke old token, mint fresh (Contents+PR write, zkas-rusty-kd, watch the
   Read-only dropdown trap — confirmation dialog must say "Read and write").
2. Check PR #2 status; merge if not yet merged; REBASE merged-ws1-port onto main.
3. **c.4 — template decoration (THE design session; re-derive, don't port):**
   reference wrap hook is zkas-primary; ours inverts: get_block_template serves
   PLAIN Kaspa parent by default; when zkas leg attached, opportunistically fetch
   zkas template under a strict time budget (next job plain rather than late —
   non-blocking commitment attach, spec §3), build_parent_block → committed parent,
   MergedPending (needs Arc<Mutex<>> home on KaspaApi) keyed H_fc. Read the
   reference's get_block_template wrap + submit aux-reassembly SIDE-BY-SIDE first
   (zkas-pool-kd/bridge/src/kaspaapi.rs merged sections + share_handler dual paths).
4. c.5 — dual-target share handling + dual settlement (invariants 3,4,5,6 tests).
5. Then: main.rs/yaml config wiring (WS5(a) port from solo-dual-mode's
   new_with_merged — inverted), V2 gate on live nodes, WS4 mode machine branch.
6. Housekeeping queue: delete merged branch ws2-notification-hub; upstream PR of
   cb632f7 address fix to firecash/zkas-rusty; spec Draft 4 (risc0 two-vector
   asterisk §7, WS2 marked BUILT, c.1–c.3.5 status, this doc's provenance
   corrections are already in Draft 3); re-enable -D warnings on bridge-check
   clippy; check full Tests-suite verdict past test 4; 24h checkpoint numbers;
   France-relay peer check (51.210.219.138 in getConnectedPeerInfo? likely already
   peered at outpeers=16 on ~19-node network — verify before adding).

## 7. Process rules earned today (bind future sessions)
- LOCAL-FIRST: every command destined for CI runs on the Windows machine first.
  The sandbox cannot compile (needs Rust 1.91; toolchain domain blocked) — his
  machine (cargo 1.95) is the local. Never hand-patch Cargo.lock.
- GitHub status-check contexts = JOB names ("bridge"), not workflow names.
- Fine-grained PAT permission dialog must be verified against the confirmation
  modal (Read and write), and API `permissions`/`admin:true` reflects the ACCOUNT,
  not the token — the only real write test is a write.
- Workflow files cannot be pushed by the token (no Workflows scope, by choice):
  all .github/workflows edits go through Michael's web UI.
- nextest fail-fast means "Test Suite red" says nothing about OUR tests until the
  blocker is identified; bridge-check is the scoped truth.
- Verbatim-ported files stay verbatim; new assertions live in tests/ next to them.
- Assumptions about wire formats lose to field forensics (raw-bytes vs ASCII-hex).
