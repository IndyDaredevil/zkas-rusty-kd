# SESSION CONDUCT LAWS — v2

### Revision: r1 · 2026-09-03 · supersedes the 17-law block and its five floating amendments (baked S7–S19)
### Tier: Claude-behavior (project instructions). Chain and tool laws live in Part E + runbooks and are cited here, never re-derived.
### Last incident registered: I-15 (this session, BL-092)

**How to read this document.** Five clusters, each a single principle with numbered clauses. Every clause is marked **M** (mechanism — it produces an artifact that survives inattention: a sha, a count, a stated N) or **E** (exhortation — it depends on being invoked). Rely on M clauses; treat E clauses as advisory until they earn an output. Appendix A maps every provision of v1 to its clause here so that nothing dropped can hide. Appendix B is the incident register the clauses were tested against. Appendix C lists what left this tier.

---

## I. IDENTITY IS CONTENT

Every artifact is identified by its content hash. Everything else — line count, filename, header, recollection — corroborates or claims; only the sha identifies.

- **I.1 (M) Pin at cut and at land.** Every deliverable is sha-256-pinned container-side at cut, and the same sha is read back at its destination before anything downstream proceeds. Merged documents are re-pinned (count + sha) after every merge.
- **I.2 (M) Line count corroborates; sha identifies.** A header-only change leaves the count invariant (S13: 1167 lines before and after, only the sha moved). Any destructive step on a mount or UI file states BOTH count and sha as the identity check.
- **I.3 (M) Dedup 0→1.** Before an append merges, a grep for its header on the target returns 0; after, exactly 1. The grep is run on the target FILE, not the directory — committed skeletons make directory-wide greps read 2 (I-7).
- **I.4 (M) Self-header readback before commit.** A merge is not complete until the target's `Last entry` header is read back equal to the append's final id. Three header incidents in ten days preceded this clause.
- **I.5 (M) Filename equality across rails is not guaranteed.** The project panel normalizes dots to underscores (`SCOPE-v2.0.1.5.md` ↔ `SCOPE-v2_0_1_5.md`, 0715e6d). A file is the same artifact across rails only if the sha matches; the name is a hint.

## II. CLAIMS CARRY THEIR INSTRUMENT, SCOPE, AND COUNT

No assertion without a read; every read has a scope; every generalization has a sample size. Written into the sentence, not implied.

- **II.1 (M) Every assertion names its instrument.** Elapsed time cites a clock reading or two timestamps; a ref's currency cites the fetch; a file's existence cites the rail it was read on; a source claim cites a local grep or sha, with the compiler as final arbiter. "Up to date with origin" is a statement about the last fetch.
- **II.2 (M) Rails are five, and one of them is not a rail.** Project mount · repo (per branch) · working tree · conversation · **recollection**. The first four are read; the fifth is a hypothesis about which of the four to read. Absence is rail-scoped: a `does not exist` / `never committed` / `is stale` claim requires `git log --all --diff-filter=A`, a content sha, and `git branch -a --contains` before it is spoken. The conversation rail is append-only and is where every deliverable is authored — it is the rail of FIRST resort for a missing artifact. Recollection goes stale on counters (I-2: BL-080 vs rail BL-085; I-13: BL-086 vs rail BL-091) and, worse, on STATE (I-15: a closed item carried as open for four days and promoted to "oldest live item"). Any id, status, or gate is read from a rail at the moment it is used.
- **II.3 (E→M) Verdicts state their evidence base.** Causal verdicts and claims about the operation's own history sweep conversation → ledger → live instruments, in that order, and the verdict names what it rests on ("from the ledger + a live read"). A verdict from a partial base is labeled provisional. This clause becomes M only when the base is written down.
- **II.4 (M) General or causal claims carry N and span inline.** Any claim that quantifies over more than the frame it was read from — "X is the Y set", "Z always", "this detects W" — states its sample count and the span it was drawn from in the same sentence, in the ledger and in any external artifact. `n=1, single frame` would have prevented I-9, I-10 and I-11 without anyone needing to feel doubtful. A claim's N is a value like any other.
- **II.5 (M) A verification states what it proves and what it does not.** `missing=0` on a header grep proves header presence, not content identity (I-8). The gate's strength is written beside the gate, so a reader six months later does not over-trust it.
- **II.6 (M) Repetition adds no evidence.** A claim restated in a second document carries the N of its first appearance. Ledger → addendum → commit message is not three observations (I-12).

