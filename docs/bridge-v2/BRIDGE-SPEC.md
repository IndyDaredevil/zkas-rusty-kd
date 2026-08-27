# BRIDGE SPEC — zkas-rusty-kd RC merged stratum-bridge
### Revision: r2 · 2026-08-26 · Describes production release v2.0.1.4 (engine 2.0.1)
### r2 corrections vs r1 (r1 VOID, never committed): §1 in-workspace immunity claim
### narrowed to the lockfile-drift class per the Draft 4 retraction (r1 repeated the
### retracted BL-004 attribution — inherited from the ledger's own BL-004 wording,
### cross-doc drift now flagged); §3.6 invariant-7 test upgraded from "suggested" to
### existing-per-Draft-4 (verify in tree); §7 invariant-8 accounting law added.
### Status: OWNED artifact spec. Versioned internally against the bridge release line;
### revision trigger = a bridge release, or a NODE-CONTRACT revision (re-review §3).
### Companion documents (cited, never duplicated — tier law 10):
###   NODE-CONTRACT-v1.0.5.md (engine contracts), ENGINEERING-LEDGER.md (BL-series
###   evidence), SCOPE-v2.0.1.5.md (next-release scope), reporter/webhook docs,
###   archive-merged-bridge-v2-spec-draft4.md (design-history record: port rationale,
###   invariants 1–8, V-gates — the WHY behind contracts this spec states as law).
### Local source is ground truth (conduct law 4): where this spec and
### C:\Users\inmyh\zkas-rusty-kd disagree, the tree + compiler win; file a ledger entry.

---

## 1. IDENTITY & VERSIONING

- Repo: github.com/IndyDaredevil/zkas-rusty-kd (fork of firecash/zkas-rusty;
  bridge is IN-TREE). In-workspace builds structurally close the BL-005
  LOCKFILE-DRIFT class ONLY (bridge and node compile consensus constants from
  the same file). The BL-004 cdylib/risc0 LNK class lives INSIDE the workspace
  and required three separate fixes — the broader "structurally immune" claim
  was RETRACTED in Draft 4 of the port spec (archive-merged-bridge-v2-spec-
  draft4.md §6); the ledger's BL-004 entry still carries the over-broad
  wording (cross-doc drift, flagged for the next ledger session).
- Local tree: `C:\Users\inmyh\zkas-rusty-kd`.
- Branch model: the production line IS the current `merged-vE.N.G.B` release
  branch; each release branch becomes canonical on release — NO
  merge-to-production step, lineage strictly linear (verified 2026-08-20).
  Current: `merged-v2.0.1.4` @ `336b7a5` = origin = tag `v2.0.1.4-win` =
  running exe (the four-way identity check — the ONLY accepted identity proof).
  Lineage: 233c5f7 (v2.0.1.2) → 10892ed (v2.0.1.3) → 336b7a5 (v2.0.1.4).
  `merged-ws1-port` is RETIRED and a BL-019-class trap while it exists.
- Version banner: DERIVES from `CARGO_PKG_VERSION` + `BRIDGE_BUILD: u32`
  ordinal (no hardcoded constant); `deploy.yaml` extracts release tag + banner
  at release time and FAILS the build on mismatch (canary-tag-aware,
  empty-extraction-fails-loud). Banner format:
  `RC merged bridge v2.0.1.4 (engine 2.0.1)` — the engine prefix doubles as a
  rebase-drift alarm vs the node engine (BL-030).
- Versioning scheme `E.N.G.B`: engine-prefixed; bump B per bridge release on
  the same engine.

## 2. RUNTIME TOPOLOGY (production, Kron)

| Component | Endpoint | Notes |
|---|---|---|
| Stratum instance 1 | `:5755` | KS0 Ultras |
| Stratum instance 2 | `:5765` | KS7 Lites |
| Canary stratum | `:5775` | isolated canary + dedicated prom port (meta-principle 8) |
| Bridge metrics | `:3034` | Prometheus scrape target `rc_merged_bridge` |
| zKAS node (template source) | 127.0.0.1:16810 gRPC | NODE-CONTRACT §2 |
| Kaspa node (parent template) | 16110 gRPC | kaspad v2.0.1 |

