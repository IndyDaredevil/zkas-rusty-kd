# ADDENDUM to SESSION-STATE-2026-08-05 — late session (through ~00:05 08-06)

## WS1 completed c.4–c.6 + RC LIVE (branch merged-ws1-port, PR #3 ready-for-review)
- c.4 (35cf123): template decoration — commitment enters via GetBlockTemplateRequest
  extra_data (coinbase_tag||ZKMM||hex(H_fc)); parent pays the WORKER's kaspa address
  (payout model (a) natural consequence); current_zkas_template(): cache-first
  (TTL 500ms) + Semaphore(1) single-flight + 250ms hard budget → any miss = plain,
  never late. Helper trio ported flag-free (gates on JOB's commitment).
- c.5 (1f9bf99): dual-target share gate (zkas via merged_fc_target OR kaspa bits);
  zkas settlement spawned FIRST (claim→stash→assemble_aux_block→submit), KAS arm
  iff meets_network_target and NEVER gated by claim (inv 4/5/6); trait grew 4
  merged methods w/ plain defaults (mocks) + concrete overrides (both halves in
  one commit — defaults-only would silently never settle).
- c.6 (62c93ae): env wiring ZKAS_MERGED_NODE + ZKAS_TREASURY_ADDRESS (both
  required; one alone warns loud). Unset = byte-for-byte RKStratum.
- d0232e1: consensus/core network.rs accepts 'kaspa-' name prefix read-side
  (twin of 7372571; to_prefixed/p2p/datadir untouched). Found LIVE in RC's first
  60s: real-Kaspa-primary was never exercised in fork history. Also FOURTH sed
  casualty repaired (error msg read "legacy 'zkas'"). UPSTREAMABLE.
- e3b4072: V2 observability — "merged: committing to H_fc …" template debug +
  "[MERGED] dual-target: zkas_target=… kaspa(network)_target=…" per share.
- PR #2 MERGED to main (8b19f69 + WS2 stack). PR #3 = RC v2.1, ready-for-review,
  gate=bridge check. All 135 tests green on Windows after every commit.

## RC live-run facts (rc-v2-smoke.yaml, --node-mode external)
- Kaspa gRPC = **16110**. 17110 = wRPC-Borsh (legacy RKStratum client; that's
  the remembered "optimization" + why it needed tcp_no_delay). Symptom key:
  wrong-protocol port = accept-then-abort (BrokenPipe), dead port = refused.
- tcp_no_delay: NOT in v2 schema (silently ignored if added); tonic client
  defaults NODELAY on. Node line latency healthy.
- --node-mode external REQUIRED (default Inprocess → Windows stub politely errs).
  WS5 punch: flip default or bake into launcher.
- Background attach: "Merged mining ACTIVE after 1 connect attempt(s)" 7ms after boot.
- NODE line full parity with production RKStratum (n=Mainnet, blk synced,
  d=1.52e16, tip) after the kaspa-prefix fix.
- WS2 live-verified: job counter 6→621 in 194s ≈ 3.2 jobs/s = 10 BPS push
  coalesced by 250ms limiter (ticker alone can't produce this).
- w9m (KS0U, 4.52 TH/s) on :5755, kaspa-address username, 1321 shares/21min, 0 stale.
- **FIRST BLOCK: KAS chain block 08-06 00:00:34, explorer-verified (kaspa.stream),**
  ~21 min into first shift (expected ~2h at 4.52 TH/s — lucky). Solo-vs-DOUBLE
  UNRESOLVED at session end: restart may have lost env vars (second window) —
  CHECK: boot banner ENABLED lines, scrollback for "[ZKAS]" and "committing to
  H_fc", treasury explorer for ~53.8 ZKAS coinbase @ same timestamp.

## RC punch list (new, small)
- prom balance fetch fails (mining wallet not utxo-synced BY DESIGN) → gate off.
- clippy -D restore on bridge-check; Inprocess default; env-vars-in-window
  footgun → launcher .cmd bakes RUST_LOG + merged vars + --node-mode external.
- spec §7 asterisk: in-workspace immunity = lockfile class only; cdylib class
  had TWO vectors (kaspad-gateable + consensus→shielded→risc0 host stub).

## Next: settle solo-vs-double; V2 pair line from the dual-target debug
(needs env-complete restart); then V3 (near-miss rate), soak, PR #3 merge,
deploy.yaml --bin stratum-bridge (web UI), tag v2.1.0-rc1-win. Token expires
08-12; revoke+remint per discipline.
