#![no_std]

extern crate alloc;
extern crate core;

pub mod streaming;

use alloc::vec;
use kaspa_hashes::{Hash, Hasher, MerkleBranchHash, ZERO_HASH};

pub use streaming::StreamingMerkleBuilder;

pub fn calc_merkle_root(hashes: impl ExactSizeIterator<Item = Hash>) -> Hash {
    calc_merkle_root_with_hasher::<MerkleBranchHash>(hashes)
}

pub fn merkle_hash(left: Hash, right: Hash) -> Hash {
    merkle_hash_with_hasher(left, right, MerkleBranchHash::new())
}

pub fn merkle_hash_with_hasher(left: Hash, right: Hash, mut hasher: impl Hasher) -> Hash {
    hasher.update(left).update(right);
    hasher.finalize()
}

/// Standard Merkle convention: a tree with one leaf is the leaf itself.
/// Callers must ensure the set of valid leaf hashes is disjoint from valid
/// internal-node hashes (typically via per-domain hashers) so the two cases
/// cannot be confused.
pub fn calc_merkle_root_with_hasher<H: Hasher>(mut hashes: impl ExactSizeIterator<Item = Hash>) -> Hash {
    match hashes.len() {
        0 => return cold_path_empty(),
        1 => return hashes.next().unwrap(),
        _ => {}
    }
    let next_pot = hashes.len().next_power_of_two();
    let vec_len = 2 * next_pot - 1;

    let mut merkles = vec![None; vec_len];
    for (i, hash) in hashes.enumerate() {
        merkles[i] = Some(hash);
    }
    for (offset, i) in (next_pot..).zip((0..vec_len - 1).step_by(2)) {
        if merkles[i].is_none() {
            merkles[offset] = None;
        } else {
            merkles[offset] = Some(merkle_hash_with_hasher(merkles[i].unwrap(), merkles[i + 1].unwrap_or(ZERO_HASH), H::default()));
        }
    }
    merkles.last().unwrap().unwrap()
}

#[inline(never)]
#[cold]
fn cold_path_empty() -> Hash {
    ZERO_HASH
}

/// Build the Merkle witness (authentication path) for the **first leaf** (index 0) of the tree
/// `calc_merkle_root_with_hasher::<H>` builds — i.e. the ordered list of right-siblings on the path
/// from leaf 0 up to the root. A verifier reproduces the root by folding
/// `acc = merkle_hash_with_hasher(acc, sibling, H)` over the returned siblings, starting from the
/// leaf-0 hash.
///
/// This is the producing side of the coinbase inclusion proof: leaf 0 is the coinbase transaction,
/// so the witness proves the coinbase is committed by the block's `hash_merkle_root`. It is the
/// general (any leaf count) form of the 1-/2-tx shapes hand-built in the `auxpow` tests, and the
/// exact path a canonical-`R` witness (`coinbase_branch`) or an `AuxPow::coinbase_merkle_branch`
/// needs. A single-leaf tree yields an empty witness (the root is the leaf itself).
pub fn create_first_leaf_merkle_witness<H: Hasher>(leaf_hashes: &[Hash]) -> alloc::vec::Vec<Hash> {
    let mut branch = alloc::vec::Vec::new();
    if leaf_hashes.len() <= 1 {
        return branch;
    }
    // Reduce level by level exactly as the padded perfect tree does on the left spine: at each
    // level leaf 0's node sits at position 0, so its sibling is position 1, and folding pairs
    // (padding a missing right child with ZERO_HASH) lands leaf 0's parent at position 0 again.
    let mut level = leaf_hashes.to_vec();
    while level.len() > 1 {
        branch.push(level[1]);
        let mut next = alloc::vec::Vec::with_capacity(level.len().div_ceil(2));
        let mut i = 0;
        while i < level.len() {
            let left = level[i];
            let right = if i + 1 < level.len() { level[i + 1] } else { ZERO_HASH };
            next.push(merkle_hash_with_hasher(left, right, H::default()));
            i += 2;
        }
        level = next;
    }
    branch
}

