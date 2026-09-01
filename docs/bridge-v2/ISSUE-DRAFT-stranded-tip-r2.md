# A month-old side-branch tip persists in `getBlockDagInfo.tipHashes` (archival node); anchored hashrate estimation on it is faithful but pathological

On a mainnet archival zkas-node, `getBlockDagInfo` has returned the same block
as `tipHashes[0]` across every observation in a 46-hour instrumented window
(and, by gauge history, since at least 2026-08-27). The block is a side-branch
tip from **2026-07-31** — ~2.74M blocks behind virtual at time of writing, far
beyond the merge-depth bound — so it can never be merged, and on a non-pruning
node it appears it will never leave the tip set. `estimateNetworkHashesPerSecond`
anchored on it returns a bit-identical value from that block's difficulty era,
which is *correct* behavior for an anchored estimate; the surprise is the
anchor's permanence and its stable position at index 0.

## Environment

- zkas-node **v1.0.6** (win64 release binary), Windows 11 Pro
- Mainnet, **archival** (`--shielded-history=on`, no pruning)
- Observed via gRPC (`:16810`), protowire protos from the rusty-kaspa base

## The stranded tip

| field | value |
|---|---|
| hash | `e8dc1a034c0cfd992c295703d775779fffe2ab467d9c7c7130c51b918555c12e` |
| header timestamp | 2026-07-31 11:26:54.799 UTC |
| daaScore | 418,627 |
| blueScore | 415,031 |
| virtual daaScore at observation (2026-08-31 21:59 EDT) | 3,154,311 |
| depth behind virtual | 2,735,684 blocks ≈ 31.7 days @ 1 BPS |

Depth-vs-wall-clock cross-check: the block's header age (31.4 days) matches its
DAA-depth age (31.7 days) — consistent with a branch that stranded at birth.
Persistence re-check ~4.6 h after the observations above: `getBlockDagInfo` at
virtualDaaScore 3,170,595 returns the same hash, still at index 0.

## Observed data

A 5-minute sampler recorded the node's bridge-reported anchored estimate for
46 hours (520 samples):

- **496/520 (95.4%)**: bit-identical `1676882337221918` H/s — the difficulty
  era of the stranded anchor (d ≈ 8.4e14 epoch)
- **24/520**: live values 2.88–3.14e16 H/s, i.e. ≈ 2 × current difficulty,
  exactly as expected at 1 BPS — these occurred when a fresh tip transiently
  occupied index 0
- **0/520** anywhere between the two states

## Reproduction (copy-paste, ~2 minutes)

Two `getBlockDagInfo` calls 60 s apart: `tipHashes[0]` is identical both times
while `virtualDaaScore` advances ~60 and the *other* tips churn:

```
grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto \
  -d '{"getBlockDagInfoRequest":{}}' NODE:16810 protowire.RPC/MessageStream
```

Anchored estimate on that hash returns the constant, on demand:

```
grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto \
  -d '{"estimateNetworkHashesPerSecondRequest":{"windowSize":1000,"startHash":"e8dc1a034c0cfd992c295703d775779fffe2ab467d9c7c7130c51b918555c12e"}}' \
  NODE:16810 protowire.RPC/MessageStream
```

Unanchored (virtual) estimate returns live, changing, physically-consistent
values (observed 3.101e16 then 3.118e16, 90 s apart):

```
grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto \
  -d '{"estimateNetworkHashesPerSecondRequest":{"windowSize":1000}}' \
  NODE:16810 protowire.RPC/MessageStream
```

## What is NOT claimed

- The estimator is **faithful to its anchor** (verified against the
  `Some(start_hash)` path in consensus); no estimator bug and no consensus
  fault is demonstrated.
- Our own downstream tooling anchored estimates on `tipHashes.first()` — that
  was our bug, fixed on our side; it is how the condition was noticed, not
  part of the report.
- Single-vantage observation: we cannot say whether other nodes carry this
  tip. (Provenance unknown; it predates our local block records.)

## Questions

1. Is indefinite tip-set retention of a beyond-merge-depth side branch the
   intended behavior on an archival node — i.e., is tip eviction coupled to
   pruning by design, or should unmergeable tips age out independently?
2. Is `tipHashes` ordering specified anywhere (insertion order? stable?)? Its
   stability is what makes `first()` an attractive-nuisance anchor for API
   consumers; a doc note may be worth more than any code change.
3. If useful, happy to provide the full header, run further probes from this
   vantage, or turn the anchor-semantics note into a docs PR.