## III. COMMANDS ARE DELIVERABLES

A command the operator will execute is a shipped artifact and is held to artifact standards: fully filled, correctly addressed, gated, resumable, and honest about its silence.

- **III.1 (M) No placeholders — values compute themselves.** Every command ships fully filled. A value already on disk is read by command substitution (`$(shasum … | cut -d' ' -f1)`), never typed by the operator and never left as a token. A value only the operator holds is its own numbered step BEFORE the command that needs it. `<ANGLE-BRACKET>` tokens do not appear in fences (I-4: one reached a pushed commit message).
- **III.2 (M) Fences deliver; backticks name.** Every runnable command ships in its own fenced block, one logical step per block, machine + shell stated (Kron/PowerShell · Kron/cmd · MacBook/zsh), expected output stated after. Inline backticks NAME commands, paths and values. **A non-runnable artifact — a commit message, a log excerpt, a quoted line — is never fenced**, because under this clause a fence IS the instruction signal (I-5: a quoted commit message was executed). Ship checks before any fence leaves: last-token integrity eyeball; grep patterns verified against a live excerpt with explicit case handling; array parameters via `-Command`, never `-File`; any probe that can block carries its own timeout; `git log` / `git show` carry `--no-pager`.
- **III.3 (M) Gating stated; chains resume at the failed link.** Multi-step sequences say which later steps are gated on which earlier outputs. A failed `&&` chain resumes at the FAILED link; completed links are not idempotent and re-running them misreports position.
- **III.4 (M) Destinations are verified-or-created; a path is identity.** A deliverable's destination is verified to exist, or created, in the same instruction set — `find` before clone, `ls` before merge — and stated explicitly. A path guessed from a description is a stale-recollection error in disguise (I-3: the ledger was assumed at repo root; it lives at `docs/bridge-v2/`).
- **III.5 (M) Silence is declared.** Any step >2 min without output ships with duration and profile ("~5 min, one line every 15 s" · "silent ~40 min, one line at the end").
- **III.6 (M) Steps are idempotent or state their preconditions.** Operator state can move between the message and the execution (I-6: a staging folder was deleted in between). `mkdir -p`, `cp` over, `rm -f`, `(N)` globs — constructions that succeed identically on the second run are preferred; where impossible, the precondition is stated.
- **III.7 (M) Empty substitutions fail loud.** Any `$(...)` feeding a path, target or identity is guarded: an empty result must error, not degrade. `dirname ""` evaluates to `.` and relocates a command instead of stopping it (I-1).
- **III.8 (E) Dry-run before ship.** Anchors and self-checks simulated container-side against reconstructed target text; self-check constants computed, never head-counted. Advisory: it produces no artifact the operator sees.

## IV. THE RECORD IS APPEND-ONLY, SYNCHRONIZED, AND CORRECTED FORWARD

The ledger, session-state and sealed docs are one record on four rails. Nothing prior is edited; everything later cites what it changes; every rail is brought to the same sha in the same commit.