- Launch: ONLY via `run-rc-merged.cmd` (tracked in-repo as of v2.0.1.4).
  First line `cd /d %~dp0` (BL-018 PATH-collision guard). The two ENABLED
  env lines are the contract — read them every launch (BL-017: cmd.exe
  `set` without `=` is a query; a lost assignment runs silently degraded).
- Required env: `ZKAS_MERGED_NODE` + `ZKAS_TREASURY_ADDRESS` — BOTH required
  or the bridge runs silent plain-mode (BL-019). Kaspa leg: unset/unreachable
  Kaspa node degrades gracefully to a synthetic parent — zKAS keeps flowing,
  that round earns no KAS (fail-safe verified in source, BL-017).
- Production exe caveat: the running binary is a release-zip copy sitting in
  `target\release` — any local cargo build silently overwrites it (BL-019).
  Deploy sequence is park / copy / hash-verify / kill / launch; a running
  process must be killed explicitly (sub-second "Finished" = stale tree).

## 3. ENGINE CONTRACT (cites NODE-CONTRACT-v1.0.5.md §4 — re-review on any node bump)

The bridge honors, and its tests/guards enforce:
1. Template coinbase used VERBATIM — never rebuilt (dev fee, red-block reward,
   shielded_root all preserved). NODE-CONTRACT §4.1–4.3.
2. Payload handled by PARSING, never hardcoded offsets (+32-byte shielded_root).
3. AuxPoW assembly per NODE-CONTRACT §4.4: `ZKMM || H_zk` exactly once in the
   PARENT (Kaspa) coinbase extra_data; parent mined against ZKAS bits; solved
   parent submitted to BOTH chains (KAS as normal block, zKAS as aux block).
4. Merged submissions travel the RpcRawHeader/wRPC path ONLY (plain gRPC drops
   aux_pow — NODE-CONTRACT §3.1).
5. Magic-collision guard: `coinbase_tag_suffix` sanitizer REJECTS any value
   containing "ZKMM" (BL-009 — a passed-through "ZKMM" creates a duplicate
   magic window and kills 100% of merged submissions).
6. Lockfile discipline: kaspa-* crates resolve in-workspace (in-tree port), so
   the BL-005 FCMM class (stale git-dep lock pinning a pre-rename ancestor)
   is structurally closed. The invariant-7 magic-byte test EXISTS per Draft 4:
   it decodes `embed_commitment()` output against the consensus constant
   itself — never a string literal or raw-byte assert (the first version
   asserted raw bytes and was wrong). It stays as the tripwire for any future
   rename; the FORMAT half of the commitment is not covered by the structural
   argument. (Verify presence in the current tree — conduct law 4.)

## 4. STRATUM & SESSION SEMANTICS

- IceRiver fleet contract (BL-022): pre-authorize extranonce handshake
  required and works; rigs run priority-failover (NOT round-robin) — a
  merged-rig appearing on the production RKStratum dashboard IS the failover
  alarm; sub-minute backup-pool flicker at rig boot is normal.
- Wallet capture: the stratum username wallet is captured ONCE at authorize
  (single write site; BL-007). Config-page saves do NOT re-authorize a live
  session — wallet/worker changes require a rig reboot/reconnect.
  `zkas:` addresses take the fallback-tolerant path (empirically correct;
  never set POOL_FALLBACK_ADDRESS).
- Template delivery: notification-driven; polling listener is strictly
  fallback and must WARN loudly (BL-006 — silent capability degradation
  turned block_wait_time into the job cadence).

## 5. METRICS CONTRACT (`:3034`)

Series inventory (evidenced in ledger/production; verify against a live
`/metrics` render before treating as exhaustive):

| Series | Type | Notes |
|---|---|---|
| `ks_blocks_mined` | counter | KAS leg; warm-up zero-init per worker |
| `ks_double_blocks_mined` | counter | increments only after BOTH legs confirm blue (blue-confirm loop 30×2s — drives BL-026 timing) |
| `ks_merged_parent_submit_total` | counter | Kaspa-parent submissions |
| `ks_blocks_not_confirmed_blue` / `ks_zkas_blocks_not_confirmed_blue` | counter | proof series for zero-init reaching TSDB (BL-025) |
| `ks_valid_share_diff_counter` | counter | UNITS: exports as diff × 2^32 / 1e9 (GH); correct recording-rule multiplier is × 1e9, NOT × 2^30 |
| `ks_zkas_network_difficulty_gauge` | gauge | 30s stats loop (v2.0.1.4) |
| `ks_zkas_estimated_network_hashrate_gauge` | gauge | 30s stats loop; self-consistent <1% vs difficulty; the ONLY valid Luck denominator — the D_z/D_k ratio is UNPINNABLE (BL-028: ±9% in 20 min, ~30%/day) |

