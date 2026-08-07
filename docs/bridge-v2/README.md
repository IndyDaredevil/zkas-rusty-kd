# Merged-Mining Bridge v2 — working documents

Version-controlled home for the v2 bridge's design spec and session-continuity
notes. These live in the repo (rather than a chat session or a local folder) so
they version alongside the code they describe and survive machine changes.

## Reading order for a cold start

1. **`SESSION-STATE-2026-08-07.md`** — CURRENT state. Where every branch, PR,
   config, port, and open item stands; the bug ledger; the process rules.
   Read this first; it supersedes both archived state docs.
2. **`merged-bridge-v2-spec.md`** — the design spec (**Draft 4, 2026-08-06**).
   Architecture contract (incl. as-built attach lifecycle, commitment wire
   format, and template budget), workstreams WS1–WS8 with status marks, the
   **eight** consensus invariants, the V2–V7 validation gates, rollout plan
   with the cutover deviation recorded. Draft 3 remains in git history.
3. Archived state docs (`archive-*`) — kept for provenance only; every fact in
   them that still matters is carried forward into the current state doc.

## Conventions

- **One state doc is current at a time.** When a new one is written, the prior
  becomes `archive-SESSION-STATE-<date>.md` and the new one carries forward
  anything still live. Never fork the truth across two current docs.
- **The spec is the contract; the state doc is the position.** Design decisions
  and invariants belong in the spec; what is built, running, or broken belongs
  in the state doc. When they disagree, the state doc describes reality and the
  spec needs a revision — say so explicitly rather than silently editing.
- Drafts are numbered in the spec's header block, each listing what changed
  from the prior draft. Never renumber silently.

## Related, elsewhere in the repo

- `bridge/` — the bridge crate itself (stratum core, merged mining, telemetry).
- `bridge/tests/invariants.rs` — the consensus-invariant tripwires (spec §5).
- `start-rc-bridge.cmd` (repo root) — RC launcher; bakes the merged env vars and
  `--node-mode external` so a restart cannot silently drop merged mining.
- `.github/workflows/bridge-check.yaml` — the scoped CI gate (`bridge` job) that
  `main` requires; full-workspace `ci.yaml` carries an inherited noise floor
  (Lints, Check no_std) documented in the state doc.
