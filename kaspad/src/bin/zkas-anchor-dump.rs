//! Dump a node's shielded anchor→source-block index.
//!
//! # Why
//!
//! `anchor_block` maps a shielded tree root to the block that produced it, and it is written
//! last-write-wins. Sibling blocks can carry an identical root, so the entry a node ends up
//! with depends on the order it validated them — orphans included. A node that indexed a
//! non-canonical producer fails `is_shielded_anchor_final`'s chain-ancestor test and drops the
//! spend; a node that indexed the canonical one keeps it. The two then disagree about a
//! block's coinbase forever.
//!
//! This index is therefore consensus-relevant state that is NOT derivable from the canonical
//! chain. A freshly synced node rebuilds it from canonical blocks only and so cannot reproduce
//! the entries an orphan produced. Dumping it from a node that followed the chain live is how
//! those entries get recovered.
//!
//! # Usage
//!
//! Opens the database READ-ONLY, so it is safe to point at a running node's datadir.
//!
//! ```text
//! zkas-anchor-dump --db <appdir>/<network>/datadir/consensus/consensus-001 --out anchors.tsv
//! ```
//!
//! Output is one `<anchor-hex>\t<block-hash>` pair per line, sorted by the store's key order.

use kaspa_consensus::model::stores::selected_chain::{DbSelectedChainStore, SelectedChainStoreReader};
use kaspa_consensus::model::stores::shielded::{
    AnchorBlockStoreReader, DbAnchorBlockStore, DbShieldedTreeStore, ShieldedTreeStoreReader,
};
use kaspa_database::prelude::{CachePolicy, ConnBuilder};
use kaspa_hashes::Hash;
use kaspa_shielded_core::tree::{GlobalTree, NoteCommitmentTree};
use std::io::{BufWriter, Write};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(db_path) = arg("--db") else {
        eprintln!("usage: zkas-anchor-dump --db <consensus-NNN dir> [--out <file>]");
        eprintln!("       the database is opened read-only; a running node is fine");
        std::process::exit(2);
    };
    let out_path = arg("--out");

    // Read-only: does not take the LOCK file, so this can run against the live node. It sees
    // the last flushed state, which for an index written on every block is effectively current.
    let db = ConnBuilder::default().with_db_path(db_path.clone().into()).with_files_limit(128).build_readonly()?;
    let store = DbAnchorBlockStore::new(db.clone(), CachePolicy::Empty);
    // A source block that is NOT on the selected chain is a POISONED entry: the anchor resolves
    // to a block `is_shielded_anchor_final` will reject as non-canonical, so this node drops
    // every spend proving against that root — while a node whose index holds the canonical
    // producer of the same root keeps them. `get_by_hash` errors for a non-chain block.
    let chain = DbSelectedChainStore::new(db.clone(), CachePolicy::Empty);

    let mut out: Box<dyn Write> = match &out_path {
        Some(p) => Box::new(BufWriter::new(std::fs::File::create(p)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // --collisions: emit the COMPLETE set of landmine anchors in one offline pass.
    //
    // A landmine is a tree root that a CANONICAL block produces but the index maps to an orphan.
    // Wallets anchor to roots they see on the canonical chain, so such a root is reachable by a
    // real spend — and when one arrives, the chain fails the orphan on the chain-ancestor test and
    // drops it while every freshly synced node resolves the canonical producer and keeps it. Each
    // one permanently wedges all new nodes.
    //
    // Discovering these by syncing and waiting for a rejection finds one per full IBD. Walking the
    // selected chain and recomputing each block's root finds all of them at once.
    // Streams the selected chain and holds nothing but the current block's frontier, so it runs
    // in tens of MB and is safe to point at a production box. (Building a root→block map over the
    // whole chain instead was enough to get OOM-killed on a 7 GB node host.)
    if std::env::args().any(|a| a == "--collisions") {
        let trees = DbShieldedTreeStore::new(db, CachePolicy::Empty);
        let (tip_index, _) = chain.get_tip()?;
        let (mut scanned, mut pins) = (0u64, 0u64);
        for i in 0..=tip_index {
            let Ok(block) = chain.get_by_index(i) else { continue };
            let Ok(state) = trees.get(block) else { continue };
            if state.size == 0 {
                continue;
            }
            let Ok(tree) = GlobalTree::from_state(&state) else { continue };
            let root = tree.anchor().to_bytes();
            scanned += 1;
            // What does the index say this canonical block's own root resolves to?
            match store.get(&root) {
                Ok(Some(indexed)) if indexed != block && chain.get_by_hash(indexed).is_err() => {
                    // A root a wallet can legitimately anchor to, resolving to a NON-canonical
                    // block: the chain drops such a spend, every fresh sync keeps it. Landmine.
                    writeln!(
                        out,
                        "{}\t{}\t# canonical producer {} at chain index {}",
                        faster_hex::hex_string(&root),
                        indexed,
                        block,
                        i
                    )?;
                    pins += 1;
                }
                _ => {}
            }
            if i % 100_000 == 0 {
                eprintln!("  scanned {i}/{tip_index} ({pins} landmines so far)");
            }
        }
        out.flush()?;
        eprintln!("canonical blocks with shielded state: {scanned}");
        eprintln!("LANDMINES (canonical root, orphan-indexed): {pins}");
        return Ok(());
    }

    let (mut count, mut poisoned) = (0u64, 0u64);
    for entry in store.iter_all() {
        let (anchor, block) = entry?;
        let canonical = chain.get_by_hash(block).is_ok();
        if !canonical {
            poisoned += 1;
        }
        writeln!(out, "{}\t{}\t{}", faster_hex::hex_string(&anchor), block, if canonical { "CANON" } else { "ORPHAN" })?;
        count += 1;
    }
    out.flush()?;
    eprintln!("dumped {count} anchor→block entries from {db_path}");
    eprintln!("POISONED (source block not on the selected chain): {poisoned}");
    Ok(())
}