Behavioral contracts:
- Counters are warm-up ZERO-INITED and the zero-init REACHES the TSDB in the
  counting label context (BL-025: `series=N zeros=N` is the one-query test).
  Consequence: NO birth clauses in alert rules — the pattern is retired and
  must not be reapplied (BL-013 superseded by BL-025).
- The bridge NEVER retires a session's series: reconnects leave retired series
  in the SAME scrape as live ones. All block expressions must be wrapped
  `sum without (ip)` with `increase()` INSIDE the sum (BL-025). `wallet` is a
  second latent churn vector (BL-007 capture-once).
- `ip` label: port-strip relabel history is muddy (BL-010 revert note) —
  verify final relabel state whenever touching monitoring.
- OPEN (BL-029): `/metrics` render cost is plausibly UPTIME-dependent
  (series accretion). 12s-ceiling scrape kills observed at multi-day uptime;
  14s timeout vs 15s interval is the HARD ceiling. Test: `[8h]` max after
  several days unbroken uptime; remedies if confirmed are series retirement
  in the bridge or a longer scrape interval — NOT more timeout.
- Estimator drift (BL-028 minor): estimated_hashrate/difficulty reads
  ~2.05–2.11 vs theoretical 2.0 at 1 BPS — observed-blockrate window, not a
  constant offset. Fix scoped to v2.0.1.5 Stream A.

Scrape config coupling: 15s interval / 14s timeout; alert range windows 3m
with Alertmanager info-route group_wait/group_interval 90s — a range window
IS a notification deadline (BL-024/BL-026); never change one side alone.

## 6. LOGGING CONTRACT (consumed by the reporter — breaking changes here are
## breaking changes to the accounting rail)

