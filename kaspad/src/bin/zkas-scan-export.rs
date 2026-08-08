//! Export the shielded scan archive — the complete, self-verifying wallet-history backup.
//!
//! # Why this exists
//!
//! On an all-shielded chain a wallet cannot rebuild itself from a UTXO set; it recovers its
//! notes by replaying the per-chain-block scan archive. That archive is written only by nodes
//! that validated the blocks themselves, and IBD does not transfer it —
//! `PruningPointShieldedMetadata` carries the frontier and a nullifier MuHash, which are
//! aggregates and cannot yield anyone's notes. So a freshly synced node holds history only from
//! its pruning point forward, and the pre-pruning-point history exists ONLY on nodes that were
//! running at the time.
//!
//! Measured on mainnet 2026-08-05: the whole archive is 467,072 entries / 231.9 MiB — 2.35% of a
//! 10.3 GiB database. Losing it would be unrecoverable; copying it is cheap. This tool makes that
//! copy, and verifies it is complete rather than asking anyone to trust it.
//!
//! # Self-verification
//!
//! `GlobalTree::append` is pure append-only, so the note-commitment tree is a deterministic
//! function of the leaf sequence. The export replays every leaf it writes — coinbase mints first,
//! then accepted actions, the order consensus appended them — into a fresh tree and requires the
//! result to EQUAL the node's own stored frontier at the final block. A truncated, reordered or
//! corrupt export cannot match. `VERIFIED` means the file provably reconstructs the chain's tree;
//! anything else exits non-zero.
//!
//! It did not always mean that. Until 2026-08-08 the replay skipped coinbase leaves while the
//! stored frontier contained them, so the two sides were incomparable — measured 841,577 replayed
//! against 2,239,146 stored — and the check had been weakened to `replayed.size > stored.size`,
//! which printed `VERIFIED` for a truncated file, a reordered one, or a one-leaf one. Treat a
//! `VERIFIED` from an older build as unproven.
//!
//! # Usage
//!
//! Opens the database READ-ONLY — safe against a running node.
//!
//! ```text
//! zkas-scan-export --db <appdir>/<network>/datadir/consensus/consensus-001 --out history.bin
//! ```
//!
//! Format: `ZKASHIST\x01` magic, then repeated
//! `[8B LE genesis-based chain index][32B block hash][4B LE len][bincode(ShieldedScanBlockData)]`,
//! ascending chain order (genesis first) — the order the tree was built in, so it replays
//! directly.
//!
//! The chain index is carried because the scan records alone are NOT restorable: a wallet reaches
//! them through `get_shielded_chain_range`, which resolves the start block via
//! `selected_chain_store.get_by_hash()`. Without index entries a restored node holds the data and
//! still refuses to serve it — measured. The index is genesis-based here; the importer rebases,
//! because `init_with_pruning_point` numbers a synced node from ITS pruning point as 0.

