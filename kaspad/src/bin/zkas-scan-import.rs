//! Restore a shielded scan archive exported by `zkas-scan-export`.
//!
//! # Why
//!
//! A node that synced from a headers proof holds per-note history only from its pruning point
//! forward, so a wallet querying it sees a silently partial balance. This restores the missing
//! range from a verified export, without re-validating a single historical block.
//!
//! # Restoring records alone is NOT enough
//!
//! Measured: importing only the scan records left the node still answering
//! `cannot find header` for an old block. Wallets reach history through
//! `get_shielded_chain_range`, which resolves the start block via
//! `selected_chain_store.get_by_hash()`; with no index entry it returns `None` and the caller
//! falls back to the reachability path, which needs headers a pruned/fresh node does not have.
//! The chain index is the enumerator, and it must be restored too.
//!
//! # Rebasing
//!
//! `init_with_pruning_point` numbers a synced node from ITS OWN pruning point as index 0, while
//! the export is genesis-based. The two spaces collide, and there is no room below 0 for history.
//! So the import rebases: it finds the node's pruning-point block in the export, learns its
//! genesis-based index `P`, shifts every existing local entry up by `P`, then writes the export's
//! entries at their genesis-based indices. Afterwards the node's index space is the canonical
//! genesis-based one — identical to a node that had the history all along.
//!
//! # Safety
//!
//! The scan archive is **never read by consensus** — it only serves `GetShieldedBlocks` — so a
//! bad import cannot affect validation or fork the node. Records the node produced itself are
//! authoritative and are never overwritten. Run with the node STOPPED (RocksDB takes an
//! exclusive lock).
//!
//! ```text
//! zkas-scan-import --db <appdir>/<network>/datadir/consensus/consensus-NNN --in history.bin
//! ```

use kaspa_consensus::model::stores::selected_chain::{DbSelectedChainStore, SelectedChainStoreReader};
use kaspa_consensus::model::stores::shielded::{DbShieldedScanBlockStore, ShieldedScanBlockData, ShieldedScanBlockStoreReader};
use kaspa_database::prelude::{CachePolicy, ConnBuilder};
use kaspa_hashes::Hash;
use std::collections::HashMap;
use std::io::{BufReader, Read};

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

struct Record {
    index: u64,
    block: Hash,
    data: ShieldedScanBlockData,
}

fn read_export(path: &str) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    let mut r = BufReader::new(std::fs::File::open(path)?);
    let mut magic = [0u8; 9];
    r.read_exact(&mut magic)?;
    if &magic != b"ZKASHIST\x01" {
        return Err("not a ZKASHIST v1 export (re-export with the current zkas-scan-export)".into());
    }
    let mut out = Vec::new();
    loop {
        let mut idx = [0u8; 8];
        match r.read_exact(&mut idx) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let mut hb = [0u8; 32];
        r.read_exact(&mut hb)?;
        let mut lb = [0u8; 4];
        r.read_exact(&mut lb)?;
        let mut payload = vec![0u8; u32::from_le_bytes(lb) as usize];
        r.read_exact(&mut payload)?;
        out.push(Record { index: u64::from_le_bytes(idx), block: Hash::from_bytes(hb), data: bincode::deserialize(&payload)? });
    }
    Ok(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (Some(db_path), Some(in_path)) = (arg("--db"), arg("--in")) else {
        eprintln!("usage: zkas-scan-import --db <consensus-NNN dir> --in <history.bin>");
        eprintln!("       run with the node STOPPED");
        std::process::exit(2);
    };

    let records = read_export(&in_path)?;
    eprintln!(
        "export holds {} records, genesis-based indices {}..{}",
        records.len(),
        records[0].index,
        records[records.len() - 1].index
    );
    let by_hash: HashMap<Hash, u64> = records.iter().map(|r| (r.block, r.index)).collect();

    let db = ConnBuilder::default().with_db_path(db_path.clone().into()).with_files_limit(128).build()?;
    let scans = DbShieldedScanBlockStore::new(db.clone(), CachePolicy::Empty);
    let mut chain = DbSelectedChainStore::new(db.clone(), CachePolicy::Empty);

    // Snapshot the local index before touching anything: index 0 is this node's own base
    // (its pruning point after `init_with_pruning_point`), not genesis.
    let (local_tip, _) = chain.get_tip()?;
    let mut local: Vec<(u64, Hash)> = Vec::new();
    for i in 0..=local_tip {
        if let Ok(h) = chain.get_by_index(i) {
            local.push((i, h));
        }
    }
    let local_base = local.first().map(|(_, h)| *h).ok_or("local selected chain index is empty")?;
    eprintln!("local index holds {} entries, base block {}", local.len(), &local_base.to_string()[..16]);

    // Learn the shift: where does this node's base sit in genesis-based numbering?
    let shift = match by_hash.get(&local_base) {
        Some(&p) => p,
        None => {
            eprintln!(
                "ERROR: this node's base block {} is not in the export. The export is from a \
                 different chain, or predates this node's pruning point. Refusing to rebase.",
                local_base
            );
            std::process::exit(1);
        }
    };
    eprintln!("rebasing local entries by +{shift} (local index 0 == genesis index {shift})");

    let mut batch = rocksdb::WriteBatch::default();
    // Rebase existing entries, highest first so a shift can never overwrite an entry not yet moved.
    for &(i, h) in local.iter().rev() {
        chain.rebase_entry(&mut batch, i, h, i + shift)?;
    }
    chain.set_highest_index(&mut batch, local_tip + shift)?;
    db.write(std::mem::take(&mut batch))?;
    eprintln!("rebased {} local entries; highest index now {}", local.len(), local_tip + shift);

    // Write the historical range: index entries (the enumerator) plus scan records.
    let (mut idx_written, mut rec_written, mut rec_skipped) = (0u64, 0u64, 0u64);
    for r in &records {
        if r.index >= shift {
            continue; // covered by the node's own validated range
        }
        chain.write_entry(&mut batch, r.index, r.block)?;
        idx_written += 1;
        if scans.get(r.block)?.is_some() {
            rec_skipped += 1;
        } else {
            scans.set_batch(&mut batch, r.block, r.data.clone())?;
            rec_written += 1;
        }
        if idx_written % 20_000 == 0 {
            db.write(std::mem::take(&mut batch))?;
            eprintln!("  {idx_written} index entries, {rec_written} scan records");
        }
    }
    db.write(batch)?;

    eprintln!("RESTORED {idx_written} chain-index entries and {rec_written} scan records ({rec_skipped} already present)");
    eprintln!("node history now starts at genesis-based index 0; restart the node and query an old block to confirm");
    Ok(())
}
