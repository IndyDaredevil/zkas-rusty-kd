===============================================================================
 ZKas — release binaries
===============================================================================

This archive contains statically-linked ZKas binaries. ZKas is a
shielded-by-default (Orchard / Halo 2) fork of rusty-kaspa; kHeavyHash PoW,
1 block/second, ticker $ZKAS. 1 ZKAS = 100,000,000 sompi.

-------------------------------------------------------------------------------
 What's in here
-------------------------------------------------------------------------------
  kaspad           ZKas full node. Run with --utxoindex.
  zkas-miner       Built-in CPU miner (kHeavyHash). Enough to bootstrap /
                   solo-mine; for real hashrate use an ASIC or GPU (see below).
  shielded-pay     CLI shielded wallet: derive a zkas: address, check
                   balance, send private payments, and sign/verify address
                   ownership. Offline-capable for address + sign/verify.
  zkas-walletd     Local wallet daemon (REST) that powers the web/mobile wallet.
  zkas-api         Explorer / network-stats API server.
  stratum-bridge   Stratum bridge for pointing ASICs/pools at a ZKas node.

-------------------------------------------------------------------------------
 GPU / ASIC mining
-------------------------------------------------------------------------------
There is NO bundled GPU miner. ZKas's proof-of-work is kHeavyHash,
BYTE-IDENTICAL to Kaspa, so any existing Kaspa kHeavyHash miner works unchanged:

  - GPU:  bzminer, lolMiner, Rigel, etc. — point them at a ZKas node's
          stratum (or the pool at pool.zkas.info) with a
          zkas: address as the username.
  - ASIC: IceRiver / Bitmain / Goldshell kHeavyHash units work as-is.

The bundled zkas-miner is CPU-only and intended for bootstrapping or solo
low-difficulty mining, not competitive hashrate.

-------------------------------------------------------------------------------
 Solo merged mining (ZKas + Kaspa)
-------------------------------------------------------------------------------
The release bridge connects to the ZKas node through kaspad_address. Supplying
both merged fields enables the Kaspa parent connection; no separate enable flag
is required:

  kaspad_address: "127.0.0.1:16110"
  merged_kaspa_address: "127.0.0.1:17110"
  merged_kaspa_pay_address: "kaspa:YOUR_KASPA_ADDRESS"
  stratum_port: "0.0.0.0:5555"
  min_share_diff: 8192

Run:

  stratum-bridge.exe --node-mode external --config config.yaml

Configure the ASIC/miner with `zkas:YOUR_ZKAS_ADDRESS` as its Stratum username.
ZKas rewards pay that username directly; Kaspa parent rewards pay
merged_kaspa_pay_address. Startup must log both the ZKas endpoint and
`Real merged mining ENABLED` with the Kaspa endpoint. If it does not, stop and
check that the archive contains the AuxPoW-capable bridge.

-------------------------------------------------------------------------------
 Wallets
-------------------------------------------------------------------------------
  - CLI:    shielded-pay  (in this archive)
  - Web:    https://wallet.zkas.info   (also has on-device "Local" tools)
  - Paper:  https://zkas.info/paper-wallet.html  — an OFFLINE,
            single-file cold-storage wallet. Save it, go offline, generate.
            Source: github.com/firecash/zkas-paper-wallet

Quick start:
  ./kaspad --utxoindex
  ./shielded-pay address --seed-byte 1 --network mainnet
  ./zkas-miner -s 127.0.0.1:16810 -a zkas:<your-address> -t 4
===============================================================================
