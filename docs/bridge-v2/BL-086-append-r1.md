
**BL-086 · 2026-09-02 · upstream/nodes — #6 closed CONFIRMED; the class was
6× bigger; Kron measured unhealed; and the field we should have read was in
the response all along**
Upstream closed `firecash/zkas-rusty#6` with a live confirmation: at the
first pruning-point advancement after their deploy (2026-09-01 11:41 UTC)
the archival cleanup fired on both public archival nodes and removed the
stranded tip **plus five more unmergeable side-branch tips** that had
accumulated unnoticed — `pruned 6 unmergeable side-branch tips`. Fix
`9a464d51` is on main, ships in the next tagged release; an upgraded
archival node self-heals at its first pruning-point advancement (~once per
finality interval, ≈12h at 1 BPS), no resync. Our report was explicitly
single-vantage ("we cannot say whether other nodes carry this tip"); the
close supplies the population. Accumulation is a steady-state property of
archival mode, not one 07-31 branch — `e8dc1a03…` was the one visible at
index 0, not the class.
**KRON IS UNHEALED — MEASURED, not inferred.** Probe 2026-09-02 from
`/Users/pearsonmw/zkas-lab/proto` (protowire-protos.zip pinned
`3f2d2531…62a76` at run — same proto set as the A3 probes, so directly
comparable): `tipHashes[0]` is still
`e8dc1a034c0cfd992c295703d775779fffe2ab467d9c7c7130c51b918555c12e`,
`virtualDaaScore` 3,287,534, depth behind virtual **2,868,907 ≈ 33.2 days**
@ 1 BPS. DAA advanced **116,939 (~32.5 h of chain)** since the 08-31
re-check at 3,170,595 and the squatter did not move. v1.0.6 predates
`9a464d51`; version ordering held, and the inference is now a reading.
**THE DISCRIMINATOR WAS AT THE API SURFACE THE WHOLE TIME.** The same
response carries `tipHashes` (3 entries) and `virtualParentHashes` (2) —
and the stranded block is in the former and NOT the latter:
`tipHashes` = e8dc1a03…, 2dec178a…, d42551f4… · `virtualParentHashes` =
2dec178a…, d42551f4… · `sink` = 2dec178a…. The node already knew the tip was
unmergeable and already excluded it from the list that matters. We anchored
on the wrong field — not on a field the node could not disambiguate.
Three consequences: (1) our question 2 has a better answer than a note about
`tipHashes` ordering — ordering is real but secondary, `tipHashes` is simply
NOT the anchor list; `sink` is the selected chain tip and is a single
specified value. `None` remains the v2.0.1.6 fix (nothing here argues for
re-anchoring), with `sink` named in a patch comment so the next reader does
not re-derive it. (2) **Strandedness is a one-line detector**:
`tipHashes` − `virtualParentHashes` = the unmergeable set, computable from a
call the bridge already makes on the 30s tick — a candidate metric, not a
520-sample investigation. (3) **The 08-30 WATCH item closes arithmetically
and was never a second phenomenon**: current `difficulty` 1.4994e16, so
2×d_z ≈ 2.999e16 against the frozen 1.676882337221918e15 = **17.9×** — the
"bimodal ~18×" oscillation is exactly the two anchors' difficulty eras, and
A3's root cause covers it with no residual.
Unchanged: v2.0.1.6 holds priority. The bridge fix is the ONLY remediation
on our rail until a tagged node release ships and we cut over, and the KAS
leg at `kaspaapi.rs:511` is a different codebase entirely — upstream's fix
touches nothing there, so that `first()` anchor stays live, masked only by
10-BPS merge speed. **NEW GATE**: watch for the tagged release carrying
`9a464d51`; the post-cutover acceptance criterion is free — the stranded
tips should leave `tipHashes` within one finality interval (~12h) with NO
resync, which is a cleaner check than anything in NODE-CUTOVER r1.
Corrections, mine, both in this sitting: (a) the first probe never reached
the node — `find ~/zkas` returned empty, `dirname ""` evaluates to `.`, so
`cd .` SUCCEEDED and grpcurl ran from the operator's current directory and
failed on a missing proto. An empty command substitution inside `dirname`
degrades to a valid path instead of erroring; any find-then-cd one-liner
needs the empty case guarded or it silently relocates the command. (b) I
carried the ledger tip as BL-080 while the mount read BL-085 — a five-entry
stale self-pin, corrected by reading the rail instead of the summary.
**Lesson:** read the WHOLE response, not the field you came for. A 46-hour
sampler, a 520-sample distribution, a source read at the running commit, and
a public issue all sat downstream of a single JSON body that contained its
own disambiguator two fields below the one we parsed — and the cheapest
instrument in the entire A3 chain was the one nobody ran: print the response
and look at it.
