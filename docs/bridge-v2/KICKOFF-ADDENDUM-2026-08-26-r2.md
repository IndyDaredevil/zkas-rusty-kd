# KICKOFF ADDENDUM — 2026-08-26 late session (spec-pair commit) — r2
### Rides with KICKOFF-v2.0.1.5-r2.md (da380662...7adc15, which stays LIVE and
### unmodified). Corrections and additions from the 08-26 spec-split session.
### r2 vs r1 (r1 sha 2dc7c8cf...bb5501 VOID, never committed): §A.4 clone
### question RESOLVED (canonical path declared, duplicate deleted); §B
### two-clone ledger candidate finalized from pending to written.
### Read the kickoff first; where this addendum and the kickoff disagree, the
### addendum wins (it is strictly later).

## A. Corrections to the kickoff's stated facts

1. Docs-tip is now `e7426e1` (kickoff §1 says 5b37875). Commit e7426e1 added
   BRIDGE-SPEC.md r2 + NODE-CONTRACT-v1.0.5.md r1 and archived the Draft 4
   port spec as archive-merged-bridge-v2-spec-draft4.md (git mv, history
   preserved). Code identity 336b7a5 unchanged.
2. KRON-HARDENING.md was NOT on origin at kickoff cut time (kickoff header
   lists it as a companion doc in docs/bridge-v2/) — it existed only as an
   untracked file in the laptop clone + the 08-22..26 chat cut. Committed
   alongside this addendum; if reading this from the repo, the discrepancy
   is closed.
3. Companion-doc list gains three: BRIDGE-SPEC.md (r2, as-built system spec,
   sha 070c717d...557001), NODE-CONTRACT-v1.0.5.md (r1, consumed-binary
   contract, sha a94ef5b5...a2ee0), archive-merged-bridge-v2-spec-draft4.md
   (frozen design history: port rationale, invariants 1-8, V-gates).
4. Laptop clone: CANONICAL PATH IS `~/zkas/zkas-rusty-kd` (the 08-22 clone;
   blob:none, docs/bridge-v2 sparse cone, gh browser-auth). A duplicate
   clone accidentally created at `~/zkas-rusty-kd` during the 08-26 late
   session (spec-pair commits e7426e1 were made FROM the duplicate) was
   verified clean and deleted same session; the canonical clone was
   fast-forwarded to e7426e1. One clone, one path, from here forward.

## B. Additions to §7 (BL-032+ raw list)

- BL-004 cross-doc drift: the ledger's "structurally immune" wording was
  retracted by Draft 4 §6 (immunity = lockfile-drift class ONLY; the
  cdylib/risc0 LNK class lives in-workspace and took three fixes). The
  ledger inherited forward what the spec had walked back. Correct BL-004's
  text at reseal.
- Meta-principle 1, documentation tier: BRIDGE-SPEC r1 reproduced the
  retracted claim by drafting from ledger + memory instead of reading the
  in-repo artifact; the pre-move `ls` gate surfaced the old spec and caught
  it before commit. A spec cut from secondary records inherits their drift.
- Conduct law minted: a deliverable's destination is verified-or-created
  within the same instruction set, and multi-step sequences state their
  gating (deploy one-liners shipped against a roadmap-item checkout; ~2 min
  cost, zero writes — atomic mv failures).
- Discovery: the "incomplete docs/bridge-v2 commit" open item was already
  closed on origin pre-session (SCOPE, ledger, session-state, README,
  monitoring/, reporter/ all present at 5b37875).
- Two-clone incident (08-26, resolved same session): the failed `cd` that
  opened the session was read as "checkout never stood up" when a clone
  existed at a DIFFERENT path — a second clone was created, and the two
  briefly diverged (new at e7426e1, old self-reporting "up to date with
  origin" at 5b37875 from stale refs — BL-020 live, at repo scale). The
  old clone held the only copies of two uncommitted docs, exactly where the
  hazard analysis predicted. Laws: `find` for existing clones before
  cloning; a clone's path is part of its identity and gets recorded like a
  sha; "up to date with origin" is a statement about last-fetched refs,
  never about origin.

## C. Guidance for the new chat's streams (additive, no reordering)

- A1' source read anchors on BRIDGE-SPEC §5 (metrics contract) and §6
  (logging contract) as the as-built reference; discrepancies vs the tree
  are ledger entries per that spec's own ground-truth clause.
- Stream P enrichment work cites BRIDGE-SPEC §6/§9 (kaspa-parent join
  window, schema pointers) + NODE-CONTRACT §3 (RPC caveats, aux_pow
  stripped from responses).
- Queue item 7 reduces to: fold §2-§5 of the kickoff back into SCOPE r2 and
  commit it (KICKOFF + this addendum + KRON-HARDENING land with the 08-26
  late-session commit).
- Invariant 8 (accounting law, `solves = kas + zkas - doubles`) is now
  codified in BRIDGE-SPEC §7 as a standing contract on ALL consumers —
  P-stream schemas must honor it.
