# Running a ZKas node

`kaspad` is the ZKas node. Binaries are on the
[Releases](https://github.com/firecash/zkas-rusty/releases) page; to compile instead see
[docs/BUILDING.md](BUILDING.md).

```bash
./kaspad --appdir=./zkas-node --rpclisten=127.0.0.1:16110 --utxoindex \
  --connect=185.147.157.125:16111 --connect=160.187.211.153:16111
```

The node syncs from the seed nodes you give it and follows the tip. Without `--connect` it
resolves the mainnet seeder `seed.zkas.info` and dials every address it returns; once connected it discovers
the rest of the network through gossip. Keep the RPC bound to `127.0.0.1`: it is a control
interface, not something to expose.

**Seed nodes (mainnet):** `185.147.157.125:16111` · `160.187.211.153:16111`

### Ports

The current mainnet runs on the `161xx` block:

| Purpose | Port |
|---|---|
| RPC (gRPC) | **16110** (bind to loopback) |
| p2p | **16111** |

Only outbound access to a peer's **p2p** port (`16111`) is required to sync; inbound p2p is
optional (it lets others sync from you). A ZKas node and a merged-mining Kaspa parent are
kept on separate port blocks so both run on one host with no overrides.

---

## Pruned, archival, or shielded-history — which node do you need?

This is the part people get wrong, because ZKas keeps **two** kinds of history — the public
block data and the shielded note history — and they are pruned independently.

| | Pruned (default) | `--shielded-history=on` (pruned) | `--archival` |
|---|---|---|---|
| Public block bodies below the pruning point | discarded | discarded | **kept** |
| Notes/history from **before** the node first synced | **not fetched** | **fetched + kept** | **fetched + kept** |
| Notes/history from the node's **first sync onward** | **all kept forever** | all kept forever | all kept forever |
| Can fully validate the chain & every spend | ✅ | ✅ | ✅ |
| Serves wallet balances complete **since first sync** | ✅ | ✅ | ✅ |
| Serves wallet balances complete **back to genesis** | ❌ | ✅ | ✅ |
| Serves an explorer old **public** blocks/txs | ❌ | ❌ | ✅ |
| Disk | light | light + note archive | **heavy** |

### 1. Pruned node — the default, and what most people should run

A pruned node discards public block **bodies** below the pruning point, but it keeps two
shielded things **forever**:

- the **shielded consensus state** — the note-commitment tree frontier and the nullifier
  set — everything needed to fully validate the chain and every future spend; and
- the **per-block shielded scan archive** — the compact note/coinbase records a wallet
  replays to recover its balance.

The pruner **never touches the scan archive or the nullifier set**. Pruning deletes block
bodies, UTXO state and acceptance data; the scan archive is deliberately retained (ZKas
diverges from upstream here — "the reason is user funds"), along with a compact chain
index so the records can still be enumerated in chain order after the blocks themselves are
gone. So **from the moment a node first syncs, it writes and keeps the notes of every block
it processes, and never prunes them.** A restore-from-seed against a plain pruned node
returns a **complete** balance for everything at or after that node's first sync — wallet
recovery does not depend on an archival node existing somewhere on the network.

The one gap is history from **before** the node ever synced. IBD seeds a fresh node with
only the aggregate shielded state (a frontier plus a nullifier MuHash), which reveal no
one's notes, so a pruned node has no per-note archive below its **initial** pruning point.
A wallet that needs notes older than the node's first sync reads a **silently partial**
balance there — the number looks final but is a lower bound. That window, and only that
window, is what `--shielded-history` backfills. A fresh non-archival node still syncs
genesis→tip to byte-identical state; archival is **not** required for validation or mining.

### 2. Pruned + `--shielded-history=on` — a light wallet-serving node

`--shielded-history=on|off` controls whether the node **fetches the shielded note history
below its pruning point** from peers during IBD.

```bash
./kaspad --appdir=./zkas-node --utxoindex --shielded-history=on \
  --connect=185.147.157.125:16111
```

With it on, the scan archive and chain index survive pruning, so this node serves wallets
**complete** history — full, correct balances — while still pruning the bulky public block
bodies. This is the right shape for a wallet backend (`zkas-walletd`) or a hosted wallet
that does not also need to serve an explorer.

Default: **on when `--archival` is set, off otherwise.** Add `--verify-shielded-history` to
run the verification pass over the transferred history.

### 3. `--archival` — keep everything (explorers, full history)

```bash
./kaspad --appdir=./zkas-node --utxoindex --archival \
  --rocksdb-preset=hdd --connect=185.147.157.125:16111
```

`--archival` stops the node from deleting **public** block data when the pruning point
advances, so it retains the complete block/transaction history — what a block explorer
needs. It also turns `--shielded-history` on by default, so an archival node serves both
public and shielded history in full.

Two caveats worth knowing:

- **Archival only retains from now forward.** Enabling it on an already-pruned node does
  not backfill the blocks that were pruned before — it keeps everything from this point on.
  For a complete archive you must enable it on a node syncing from genesis (or import a
  full-history snapshot).
- It is **heavy on disk**. On spinning disks add `--rocksdb-preset=hdd` (larger write
  buffers, BlobDB, aggressive cold-data compression, an I/O rate limiter). See
  [docs/archival.md](archival.md) for the full RocksDB tuning.

---

## Flags reference

| Flag (env) | What it does |
|---|---|
| `--appdir=<dir>` (`KASPAD_APPDIR`) | Data directory. |
| `--rpclisten=<ip:port>` | Bind the gRPC RPC. Keep on `127.0.0.1`. |
| `--connect=<ip:port>` | Connect **only** to these peers (repeatable). Needed to bootstrap. |
| `--addpeer=<ip:port>` | Add a persistent peer but still discover others (repeatable). |
| `--utxoindex` (`KASPAD_UTXOINDEX`) | Build the UTXO index (needed for some RPCs and address queries). |
| `--archival` (`KASPAD_ARCHIVAL`) | Retain old public block data past the pruning point. Heavy disk. |
| `--shielded-history=on\|off` | Fetch shielded note history below the pruning point. Default: on with `--archival`, else off. |
| `--verify-shielded-history` | Verify the transferred shielded history during IBD. |
| `--rocksdb-preset=default\|hdd` | Storage tuning; `hdd` for archival nodes on spinning disks. |
| `--ram-scale=<f>` | Scale in-memory caches (e.g. `2.0` on a large box). |

Run `./kaspad --help` for the complete list.

---

## Recovery basics

- **Node won't start / corrupt DB:** stop the node, move the appdir aside, resync from a
  peer. A pruned node resyncs quickly; an archival node is slow — snapshot it instead of
  resyncing where possible.
- **Wallet balances look low for old coins after a resync:** the node is pruned and was not
  given `--shielded-history=on` (or `--archival`), so it lacks note history from *before it
  first synced*. Coins received since its first sync are complete; older ones read as a
  lower bound. Re-run with shielded history enabled (or point the wallet at a node that has
  it) and let the wallet rescan.
- **A plain pruned node is fine for wallets whose coins are all newer than the node's first
  sync** — it keeps every note from that point on. Only balances reaching below its initial
  pruning point are partial.
