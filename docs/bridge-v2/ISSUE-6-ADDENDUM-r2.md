Follow-up on the docs half of question 2 — **not a reopen**. `9a464d51` is confirmed, and this is an observation from an un-upgraded vantage, which is the only place the condition is still visible.

Probing a **v1.0.6** archival node (predates the fix, so no cleanup has fired and the tip from the original report is still present), the `getBlockDagInfo` response carries its own discriminator:

```
grpcurl -plaintext -max-time 10 -import-path . -proto messages.proto \
  -d '{"getBlockDagInfoRequest":{}}' NODE:16810 protowire.RPC/MessageStream
```

```json
{
  "getBlockDagInfoResponse": {
    "networkName": "zkas-mainnet",
    "blockCount": "3287534",
    "headerCount": "3287534",
    "tipHashes": [
      "e8dc1a034c0cfd992c295703d775779fffe2ab467d9c7c7130c51b918555c12e",
      "2dec178ab3dff4eaec7d8e566f33f9af4ac8da1d1033ac6ff08525d9a5fd59e3",
      "d42551f41ec8bcbbe5917d0c72515a7f8f11dd9cc59bb1127346d7358e66f4e0"
    ],
    "difficulty": 1.4994495722580952e+16,
    "pastMedianTime": "1788361051079",
    "virtualParentHashes": [
      "2dec178ab3dff4eaec7d8e566f33f9af4ac8da1d1033ac6ff08525d9a5fd59e3",
      "d42551f41ec8bcbbe5917d0c72515a7f8f11dd9cc59bb1127346d7358e66f4e0"
    ],
    "pruningPointHash": "e66bc1df3cf6fb137e2200410c2127e044d8e23c03e4b4ebd7eddc7757b5e3c3",
    "virtualDaaScore": "3287534",
    "sink": "2dec178ab3dff4eaec7d8e566f33f9af4ac8da1d1033ac6ff08525d9a5fd59e3"
  }
}
```

`tipHashes` has three entries; `virtualParentHashes` has two. The stranded tip is in the former and **not** in the latter — the node has already classified it as unmergeable and already excludes it from the list that matters. `sink` is a single, specified value.

That reframes question 2 in a way that may be more useful than the ordering note I originally asked about: `tipHashes` ordering being unspecified is real, but secondary — `tipHashes` is simply not the anchor list. A consumer wanting an anchored `estimateNetworkHashesPerSecond` wants virtual (no `startHash` at all), or `sink` if an explicit anchor is required.

**Suggested docs note:** consumers should anchor on virtual or `sink`, not `tipHashes[0]`.

A derived diagnostic, tested only as far as stated: sampled the same node 20 times at 15 s cadence (274 DAA span, tip count ranging 2–5). `tipHashes − virtualParentHashes` was exactly the one stranded tip in 20/20 frames; every non-stranded tip was in the virtual parent set on every read, including three frames at 5 tips / 4 parents.

<details>
<summary>20-frame sample (virtualDaaScore, tips, virtual parents, difference)</summary>

```
3334551 tips=2 vpar=1 diff=e8dc1a03
3334569 tips=5 vpar=4 diff=e8dc1a03
3334586 tips=4 vpar=3 diff=e8dc1a03
3334603 tips=3 vpar=2 diff=e8dc1a03
3334613 tips=3 vpar=2 diff=e8dc1a03
3334625 tips=4 vpar=3 diff=e8dc1a03
3334644 tips=3 vpar=2 diff=e8dc1a03
3334651 tips=2 vpar=1 diff=e8dc1a03
3334662 tips=4 vpar=3 diff=e8dc1a03
3334676 tips=5 vpar=4 diff=e8dc1a03
3334689 tips=2 vpar=1 diff=e8dc1a03
3334699 tips=3 vpar=2 diff=e8dc1a03
3334710 tips=3 vpar=2 diff=e8dc1a03
3334727 tips=3 vpar=2 diff=e8dc1a03
3334739 tips=4 vpar=3 diff=e8dc1a03
3334763 tips=4 vpar=3 diff=e8dc1a03
3334782 tips=4 vpar=3 diff=e8dc1a03
3334791 tips=2 vpar=1 diff=e8dc1a03
3334811 tips=5 vpar=4 diff=e8dc1a03
3334825 tips=3 vpar=2 diff=e8dc1a03
```
</details>

So a tip that persists in that difference across K consecutive `getBlockDagInfo` reads is a reasonable stranded-tip indicator with no additional RPC. **Not tested, and where I'd expect it to need qualification:** tip counts above the virtual parent bound (never exceeded 5 here), transients shorter than the sampling interval, and higher-BPS networks. Persistence over a generous K covers the second; the first is a real limit on the difference set alone and is why K matters.

**Not claimed:** no bug in the estimator, no fault in `9a464d51`, and nothing here bears on the fix's correctness — an anchored estimate is faithful to its anchor, and the original defect was our consumer-side choice of `first()`. Documentation gap only.

Happy to turn the docs note into a PR against the RPC reference if that's the more useful form.
