//! Account for a node's on-disk consensus database, byte by byte, per store.
//!
//! # Why
//!
//! Every store in the consensus DB shares one RocksDB column family and is
//! separated only by a one-byte prefix (`kaspa_database::registry`). That makes
//! `du` useless for the question that actually matters when the DB grows: *which
//! store is it?* Guessing has been expensive — the shielded per-chain-block
//! snapshots (frontier, nullifier MuHash, supply, scan archive) are written once
//! per chain block and, unlike the block data around them, are never deleted by
//! the pruner, so a cost that looks negligible per block is the dominant term
//! after a few hundred thousand.
//!
//! This walks the whole keyspace once and reports, per prefix: entry count, key
//! bytes, value bytes, and mean value size. Those are *logical* bytes — the SST
//! files are compressed, so the total here reads a little above `du`; the ratios
//! between stores are what the tool is for.
//!
//! # Usage
//!
//! Opens the database READ-ONLY, so it is safe to point at a running node —
//! though on a busy node prefer a copy, since a full scan is IO-heavy.
//!
//! ```text
//! zkas-db-usage --db <appdir>/<network>/datadir/consensus/consensus-NNN [--top 25]
//! ```

use kaspa_consensus::model::stores::headers::{DbHeadersStore, HeaderStoreReader};
use kaspa_database::prelude::{CachePolicy, ConnBuilder};
use kaspa_database::registry::DatabaseStorePrefixes;
use kaspa_hashes::Hash;
use num_traits::FromPrimitive;
use rocksdb::{Direction, IteratorMode, ReadOptions};
use std::collections::BTreeMap;

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// Human-readable byte count.
fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 { format!("{bytes} B") } else { format!("{v:.1} {}", UNITS[unit]) }
}

/// Name a prefix byte via the registry enum itself, so this cannot drift from
/// it as stores are added. Anything unlisted is reported as `unknown(<byte>)`.
fn prefix_name(byte: u8) -> String {
    match DatabaseStorePrefixes::from_u8(byte) {
        Some(p) => format!("{p:?}"),
        None => format!("unknown({byte})"),
    }
}

#[derive(Default, Clone, Copy)]
struct Usage {
    entries: u64,
    key_bytes: u64,
    value_bytes: u64,
}

impl Usage {
    fn total(&self) -> u64 {
        self.key_bytes + self.value_bytes
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db_path) = arg("--db") else {
        eprintln!("usage: zkas-db-usage --db <consensus-NNN dir> [--top N]");
        eprintln!("       the database is opened read-only; a running node is fine (IO-heavy)");
        std::process::exit(2);
    };
    let top: usize = arg("--top").and_then(|s| s.parse().ok()).unwrap_or(40);
    let dump_prefix: Option<u8> = arg("--dump-prefix").and_then(|s| s.parse().ok());
    let compact = std::env::args().any(|a| a == "--compact");

    // Rewrite every SST under the *current* build's compression settings, so a
    // before/after `du` measures what the setting is actually worth on real data
    // instead of what a synthetic benchmark suggests. Opens READ-WRITE: point it
    // at a copy, never at a live datadir.
    if compact {
        let db = ConnBuilder::default().with_db_path(db_path.clone().into()).with_files_limit(128).build()?;
        eprintln!("compacting {db_path} (full range) ...");
        let t = std::time::Instant::now();
        db.compact_range::<&[u8], &[u8]>(None, None);
        eprintln!("compaction finished in {:.1}s", t.elapsed().as_secs_f64());
        return Ok(());
    }

    let db = ConnBuilder::default().with_db_path(db_path.clone().into()).with_files_limit(128).build_readonly()?;

    // A full scan should not evict the node's working set if this is pointed at a
    // live datadir, and we never need a consistent snapshot — approximate totals
    // are the entire point.
    let mut opts = ReadOptions::default();
    opts.fill_cache(false);

    let mut per_prefix: BTreeMap<u8, Usage> = BTreeMap::new();
    let mut tx_sizes: Vec<u64> = Vec::new();
    let mut header_sizes: Vec<u64> = Vec::new();
    let mut header_samples: Vec<Hash> = Vec::new();
    let tx_prefix = DatabaseStorePrefixes::BlockTransactions as u8;
    let header_prefix = DatabaseStorePrefixes::CompressedHeaders as u8;
    let mut scanned = 0u64;
    for item in db.iterator_opt(IteratorMode::From(&[0u8], Direction::Forward), opts) {
        let (key, value) = item?;
        let Some(&prefix) = key.first() else { continue };
        let e = per_prefix.entry(prefix).or_default();
        e.entries += 1;
        e.key_bytes += key.len() as u64;
        e.value_bytes += value.len() as u64;
        if prefix == tx_prefix {
            tx_sizes.push(value.len() as u64);
        } else if prefix == header_prefix {
            header_sizes.push(value.len() as u64);
            // Key layout is `prefix || hash`; sample sparsely so the anatomy pass
            // stays cheap on a large DB.
            if header_sizes.len() % 50 == 0 && key.len() >= 33 {
                if let Ok(bytes) = <[u8; 32]>::try_from(&key[key.len() - 32..]) {
                    header_samples.push(Hash::from_bytes(bytes));
                }
            }
        }
        scanned += 1;
        if scanned % 5_000_000 == 0 {
            eprintln!("  .. {scanned} entries scanned");
        }
    }

