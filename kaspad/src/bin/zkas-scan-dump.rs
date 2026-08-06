//! Dump / verify the shielded scan archive — the per-note wallet history.
//!
//! # Why this exists
//!
//! The scan archive is the ONLY copy of per-note history. IBD transfers
//! `PruningPointShieldedMetadata`, whose contents (frontier, nullifier MuHash, supply, burns)
//! are all *aggregates* — mathematically incapable of yielding anyone's notes. A freshly synced
//! node therefore writes scan records only for blocks it validates itself, i.e. pruning point
//! forward, and **cannot obtain older history from the network at all**.
//!
//! So this data lives only on nodes that were already running when those blocks were validated.
//! If those are lost, every wallet loses the ability to recover notes received before the
//! current pruning point. The chain would keep validating perfectly; the funds history would be
//! gone. That makes an off-box copy the single highest-value backup in the system — and at
//! ~232 MiB (measured: `ShieldedScanBlock`, 467,072 entries, 2.35% of a 10.3 GiB DB) it is
//! cheap to hold in several places.
//!
//! # Usage
//!
//! Opens the database READ-ONLY, so it is safe against a running node.
//!
//! ```text
//! zkas-scan-dump --db <appdir>/<net>/datadir/consensus/consensus-NNN --out history.bin
//! zkas-scan-dump --verify history.bin
//! ```
//!
//! Format: `ZKASSCAN1` magic, then per record `u32 len | 32B block hash | bincode(data)`,
//! then a trailer `u64 count | 32B blake3-style rolling digest` so a truncated or corrupted
//! file is detected rather than silently restored.

use kaspa_consensus::model::stores::selected_chain::{DbSelectedChainStore, SelectedChainStoreReader};
use kaspa_consensus::model::stores::shielded::{DbShieldedScanBlockStore, ShieldedScanBlockStoreReader};
use kaspa_database::prelude::{CachePolicy, ConnBuilder};
use kaspa_hashes::Hash;
use std::io::{BufReader, BufWriter, Read, Write};

const MAGIC: &[u8; 9] = b"ZKASSCAN1";

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

/// Order-sensitive rolling digest: detects truncation, reordering and any byte change.
fn mix(acc: &mut [u8; 32], bytes: &[u8]) {
    let mut h = blake2b_simd::Params::new().hash_length(32).to_state();
    h.update(acc);
    h.update(bytes);
    acc.copy_from_slice(h.finalize().as_bytes());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = arg("--verify") {
        let mut f = BufReader::new(std::fs::File::open(&path)?);
        let mut magic = [0u8; 9];
        f.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err("not a zkas scan dump".into());
        }
        let (mut count, mut acc, mut bytes) = (0u64, [0u8; 32], 0u64);
        loop {
            let mut lenb = [0u8; 4];
            if f.read_exact(&mut lenb).is_err() {
                return Err("file truncated before trailer".into());
            }
            let len = u32::from_le_bytes(lenb);
            if len == u32::MAX {
                break; // trailer marker
            }
            let mut rec = vec![0u8; len as usize];
            f.read_exact(&mut rec)?;
            mix(&mut acc, &rec);
            count += 1;
            bytes += len as u64;
        }
        let mut tc = [0u8; 8];
        f.read_exact(&mut tc)?;
        let mut td = [0u8; 32];
        f.read_exact(&mut td)?;
        let want_count = u64::from_le_bytes(tc);
        if want_count != count || td != acc {
            return Err(format!("CORRUPT: records {count} (want {want_count}), digest mismatch {}", td != acc).into());
        }
        println!("OK  {count} records, {:.1} MiB payload, digest {}", bytes as f64 / 1048576.0, faster_hex::hex_string(&acc));
        return Ok(());
    }

    let Some(db_path) = arg("--db") else {
        eprintln!("usage: zkas-scan-dump --db <consensus-NNN> --out <file> | --verify <file>");
        std::process::exit(2);
    };
    let out_path = arg("--out").ok_or("--out required")?;

    let db = ConnBuilder::default().with_db_path(db_path.into()).with_files_limit(128).build_readonly()?;
    let scans = DbShieldedScanBlockStore::new(db.clone(), CachePolicy::Empty);
    // The selected-chain index is deliberately retained below the retention root (see
    // pruning_processor: "the reason is user funds"), so it enumerates the FULL history even on
    // a pruned node — which is exactly the range that cannot be re-obtained from the network.
    let chain = DbSelectedChainStore::new(db, CachePolicy::Empty);
    let (tip_index, _) = chain.get_tip()?;

    let mut out = BufWriter::new(std::fs::File::create(&out_path)?);
    out.write_all(MAGIC)?;
    let (mut count, mut acc, mut bytes) = (0u64, [0u8; 32], 0u64);
    for i in 0..=tip_index {
        let Ok(block) = chain.get_by_index(i) else { continue };
        let Some(data) = scans.get(block).unwrap_or(None) else { continue };
        let mut rec: Vec<u8> = Vec::with_capacity(256);
        rec.extend_from_slice(&block.as_bytes());
        rec.extend_from_slice(&bincode::serialize(&data)?);
        out.write_all(&(rec.len() as u32).to_le_bytes())?;
        out.write_all(&rec)?;
        mix(&mut acc, &rec);
        count += 1;
        bytes += rec.len() as u64;
        if i % 50_000 == 0 {
            eprintln!("  {i}/{tip_index} chain blocks, {count} records");
        }
    }
    out.write_all(&u32::MAX.to_le_bytes())?;
    out.write_all(&count.to_le_bytes())?;
    out.write_all(&acc)?;
    out.flush()?;
    eprintln!("wrote {count} scan records, {:.1} MiB payload -> {out_path}", bytes as f64 / 1048576.0);
    eprintln!("digest {}", faster_hex::hex_string(&acc));
    let _: Hash = Hash::default();
    Ok(())
}