- Location: `C:\Users\inmyh\AppData\Local\kaspa-stratum-bridge\logs\`
  (`RKStratum_*.log`; Windows resolves data_local_dir — NOT profile root;
  the cropped-grep misread cost two turns, SESSION-STATE 08-21).
- File logging compiled default TRUE since 2026-06-24 — on for the operation's
  entire life. >100 MB/day, NO rotation; old-file deletion is safe (reporter
  reads newest only) but BY NAME only — **never `git clean`** in this tree
  (`rc-v2-smoke.yaml` production config is untracked; BL-031).
- Lines with downstream consumers (reporter parse targets — treat as API):
  - zKAS block FOUND line (H_zk, worker, timestamp) — reporter beat 1 anchor.
  - Kaspa parent accept line — lands 6–62 ms after the zKAS FOUND line across
    all 25 confirmed doubles (essentially deterministic; the join window for
    the kaspa-side enrichment columns).
  - Near-miss event line (c.15): the k/z pair on every dual-leg near-miss IS a
    free D_z/D_k ratio measurement, exact per share (BL-028).
- Known fossils/traps:
  - `[MERGED] KASPA BLOCK FOUND & accepted` is a PRE-MERGED-ERA fossil — zero
    production intersection; do not build consumers on it.
  - `[ZKAS] DOUBLE` / `full_clear` semantics UNRESOLVED (~91% true, 566/646 vs
    ~25 real doubles) — pin against bridge source before the field ships
    anywhere (v2.0.1.5 scope; SESSION-STATE 08-21 §4.7).
  - The [MERGED] log's generic "Kaspa" naming means the TEMPLATE-SOURCE node
    (zKAS on 16810) in kaspaapi contexts.

## 7. DOWNSTREAM CONSUMERS (interfaces this bridge must not silently break)

1. **zkas-reporter.ps1** (ZkasReporter task): tails newest RKStratum log
   handle-free; two-beat protocol (~T+5s provisional, ~T+60s exact sompi via
   walletd nearest-neighbor join); POSTs to the KDSM edge function v2 with
   `X-Webhook-Secret`. Metrics `:9151` loopback; four alert rules. Known gap:
   replay scans newest log only (reporter-down spanning a bridge restart can
   miss blocks; 2-file replay patch on offer).
2. **Prometheus** (`:9090`): scrape contract per §5; 41 rules passing promtool.
3. **KDSM dashboard** (`zkas_blocks`, unique constraint
   `zkas_blocks_block_hash_key` as upsert arbiter — a partial index CANNOT
   arbitrate for supabase-js upserts; SESSION-STATE 08-21 §2.2).
4. **Telegram** via Alertmanager — deliberately INDEPENDENT second observer of
   block events (meta-principle 2 as architecture); `telegram-html` receiver
   scoped to the hourly card route only.

Standing contract on ALL consumers — **invariant 8, the accounting law**
(Draft 4): `solves = kas + zkas − doubles`. One solve is one winning nonce;
a double is ONE solve with TWO chain hashes. No counter, table, export, or
webhook may represent a double as two solves, nor render it with fewer than
both hashes. (Violated once by the first display implementation — WS3-DA.)

## 8. RELEASE & DEPLOY PIPELINE (no-local-Rust path — the pipeline IS the toolchain, BL-021)

1. Patch authoring: PowerShell anchored-replacement scripts — EOL detection,
   absolute paths, fail-loud anchor-count asserts (CRLF tree vs LF patch and
   process-cwd .NET path resolution are the two known traps).
2. Compile gate: `bridge-check.yaml` on push to `ws2-*`/`merged-*` (check +
   clippy-advisory + test, bridge crate only). `ci.yaml` full-workspace noise
   is ignorable.
3. Build: branch-targeted GitHub release → `deploy.yaml` → win64 zip assets
   (+ tag/banner guard, §1). Asset presence verified by direct URL HEAD probe,
   not the rendered releases page.
4. Canary: `:5775`, isolated prom port, control-rig holdout; runtime
   acceptance criteria stated before launch.
5. Fleet deploy: park / copy / hash-verify / kill / launch via
   `run-rc-merged.cmd`; first-launch banner line is the pipeline's own proof
   the copy landed (BL-030).
6. Rollback: parked prior exe + linear branch lineage; rollback conditions
   stated per deploy.
7. Backup hygiene: `.bak-*` mtimes LIE (Copy-Item preserves source mtime) —
   suffix is the only discriminator; keep the current release's set until one
   full production day, then delete by name (BL-031).

## 9. KNOWN ISSUES & FORWARD SCOPE (pointers, not duplicates)

- v2.0.1.5 Stream A: BL-029 series-retirement question, `full_clear`
  semantics pin, hashrate-estimator drift, K/Z/D 24h card bug,
  submit-latency/time-to-blue + stale-share/job-delivery-latency
  instrumentation. Stream P: reporter/schema incl. kaspa-side enrichment
  (`kaspa_parent_hash`/`kaspa_double`, 6–62 ms join window). Near-miss
  percentage data EXCLUDED from scope. (SCOPE-v2.0.1.5.md is authoritative.)
- BL-008 (blocks table counted zKAS only) — addressed by the RC K/Z/D table
  design; the 24h card bug above is the residual.
- mimalloc `purge_decommits` cfg!(windows) one-liner — parked for A/B.
- BL-026 inhibition/window coupling: verified-in-design, first real double is
  the production test; failure mode to watch is SILENCE, not noise.

## 10. VERIFICATION DOCTRINE (how any claim in this spec gets checked)

- Identity: four-way check (§1), never the banner alone pre-v2.0.1.4.
- Running config: Prometheus `/api/v1/rules` API readback — file-on-disk +
  200 reload can mask stale state (BL-020).
- Scrape continuity: count the SUBJECT series and diff vs scrape attempts —
  `up` is synthesized on every ATTEMPT and exists at 0 on failure;
  `count_over_time(up[...])` is a false all-clear through a total outage
  (BL-024).
- Any post-fix duration/latency reading must state process UPTIME alongside
  the number (BL-029).
- Evidence chain for every fix: byte-level proof + live behavioral proof
  (meta-principle 3).