/// [`create_first_leaf_merkle_witness`] for the default transaction Merkle-branch hasher — i.e. the
/// coinbase inclusion path against a block's `hash_merkle_root`.
pub fn create_coinbase_merkle_witness(tx_hashes: &[Hash]) -> alloc::vec::Vec<Hash> {
    create_first_leaf_merkle_witness::<MerkleBranchHash>(tx_hashes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use core::iter;
    use kaspa_hashes::{HasherBase, SeqCommitMerkleBranch, TransactionHash};
    fn seq_comm_merkle_root(hashes: impl ExactSizeIterator<Item = Hash>) -> Hash {
        calc_merkle_root_with_hasher::<SeqCommitMerkleBranch>(hashes)
    }
    fn make_hash(data: &[u8]) -> Hash {
        let mut hasher = TransactionHash::new();
        hasher.update(data);
        hasher.finalize()
    }
    #[test]
    fn test_empty_returns_zero_hash() {
        let root = calc_merkle_root(core::iter::empty());
        assert_eq!(root, ZERO_HASH, "Empty input should return ZERO_HASH");

        let seq_root = seq_comm_merkle_root(core::iter::empty());
        assert_eq!(seq_root, ZERO_HASH, "Empty input should return ZERO_HASH for seq_comm");
    }

    #[test]
    fn test_single_entry_returns_hash() {
        let entry = make_hash(b"single_entry");
        let root = calc_merkle_root(iter::once(entry));
        assert_eq!(root, entry);

        let seq_comm_root = seq_comm_merkle_root(iter::once(entry));
        assert_eq!(seq_comm_root, entry, "Single entry should return the leaf itself");
    }

    #[test]
    fn test_two_entries_returns_hash_of_both() {
        let h1 = make_hash(b"entry1");
        let h2 = make_hash(b"entry2");

        let root = calc_merkle_root([h1, h2].into_iter());
        let expected = merkle_hash(h1, h2);
        assert_eq!(root, expected, "Two entries should hash directly together");

        let seq_root = seq_comm_merkle_root([h1, h2].into_iter());
        let seq_expected = merkle_hash_with_hasher(h1, h2, SeqCommitMerkleBranch::default());
        assert_eq!(seq_root, seq_expected, "Two entries should hash directly together for seq_comm");
    }

    #[test]
    fn test_three_entries() {
        // Tree structure for 3 entries (next_pot = 4):
        // Indices: [h1, h2, h3, None, ..., result]
        // Level 0: h1, h2, h3, None
        // Level 1: hash(h1,h2), hash(h3,ZERO)
        // Level 2: hash(hash(h1,h2), hash(h3,ZERO))
        let h1 = make_hash(b"h1");
        let h2 = make_hash(b"h2");
        let h3 = make_hash(b"h3");

        let root = calc_merkle_root([h1, h2, h3].into_iter());

        let left = merkle_hash(h1, h2);
        let right = merkle_hash(h3, ZERO_HASH);
        let expected = merkle_hash(left, right);

        assert_eq!(root, expected, "Three entries should build correct tree");
    }

    #[test]
    fn test_four_entries() {
        // Tree structure for 4 entries (next_pot = 4):
        // Level 0: h1, h2, h3, h4
        // Level 1: hash(h1,h2), hash(h3,h4)
        // Level 2: hash(hash(h1,h2), hash(h3,h4))
        let h1 = make_hash(b"h1");
        let h2 = make_hash(b"h2");
        let h3 = make_hash(b"h3");
        let h4 = make_hash(b"h4");

        let root = calc_merkle_root([h1, h2, h3, h4].into_iter());

        let left = merkle_hash(h1, h2);
        let right = merkle_hash(h3, h4);
        let expected = merkle_hash(left, right);

        assert_eq!(root, expected, "Four entries should build correct balanced tree");
    }
    #[test]
    fn test_consistency_multiple_calls() {
        let hashes: Vec<Hash> = (0..5).map(|i| make_hash(&[i])).collect();

        let root1 = calc_merkle_root(hashes.clone().into_iter());
        let root2 = calc_merkle_root(hashes.clone().into_iter());

        assert_eq!(root1, root2, "Multiple calls with same input should produce same result");
    }

    #[test]
    fn test_order_matters() {
        let h1 = make_hash(b"h1");
        let h2 = make_hash(b"h2");

        let root1 = calc_merkle_root([h1, h2].into_iter());
        let root2 = calc_merkle_root([h2, h1].into_iter());

        assert_ne!(root1, root2, "Order of hashes should matter");
    }

    /// The first-leaf witness must fold back to exactly the root `calc_merkle_root` produces, for
    /// every leaf count — this is the coinbase-inclusion proof the aux_pow / canonical-`R` verifiers
    /// check via `merkle_hash(acc, sibling)`.
    #[test]
    fn first_leaf_witness_reproduces_root_all_sizes() {
        for n in 1..=33usize {
            let leaves: Vec<Hash> = (0..n).map(|i| make_hash(&[i as u8, (i >> 8) as u8])).collect();
            let root = calc_merkle_root(leaves.clone().into_iter());
            let branch = create_coinbase_merkle_witness(&leaves);
            // Fold the coinbase (leaf 0) up the branch, exactly as the on-chain verifier does.
            let mut acc = leaves[0];
            for sibling in &branch {
                acc = merkle_hash(acc, *sibling);
            }
            assert_eq!(acc, root, "n={n}: first-leaf witness must reproduce the merkle root");
            // Sanity on branch length: ceil(log2(n)) siblings (0 for a single leaf).
            let expected_len = n.next_power_of_two().trailing_zeros() as usize;
            assert_eq!(branch.len(), expected_len, "n={n}: unexpected branch length");
        }
    }

    #[test]
    fn first_leaf_witness_matches_auxpow_two_tx_shape() {
        // Mirrors the auxpow test helper: 1 tx ⇒ empty branch; 2 txs ⇒ [hash(tx1)].
        let h0 = make_hash(b"cb");
        assert!(create_coinbase_merkle_witness(&[h0]).is_empty());
        let h1 = make_hash(b"tx1");
        assert_eq!(create_coinbase_merkle_witness(&[h0, h1]), vec![h1]);
    }

    #[test]
    fn tampered_first_leaf_witness_breaks_the_root() {
        let leaves: Vec<Hash> = (0..7u8).map(|i| make_hash(&[i])).collect();
        let root = calc_merkle_root(leaves.clone().into_iter());
        let mut branch = create_coinbase_merkle_witness(&leaves);
        branch[0] = make_hash(b"tampered");
        let mut acc = leaves[0];
        for sibling in &branch {
            acc = merkle_hash(acc, *sibling);
        }
        assert_ne!(acc, root, "a corrupted sibling must not reproduce the root");
    }
}