    // Anatomy of a header. `CompressedHeaders` is retained for the life of the
    // chain (pruning removes bodies, not headers), so whatever rides in a header
    // is paid for forever — which makes the merged-mining AuxPoW witness, a whole
    // parent Kaspa header plus its coinbase and Merkle branch, worth pricing
    // exactly rather than estimating.
    if !header_samples.is_empty() {
        let store = DbHeadersStore::new(db.clone(), CachePolicy::Empty, CachePolicy::Empty);
        let (mut with_aux, mut without_aux, mut n_aux, mut n) = (0u64, 0u64, 0u64, 0u64);
        for hash in header_samples.iter().take(2000) {
            let Ok(header) = store.get_header(*hash) else { continue };
            let full = borsh::to_vec(&*header).map(|v| v.len() as u64).unwrap_or(0);
            let mut stripped = (*header).clone();
            let had_aux = stripped.aux_pow.take().is_some();
            let bare = borsh::to_vec(&stripped).map(|v| v.len() as u64).unwrap_or(0);
            with_aux += full;
            without_aux += bare;
            n_aux += u64::from(had_aux);
            n += 1;
        }
        if n > 0 {
            println!(
                "header anatomy over {n} sampled headers: mean total {}, mean without aux_pow {}, aux_pow share {:.1}% ({} of {} carry one)",
                human(with_aux / n),
                human(without_aux / n),
                if with_aux > 0 { (with_aux - without_aux) as f64 * 100.0 / with_aux as f64 } else { 0.0 },
                n_aux,
                n
            );
        }
    }

    // Percentiles for the two stores that dominate on a shielded chain. A mean is
    // not enough to act on: a fat p50 means *every* block carries the cost
    // (structural — attack it in the format), while a fat tail over a thin p50
    // means a few real payloads (attack it with retention instead).
    for (label, samples) in [("BlockTransactions", &mut tx_sizes), ("CompressedHeaders", &mut header_sizes)] {
        if samples.is_empty() {
            continue;
        }
        samples.sort_unstable();
        let at = |q: f64| samples[((samples.len() - 1) as f64 * q) as usize];
        println!(
            "{label} value sizes: p50 {} p90 {} p99 {} max {} (n={})",
            human(at(0.50)),
            human(at(0.90)),
            human(at(0.99)),
            human(*samples.last().unwrap()),
            samples.len()
        );
    }
    println!();

    // Optional: dump raw values of one prefix so their real-world compressibility
    // can be measured out-of-band. RocksDB is opened with no explicit compression
    // config here, and a chain DB is mostly hashes (incompressible) mixed with
    // highly repetitive records (coinbase recipients, parent-header hashes) — the
    // mix decides whether a codec change is worth anything, and guessing is how
    // you waste a week on a config flag.
    if let Some(target) = dump_prefix {
        let mut out = std::fs::File::create(format!("/root/dump-{target}.bin"))?;
        let mut opts2 = ReadOptions::default();
        opts2.fill_cache(false);
        let mut written = 0usize;
        for item in db.iterator_opt(IteratorMode::From(&[target], Direction::Forward), opts2) {
            let (key, value) = item?;
            if key.first() != Some(&target) || written > 40_000_000 {
                break;
            }
            std::io::Write::write_all(&mut out, &value)?;
            written += value.len();
        }
        eprintln!("dumped {} bytes of prefix {target} to /root/dump-{target}.bin", written);
    }

    let grand_total: u64 = per_prefix.values().map(|u| u.total()).sum();
    let mut rows: Vec<(u8, Usage)> = per_prefix.into_iter().collect();
    rows.sort_by_key(|(_, u)| std::cmp::Reverse(u.total()));

    println!("db: {db_path}");
    println!("logical bytes (uncompressed keys+values): {}", human(grand_total));
    println!();
    println!("{:<32} {:>12} {:>12} {:>12} {:>10} {:>7}", "store", "entries", "keys", "values", "mean val", "share");
    println!("{}", "-".repeat(92));
    for (prefix, u) in rows.iter().take(top) {
        let mean = if u.entries > 0 { u.value_bytes / u.entries } else { 0 };
        let share = if grand_total > 0 { u.total() as f64 * 100.0 / grand_total as f64 } else { 0.0 };
        println!(
            "{:<32} {:>12} {:>12} {:>12} {:>10} {:>6.2}%",
            prefix_name(*prefix),
            u.entries,
            human(u.key_bytes),
            human(u.value_bytes),
            human(mean),
            share
        );
    }

    // The shielded stores are the ones the pruner does not touch; call out their
    // combined share explicitly, since that is the number that decides whether
    // retention work is worth doing.
    let shielded: Vec<u8> = [
        DatabaseStorePrefixes::ShieldedNullifiers,
        DatabaseStorePrefixes::ShieldedTreeFrontier,
        DatabaseStorePrefixes::ShieldedAnchors,
        DatabaseStorePrefixes::ShieldedSupply,
        DatabaseStorePrefixes::ShieldedNullifierDiffs,
        DatabaseStorePrefixes::ShieldedNullifierMuHash,
        DatabaseStorePrefixes::ShieldedScanBlock,
        DatabaseStorePrefixes::ShieldedBurns,
        DatabaseStorePrefixes::ShieldedAnchorProducers,
    ]
    .iter()
    .map(|p| *p as u8)
    .collect();
    let shielded_total: u64 = rows.iter().filter(|(p, _)| shielded.contains(p)).map(|(_, u)| u.total()).sum();
    println!();
    println!(
        "shielded stores combined: {} ({:.2}% of logical bytes)",
        human(shielded_total),
        if grand_total > 0 { shielded_total as f64 * 100.0 / grand_total as f64 } else { 0.0 }
    );
    Ok(())
}
