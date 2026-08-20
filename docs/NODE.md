# Running a ZKas node

`kaspad` is the ZKas node. Binaries are on the
[Releases](https://github.com/firecash/zkas-rusty/releases) page; to compile instead see
[docs/BUILDING.md](BUILDING.md).

```bash
./kaspad --appdir=./zkas-node --rpclisten=127.0.0.1:16110 --utxoindex \
  --connect=<seed-node-ip>:16111
```

The node syncs from the peer(s) you give it and follows the tip. ZKas mainnet ships no DNS
seeders yet, so a fresh node needs at least one `--connect`/`--addpeer` bootstrap peer to
start — get a current seed node from the community/Discord — after which it discovers the
rest of the network through gossip. Keep the RPC bound to `127.0.0.1`: it is a control
interface, not something to expose.

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
| Shielded note history below the pruning point | **not fetched** | **fetched + kept** | **fetched + kept** |
| Can fully validate the chain & every spend | ✅ | ✅ | ✅ |
| Serves wallets **complete** historical balances | ❌ (partial) | ✅ | ✅ |
| Serves an explorer old **public** blocks/txs | ❌ | ❌ | ✅ |
| Disk | light | light + note archive | **heavy** |

### 1. Pruned node — the default, and what most people should run

A pruned node discards public block bodies below the pruning point but **always keeps the
shielded consensus state** — the note-commitment tree frontier and the nullifier set. That
state is seeded at the pruning point during IBD (as a frontier plus a nullifier MuHash,
which are aggregates and reveal no one's notes), and it is all a node needs to **fully
validate the chain and every future spend**. A fresh, non-archival node syncs genesis→tip
and reaches byte-identical state; archival is **not** required for validation or mining.

The one thing it cannot do: because IBD transferred only the aggregate shielded state, a
pruned node can serve a wallet its note history **only from the pruning point forward**.
Balances that depend on older notes read as **silently partial** — the number looks
final but is a lower bound. Fine for a validating/mining node; not fine for one that
answers wallet queries.

### 2. Pruned + `--shielded-history=on` — a light wallet-serving node

`--shielded-history=on|off` controls whether the node **fetches the shielded note history
below its pruning point** from peers during IBD.

```bash
./kaspad --appdir=./zkas-node --utxoindex --shielded-history=on \
  --connect=<seed-node-ip>:16111
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
  --rocksdb-preset=hdd --connect=<seed-node-ip>:16111
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
- **Wallet balances look low after a resync:** the node is pruned and was not given
  `--shielded-history=on` (or `--archival`), so it only serves history from its pruning
  point. Re-run it with shielded history enabled and let the wallet rescan.
- **Never point a wallet daemon at a plain pruned node and trust old balances** — see the
  partial-balance note above.