use kaspa_consensus::model::stores::selected_chain::{DbSelectedChainStore, SelectedChainStoreReader};
use kaspa_consensus::model::stores::shielded::{
    DbShieldedScanBlockStore, DbShieldedTreeStore, ShieldedScanBlockStoreReader, ShieldedTreeStoreReader,
};
use kaspa_database::prelude::{CachePolicy, ConnBuilder};
use kaspa_shielded_core::ExtractedNoteCommitment;
use kaspa_shielded_core::tree::{GlobalTree, NoteCommitmentTree};
use kaspa_consensus::processes::shielded::coinbase_notes_from_outputs;
use kaspa_shielded_core::wallet::CompactActionRecord;
use std::io::{BufWriter, Write};

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db_path) = arg("--db") else {
        eprintln!("usage: zkas-scan-export --db <consensus-NNN dir> --out <file>");
        eprintln!("       opens the database read-only; safe against a running node");
        std::process::exit(2);
    };
    let out_path = arg("--out").unwrap_or_else(|| "shielded-history.bin".to_string());

    let db = ConnBuilder::default().with_db_path(db_path.clone().into()).with_files_limit(128).build_readonly()?;
    let chain = DbSelectedChainStore::new(db.clone(), CachePolicy::Empty);
    let scans = DbShieldedScanBlockStore::new(db.clone(), CachePolicy::Empty);
    let trees = DbShieldedTreeStore::new(db, CachePolicy::Empty);

    let (tip_index, _) = chain.get_tip()?;
    let mut out = BufWriter::new(std::fs::File::create(&out_path)?);
    out.write_all(b"ZKASHIST\x01")?;

    // Replay as we go: the tree is append-only, so building it in the same order we write proves
    // the file is a faithful, complete copy without a second pass over the data.
    let mut tree = GlobalTree::default();
    let (mut written, mut leaves, mut bytes) = (0u64, 0u64, 0u64);
    let mut action_leaves = 0u64;
    let mut bad = 0u64;
    let mut last_with_state = None;

    for index in 0..=tip_index {
        let Ok(block) = chain.get_by_index(index) else { continue };
        // Report WHICH record fails rather than a bare DeserializationError: a bad record is
        // otherwise invisible among hundreds of thousands, and the index/hash is what makes it
        // comparable against the source database.
        let data = match scans.get(block) {
            Ok(Some(d)) => d,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("BAD RECORD at chain index {index}, block {block}: {e}");
                bad += 1;
                if bad <= 5 {
                    continue;
                }
                return Err(format!("aborting after {bad} unreadable records").into());
            }
        };

        // Coinbase mint FIRST, then the accepted actions — the order consensus appended them in.
        // The coinbase leaves are not optional decoration: the node's stored frontier contains
        // them, so a replay that skips them can never equal it, which is how this tool's own
        // check came to compare sizes loosely instead of proving anything.
        let coinbase = if !data.coinbase_commitments.is_empty() {
            data.coinbase_commitments.clone()
        } else {
            // Records written before commitments were stored: derive with the SAME function
            // consensus validates with.
            //
            // `None` is the pre-F-02 seed (`txid || index`), which is correct because
            // `shielded_coinbase_seed_activation` is `never()` on every configured network. This
            // tool has no params handle to check that against — but it does not need to guess
            // safely, because the frontier equality check below is what catches a wrong rule: a
            // mis-derived coinbase leaf changes the tree and the comparison fails loudly. If a
            // network ever activates F-02, this exits non-zero rather than exporting silent junk.
            let outputs: Vec<(&[u8], u64)> = data.coinbase_outputs.iter().map(|(s, v)| (s.as_slice(), *v)).collect();
            coinbase_notes_from_outputs(data.coinbase_txid, &outputs, None)
                .map_err(|e| format!("cannot derive coinbase commitments for {block}: {e:?}"))?
                .iter()
                .map(|n| n.commitment.to_bytes())
                .collect()
        };
        for cmx in &coinbase {
            let c = ExtractedNoteCommitment::from_bytes(cmx);
            if bool::from(c.is_none()) {
                return Err(format!("non-canonical coinbase cmx in block {block} at chain index {index}").into());
            }
            tree.append(c.unwrap()).map_err(|_| "note commitment tree full")?;
            leaves += 1;
        }
        for tx in &data.accepted {
            for rec in tx.action_bytes.chunks_exact(CompactActionRecord::SERIALIZED_LEN) {
                let mut cmx = [0u8; 32];
                cmx.copy_from_slice(&rec[32..64]);
                let c = ExtractedNoteCommitment::from_bytes(&cmx);
                if bool::from(c.is_none()) {
                    return Err(format!("non-canonical cmx in block {block} at chain index {index}").into());
                }
                tree.append(c.unwrap()).map_err(|_| "note commitment tree full")?;
                leaves += 1;
                action_leaves += 1;
            }
        }

        let encoded = bincode::serialize(&data)?;
        out.write_all(&index.to_le_bytes())?;
        out.write_all(&block.as_bytes())?;
        out.write_all(&(encoded.len() as u32).to_le_bytes())?;
        out.write_all(&encoded)?;
        bytes += 44 + encoded.len() as u64;
        written += 1;
        last_with_state = Some(block);

        if index % 100_000 == 0 {
            eprintln!("  {index}/{tip_index} chain blocks, {written} records, {leaves} leaves");
        }
    }
    out.flush()?;

    eprintln!("wrote {written} scan records ({:.1} MiB) to {out_path}", bytes as f64 / (1024.0 * 1024.0));
    eprintln!("replayed {leaves} note commitments");
    if bad > 0 {
        eprintln!("UNREADABLE RECORDS: {bad}");
    }

    // The proof: the replayed tree must EQUAL the node's own stored frontier at the last block
    // with shielded state — same size and same frontier bytes.
    //
    // This used to compare `replayed.size > stored.size` and print VERIFIED for everything else,
    // which passed a truncated export, a reordered one, and a one-leaf one. It could not do
    // better, because the replay omitted coinbase leaves while the stored frontier includes them,
    // so the two sides were never comparable: measured 841,577 replayed against 2,239,146 stored,
    // and it still said VERIFIED. Both sides now cover the same leaves, so equality is the test.
    match last_with_state {
        Some(block) => {
            let stored = trees.get(block)?;
            let replayed = tree.to_state();
            eprintln!("stored   frontier @ {}: size {}", &block.to_string()[..16], stored.size);
            eprintln!("replayed frontier        : size {}", replayed.size);
            if stored.size == 0 {
                eprintln!("NOTE: node holds no frontier for that block (pruned checkpoint) — cannot verify");
                eprintln!("UNVERIFIED: export written but NOT proven complete");
                std::process::exit(1);
            } else if replayed != stored {
                eprintln!(
                    "INCONSISTENT: replayed frontier does not match the node's ({} vs {} leaves){}",
                    replayed.size,
                    stored.size,
                    if replayed.size == stored.size { " — same size, different contents" } else { "" }
                );
                eprintln!("The export is NOT a complete copy; do not restore from it.");
                std::process::exit(1);
            } else {
                eprintln!(
                    "VERIFIED: replayed frontier equals the node's at {} leaves ({action_leaves} action + {} coinbase)",
                    stored.size,
                    leaves - action_leaves
                );
            }
        }
        None => {
            eprintln!("WARNING: no scan records found — is this the right database?");
            std::process::exit(1);
        }
    }
    Ok(())
}