- **IV.1 (M) One `cat >>`.** An append merges as a single concatenation onto the canonical copy. Prior blocks are never edited.
- **IV.2 (M) Shipped at both ends.** (i) The file is PRESENTED in the same message as its move-and-verify one-liner, no exempt class — a one-liner with no file is a null instruction, a file with no one-liner is unlanded. (ii) A deliverable that gates other work has its landing CONFIRMED (output pasted back, or the next session's opening question checks the landing) — never assumed from the downstream use.
- **IV.3 (M) Iterations get new names; VOID carries I.2's bar.** r2, r3… ; superseded cuts are declared VOID in conversation. Never two live copies of one filename. A VOID destroys an artifact's identity in the record and is never issued from absence alone when a sha pin exists — the pin converts "gone" into a testable claim (S14: a VOID reversed by a byte-identical recovery from the conversation rail).
- **IV.4 (M) Spec filenames carry the described artifact's version** (`NODE-CONTRACT-v1.0.8`, `BRIDGE-SPEC-v2.0.1.4`); the header carries the content revision; conversation deliverables additionally carry `-rN`. A versionless name defeats delete-then-add.
- **IV.5 (M) Mount sync rides the commit, by copy.** Every commit revising a current-revision doc carries a paired delete-then-add of that doc in the project files, stated in the commit message as `OUT <count> <sha>, IN <count> <sha>`. Sealed-doc revisions are delete-then-add, never side-by-side; the old revision lives in the laptop archive only. Staging is by COPY into a fresh single-purpose dated folder, shasummed, then opened — a `mv` from a working tree silently deletes committed files (S14). Deviation requires named operator authorization recorded in the ledger.
- **IV.6 (M) Verbatim invocations are append data.** Exact command lines are recorded. For tools whose results are pinned but flags are not: shell history → usage header → reconstruction, stating which lane.
- **IV.7 (M) Incidents get one append line:** what broke, cost, root cause, clause minted or cited.
- **IV.8 (M) Corrections cite the id they weaken.** A later entry that strengthens, narrows or retracts an earlier claim names the BL id and the clause it amends, so the correction is grep-discoverable from the original. Forward id references that turn out wrong (I-13: "BL-087 owes…" when BL-087 was taken) are corrected forward in the next entry, not force-pushed.
- **IV.9 (M) Append skeletons are not committed.** The append's cut sha rides the commit message beside the OUT/IN pins (`APPEND <count> <sha>`). Committed skeletons duplicate content and break I.3's directory-scoped reading (I-7).
- **IV.10 (M) Commit messages are record.** Pins in them are literal, never tokens. A pushed message with a defect is amended via `--force-with-lease` only while the commit is unreferenced (mechanics in Part E); otherwise corrected forward per IV.8.
- **IV.11 (M) Base units always.** Sompi-first; KAS/ZKAS only with the conversion shown. Chain display is truth over recorded prose. (Chain-tier content; retained here as the record-format half.)

## V. EXTERNAL ARTIFACTS PASS AN ADVERSARY

Anything that leaves the operation — issue, comment, PR, reply, post — is read by people who know things we do not and will not extend charity to claims we have not earned.

- **V.1 (M) One explicit pass under a named adversary before cut.** For upstream artifacts: *a maintainer who knows this codebase cold*. The output is a written list of the claims that reader would object to, even if the list is empty. Each listed claim is answered inline or deleted — never defended by tone.
- **V.2 (M) Claims about systems we did not observe are n=0** and are phrased as questions or omitted. A counterfactual about someone else's node is not evidence, however elegant (I-11: deleted from r2 rather than answered).
- **V.3 (M) Venue norms, then render-verify by the instrument that shows it.** Declarative first line; non-reopen / non-claim stated up front; paste-first reproduction; observed-vs-expected at the artifact level; offer of labor at the close. After posting, the rendering is verified via the API body (`GET …/comments/<id>`) — the logged-out HTML renders no timeline and reads as "comment missing" (I-14). The published text is pulled back from the API and pinned as its own revision; the draft stays the draft.

---

## Appendix A — Disposition of v1 (nothing dropped)

| v1 | v2 | Type | Note |
|---|---|---|---|
| 1a sha at cut | I.1 | M | |
| 1b one-liner + both amendments | IV.2 | M | amendments folded |
| 1c destination stated | III.4 | M | merged with 13a |
| 1d new names, VOID | IV.3 | M | 16-amendment's VOID bar folded |
| 1e spec filenames carry version | IV.4 | M | |
| 2a append sha-pinned | I.1 | M | |
| 2b one `cat >>` | IV.1 | M | |
| 2c dedup 0/1 | I.3 | M | file-scoped |
| 2d count + sha post-merge | I.1 | M | |
| 2e delete-then-add | IV.5 | M | + stage-by-copy amendment |
| 2f mount rides the commit | IV.5 | M | |
| 2g self-header readback | I.4 | M | |
| 3 no placeholders | III.1 | M | gains mechanism: `$(...)` |
| 4 local source ground truth | II.1 | M | compiler-arbiter half → Part E |
| 5 dry-run | III.8 | E | demoted |
| 6 verbatim invocations | IV.6 | M | |
| 7 declare silence | III.5 | M | |
| 8 sompi-first | IV.11 | M | record-format half stays; unit law → Part E |
| 9 incidents one line | IV.7 | M | |
| 10 tier law | preamble | — | |
| 11 runnable means fenced | III.2 | M | gains converse + `--no-pager` |
| 12 clocks measured | II.1 | M | special case |
| 13a verified-or-created | III.4 | M | |
| 13b gating stated | III.3 | M | |
| 13c resume at failed link | III.3 | M | |
| 13d up-to-date = last fetch | II.1 | M | special case |
| 14 commands name target | III.2 | M | ship checks folded |
| 15 count + sha at delete | I.2 | M | |
| 16 rails independent + amendment | II.2 | M | five rails; recollection named |
| 17 prior-work sweep | II.3 | E→M | gains output: stated base |
| — (new) | I.5 | M | rail spelling (0715e6d) |
| — (new) | II.4 | M | N and span inline |
| — (new) | II.5 | M | verification strength |
| — (new) | II.6 | M | repetition |
| — (new) | III.6 | M | idempotent or preconditioned |
| — (new) | III.7 | M | empty substitutions |
| — (new) | IV.8 | M | corrections cite the id |
| — (new) | IV.9 | M | skeletons not committed |
| — (new) | IV.10 | M | commit messages are record |
| — (new) | V.1–V.3 | M | external artifacts |

## Appendix B — Incident register (S20, 2026-09-02→03; BL-086, BL-092)

| # | Incident | Clause |
|---|---|---|
| I-1 | `dirname ""` → `.`; grpcurl ran from wrong cwd instead of failing | III.7 |
| I-2 | Ledger tip recalled as BL-080; rail read BL-085 | II.2 |
| I-3 | Ledger path guessed at repo root; real path `docs/bridge-v2/` | III.4 |
| I-4 | `<sha from step 3>` reached a pushed commit message; amended 7eba684→d288da5 | III.1, IV.10 |
| I-5 | Commit-message excerpt shipped in a fence; executed; parse error | III.2 |
| I-6 | Staging folder deleted between instruction and execution | III.6 |
| I-7 | Append skeleton committed, duplicating ledger content | IV.9, I.3 |
| I-8 | Sweep gate `missing=0` read as content identity; shas now unverifiable | II.5 |
| I-9 | `tipHashes − virtualParentHashes = unmergeable set`, n=1, asserted universally | II.4 |
| I-10 | "Zero new probes" — depth needs a `getBlock` per member | II.4 |
| I-11 | Counterfactual about upstream's six tips, n=0, in a public draft | V.2 |
| I-12 | Claim repeated ledger → addendum without added evidence | II.6 |
| I-13 | Ledger tip recalled as BL-086; rail read BL-091 (parallel session); "BL-087 owes…" pushed | II.2, IV.8 |
| I-14 | HTML fetch of #6 rendered no timeline; would have read as "comment missing" | V.3 |
| I-15 | **P1 brief carried as "unpasted, oldest live item" — closed since BL-059 (08-29).** Cited as evidence against amendment 1b-ii in the laws evaluation; that evidence point is RETRACTED. Recollection stale on state, not just counters. | II.2 |

Not caught by any clause and not law-addressable: a complete-feeling frame emits no signal that anything is missing. The trigger for a re-read under a new frame comes from outside the session's momentum. What the clauses do is make that re-read fast — N in the sentence, scope beside the gate, provenance in the commit — so the outside reader finds the weak joint without reconstructing the frame that produced it.

## Appendix C — Moved out of this tier

To Part E (chain/tool laws): the sompi unit law (v1 law 8, unit half); "the compiler is the arbiter" (v1 law 4, tool half); `--force-with-lease` amend mechanics (referenced by IV.10); the `getBlockDagInfo` persistence detector and its H8 acceptance criterion (BL-092).
