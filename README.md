# ZKas — `zkas-rusty`

**Private-by-default money at Kaspa speed.** A fork of
[rusty-kaspa](https://github.com/kaspanet/rusty-kaspa) where every balance and transfer is
**shielded by default** (Zcash **Orchard** notes, Halo 2 proofs, no trusted setup), keeping
Kaspa's BlockDAG confirmation and **kHeavyHash** proof-of-work.

This repo is the core node and tooling: the node, the miner, the wallet daemon, the
explorer API.

## What's different from Kaspa

| | ZKas | Kaspa |
|---|---|---|
| Privacy | **Shielded by default** (Orchard) | Transparent |
| Consensus | GHOSTDAG BlockDAG, ~1 block/s | 10 blocks/s |
| PoW | **kHeavyHash** (byte-identical to Kaspa) | kHeavyHash |
| Merged mining | **Yes** — AuxPoW dual-acceptance with Kaspa | — |
| Emission | 60 ZKAS start, 3-month halving, two-step perpetual tail (6 → 0.6 ZKAS/block) | fixed cap |

- **Shielded state:** coinbase rewards and transfers enter a mandatory Orchard pool; the
  only public quantity is the fee a spender exposes to the miner. A shielded state root
  (anchor + nullifier accumulator + turnstile) is committed in the coinbase.
- **Merged mining (Option-2 dual acceptance):** a block is valid if **either** its native
  kHeavyHash clears the target **or** it carries an `AuxPoW` proof — a parent kHeavyHash
  block (e.g. a Kaspa block) whose coinbase commits to the ZKas block hash. Native mining
  stays the backbone; merged mining adds security at zero marginal cost to Kaspa miners.
  See `consensus/core/src/auxpow.rs` and `consensus/pow/src/auxpow.rs`.
- **Tokenomics:** 60 ZKAS initial reward (at 1 BPS), halving every 3 months, settling on a
  two-step perpetual tail: 6 ZKAS/block from ~month 10 through month 24, then a permanent
  0.6 ZKAS/block floor (~18.9M ZKAS/year, ~2.2% at onset decaying toward ~1%). No fixed
  supply cap.

## Run a node

Binaries: [Releases](https://github.com/firecash/zkas-rusty/releases) · to compile instead,
see **[docs/BUILDING.md](docs/BUILDING.md)**.

```bash
./kaspad --appdir=./zkas-node --rpclisten=127.0.0.1:16110 --utxoindex \
  --connect=185.147.157.125:16111 --connect=160.187.211.153:16111
```

It syncs from these seed nodes and follows the tip. ZKas mainnet has no DNS seeders yet, so
a fresh node bootstraps from a known peer; once connected it discovers the rest of the
network through gossip. Only outbound access to a peer's p2p **16111** is needed; keep the
RPC on **16110** bound to loopback.

**Node types — pruned (default), archival, and shielded-history — and every flag are
documented in [docs/NODE.md](docs/NODE.md).** Which one you want depends on whether the
node just validates/mines, serves wallets complete balances, or backs an explorer.

## Run a wallet

`zkas-walletd` is the shielded wallet daemon — REST, token-scoped, and what the web and
mobile wallets talk to. It holds viewing keys only unless you tell it otherwise.

```bash
./zkas-walletd --network mainnet --rpc-server 127.0.0.1:16810 --listen 127.0.0.1:8501
```

**[docs/WALLETD.md](docs/WALLETD.md)** is the full guide: endpoints, sizing, tuning, and
integration playbooks for exchanges and mining pools. Read §0 first — payments are sized in
**notes**, not coins, and that one fact explains most of the surprises.

Non-custodial signing (the seed never leaves the device):
**[docs/NON_CUSTODIAL_WALLET.md](docs/NON_CUSTODIAL_WALLET.md)**.

## Mine

- **Pool (ASICs):** point your miner at **mining-pool.zkas.info**. No node needed.
- **Solo:** with a synced node, mine to your `zkas:` address:
  ```bash
  ./zkas-miner -s 127.0.0.1:16810 -a zkas:<your-address> -t 4
  ```

## Binaries

| Crate | Binary | Role |
|---|---|---|
| `kaspad` | `kaspad` | the node (gRPC :16810, p2p :16811) |
| `miner` | `zkas-miner` | CPU miner (native + `--merged` AuxPoW) |
| `zkas-walletd` | `zkas-walletd` | shielded wallet daemon |
| `zkas-api` | `zkas-api` | explorer REST backend |

Companion repos: **zkas-pool** (stratum bridge), **zkas-explorer**, **zkas-wallet**,
**zkas-website**.

## Docs

| | |
|---|---|
| [docs/BUILDING.md](docs/BUILDING.md) | prerequisites, compiling, tests |
| [docs/WALLETD.md](docs/WALLETD.md) | wallet daemon: API, tuning, exchange & pool integration |
| [docs/NODE.md](docs/NODE.md) | running a node: pruned vs archival, shielded history, ports, flags, recovery |
| [docs/CLI-WALLET.md](docs/CLI-WALLET.md) | `shielded-pay` command-line wallet |
| [docs/archival.md](docs/archival.md) | archival nodes and history retention |

Explorer: https://explorer.zkas.info

## Configuration

`merged_mining_activation` and every tokenomics constant live in
`consensus/core/src/config/params.rs`. Genesis, address prefixes (`zkas:` / `zkastest:`)
and BPS are compiled in — changing them requires a rebuild and a fresh chain.

## License

Inherits rusty-kaspa's ISC license. See [`LICENSE`](./LICENSE).
