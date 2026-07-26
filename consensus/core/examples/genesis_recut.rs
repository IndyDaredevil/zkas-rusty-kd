//! Genesis re-cut helper: recompute the mainnet genesis coinbase payload, merkle
//! root, and block hash for a new Bitcoin-anchored fair-launch stamp.
//!
//! Reads from env:
//!   GENESIS_ANCHOR  ASCII fair-launch string, e.g. "zkas-mainnet btc#959697 <64-hex>"
//!   GENESIS_TS      genesis timestamp in milliseconds
//!   GENESIS_BITS    genesis difficulty bits, hex e.g. 0x1b02d093
//!
//! Prints three lines (hex, no separators) for the driver script to patch back:
//!   PAYLOAD <hex>   the full coinbase payload (fixed 52-byte prefix + anchor)
//!   MERKLE  <hex>   hash_merkle_root
//!   HASH    <hex>   genesis block hash
//!
//! The 52-byte prefix (blue score, subsidy, shielded-state root, script version,
//! varint, OP-FALSE) is taken verbatim from the current GENESIS constant, so only
//! the trailing anchor, the timestamp and the bits vary between cuts.

use kaspa_consensus_core::block::Block;
use kaspa_consensus_core::config::genesis::GENESIS;
use kaspa_consensus_core::merkle::calc_hash_merkle_root;

const PREFIX_LEN: usize = 52; // 8 blue + 8 subsidy + 32 shielded-root + 2 ver + 1 varint + 1 OP-FALSE

fn main() {
    let anchor = std::env::var("GENESIS_ANCHOR").expect("GENESIS_ANCHOR env required");
    let ts: u64 = std::env::var("GENESIS_TS").expect("GENESIS_TS env required").trim().parse().expect("GENESIS_TS must be u64 ms");
    let bits_s = std::env::var("GENESIS_BITS").expect("GENESIS_BITS env required");
    let bits = u32::from_str_radix(bits_s.trim().trim_start_matches("0x"), 16).expect("GENESIS_BITS must be hex");

    // Fixed prefix from the live constant, then the ASCII anchor.
    let mut payload = GENESIS.coinbase_payload[..PREFIX_LEN].to_vec();
    payload.extend_from_slice(anchor.as_bytes());
    let payload: &'static [u8] = Box::leak(payload.into_boxed_slice());

    let mut g = GENESIS.clone();
    g.timestamp = ts;
    g.bits = bits;
    g.coinbase_payload = payload;

    // Merkle root over the (single) coinbase tx, then the block hash over the header
    // that commits to that merkle root — exactly as `test_genesis_hashes` verifies.
    let merkle = calc_hash_merkle_root(g.build_genesis_transactions().iter());
    g.hash_merkle_root = merkle;
    let blk: Block = (&g).into();
    let hash = blk.hash();

    let hx = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    println!("PAYLOAD {}", hx(payload));
    println!("MERKLE {}", hx(&merkle.as_bytes()));
    println!("HASH {}", hx(&hash.as_bytes()));
}
