//! Binding a ZKas shielded state root `R` to its block hash `H_fc` — the `R → H_fc` link of the
//! canonical-`R` chain (see `kaspa_shielded_core::witness_chain`).
//!
//! A ZKas block commits its shielded state root `R` in the coinbase payload (the #24 commitment).
//! The block hash `H_fc = header_hash(header)` commits `hash_merkle_root`, which commits the
//! coinbase (leaf 0) via a Merkle branch. So proving `R → H_fc` is:
//!
//! ```text
//!   coinbase.payload[16..48] == R                     (the shielded commitment slot)
//!   fold(tx_hash(coinbase), branch) == hash_merkle_root
//!   header_hash(header) == H_fc
//! ```
//!
//! This module is the **reference** implementation, reusing the real
//! [`crate::hashing`] functions — so it is correct by construction. The RISC0 peg-out guest ports
//! this same logic `no_std` (transcribing the header/tx serialization), pinned against this
//! reference by [`tests`]. The guest needs it because it must show the *specific* `H_fc` that
//! Kaspa's sequencing commitment witnessed corresponds to a header whose merkle root commits the
//! very `R` the burn was proven under — otherwise a fabricated `R` could ride a real `H_fc`.

use kaspa_hashes::Hash;
use kaspa_merkle::merkle_hash;

use crate::{hashing, header::Header, tx::Transaction};

/// Byte offset of the shielded state root in a ZKas coinbase payload: `blue_score(8) + subsidy(8)`.
pub const SHIELDED_COMMITMENT_OFFSET: usize = 16;
/// Length of the shielded state root commitment.
pub const SHIELDED_COMMITMENT_LEN: usize = 32;

/// Cap on the coinbase Merkle branch length (one entry per tree level; 64 admits any block that can
/// physically exist). Rejecting an over-long branch before hashing is free and cannot refuse an
/// honest block. Mirrors `auxpow::MAX_COINBASE_MERKLE_BRANCH`.
pub const MAX_COINBASE_MERKLE_BRANCH: usize = 64;

/// Extract the shielded state root `R` committed in a ZKas coinbase payload.
pub fn extract_state_root(coinbase_payload: &[u8]) -> Option<[u8; 32]> {
    let end = SHIELDED_COMMITMENT_OFFSET + SHIELDED_COMMITMENT_LEN;
    coinbase_payload.get(SHIELDED_COMMITMENT_OFFSET..end)?.try_into().ok()
}

/// Verify a ZKas header commits shielded state root `expected_r`, returning its block hash `H_fc`.
///
/// `coinbase` is the block's coinbase (leaf 0) and `coinbase_branch` is its Merkle path (the
/// right-sibling at each level) up to `header.hash_merkle_root`. Returns `None` if the payload does
/// not commit `expected_r`, the branch does not reproduce the header's merkle root, or the branch is
/// absurdly long.
pub fn verify_state_root_binding(
    header: &Header,
    coinbase: &Transaction,
    coinbase_branch: &[Hash],
    expected_r: &[u8; 32],
) -> Option<Hash> {
    if coinbase_branch.len() > MAX_COINBASE_MERKLE_BRANCH {
        return None;
    }
    // (1) The coinbase payload commits exactly R.
    if &extract_state_root(&coinbase.payload)? != expected_r {
        return None;
    }
    // (2) The coinbase (leaf 0) is Merkle-included under the header's merkle root. Leaf 0 is always
    //     the left child at every level, so a plain left-fold reproduces the root.
    let mut acc = hashing::tx::hash(coinbase);
    for sibling in coinbase_branch {
        acc = merkle_hash(acc, *sibling);
    }
    if acc != header.hash_merkle_root {
        return None;
    }
    // (3) H_fc is the header hash, which commits the merkle root and thus (transitively) R.
    Some(hashing::header::hash(header))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subnets::SUBNETWORK_ID_COINBASE;
    use crate::tx::Transaction;

    fn r_sample() -> [u8; 32] {
        [0x5a; 32]
    }

    /// A coinbase whose payload places `r` at the shielded-commitment offset, with plausible
    /// surrounding bytes (blue_score, subsidy, then trailing miner data).
    fn coinbase_committing(r: [u8; 32]) -> Transaction {
        let mut payload = Vec::new();
        payload.extend_from_slice(&123u64.to_le_bytes()); // blue_score
        payload.extend_from_slice(&456u64.to_le_bytes()); // subsidy
        payload.extend_from_slice(&r); // shielded commitment
        payload.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]); // trailing miner data
        Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, payload)
    }

    fn header_with_merkle_root(root: Hash) -> Header {
        Header::new_finalized(
            1,
            vec![vec![1.into()]].try_into().unwrap(),
            root,
            Default::default(),
            Default::default(),
            234,
            23,
            567,
            0,
            0.into(),
            0,
            Default::default(),
        )
    }

    #[test]
    fn extract_reads_the_commitment_slot() {
        let cb = coinbase_committing(r_sample());
        assert_eq!(extract_state_root(&cb.payload), Some(r_sample()));
        // Too-short payload has no commitment.
        assert_eq!(extract_state_root(&[0u8; 10]), None);
    }

    /// Single-tx block: the coinbase is the whole tree, so `hash_merkle_root == tx_hash(coinbase)`
    /// and the branch is empty. `verify` must return exactly `header_hash(header)`.
    #[test]
    fn binds_r_to_hfc_single_tx_block() {
        let r = r_sample();
        let cb = coinbase_committing(r);
        let root = hashing::tx::hash(&cb);
        let header = header_with_merkle_root(root);

        let hfc = verify_state_root_binding(&header, &cb, &[], &r).expect("binding holds");
        assert_eq!(hfc, hashing::header::hash(&header));

        // Wrong R: the payload commits a different state root.
        let mut wrong = r;
        wrong[0] ^= 1;
        assert_eq!(verify_state_root_binding(&header, &cb, &[], &wrong), None);
    }

    /// Multi-tx block: the coinbase (leaf 0) has a sibling, so the branch folds it to the root.
    #[test]
    fn binds_r_to_hfc_with_a_sibling() {
        let r = r_sample();
        let cb = coinbase_committing(r);
        let sibling = Hash::from_bytes([0x77; 32]);
        let root = merkle_hash(hashing::tx::hash(&cb), sibling);
        let header = header_with_merkle_root(root);

        let hfc = verify_state_root_binding(&header, &cb, &[sibling], &r).expect("binding holds");
        assert_eq!(hfc, hashing::header::hash(&header));

        // Tampered branch ⇒ merkle root mismatch ⇒ rejected.
        let bad = Hash::from_bytes([0x00; 32]);
        assert_eq!(verify_state_root_binding(&header, &cb, &[bad], &r), None);
    }

    /// An over-long branch is refused before any hashing.
    #[test]
    fn rejects_absurd_branch() {
        let r = r_sample();
        let cb = coinbase_committing(r);
        let header = header_with_merkle_root(Hash::from_bytes([0x01; 32]));
        let branch = vec![Hash::from_bytes([0u8; 32]); MAX_COINBASE_MERKLE_BRANCH + 1];
        assert_eq!(verify_state_root_binding(&header, &cb, &branch, &r), None);
    }

    /// Vector generator for the guest-side hashing transcription (`vprogs zk/abi::kaspa_hashing`).
    /// Prints the real header_hash and coinbase tx_hash for fixed inputs so the guest port can pin
    /// to them. Run with: cargo test -p kaspa-consensus-core --lib gen_guest_hash_vectors -- --nocapture --ignored
    #[test]
    #[ignore]
    fn gen_guest_hash_vectors() {
        use crate::subnets::SUBNETWORK_ID_COINBASE;
        use crate::tx::{ScriptPublicKey, Transaction, TransactionOutput};
        // Coinbase: version 1, one output, R at payload[16..48].
        let mut payload = Vec::new();
        payload.extend_from_slice(&123u64.to_le_bytes());
        payload.extend_from_slice(&456u64.to_le_bytes());
        payload.extend_from_slice(&[0x5a; 32]);
        payload.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
        let spk = ScriptPublicKey::new(0, vec![0xaa, 0xbb, 0xcc].into());
        let out = TransactionOutput::new(5000, spk);
        let cb = Transaction::new(0, vec![], vec![out], 7, SUBNETWORK_ID_COINBASE, 9, payload.clone());
        let txh = crate::hashing::tx::hash(&cb);
        println!("COINBASE_TX_HASH={}", txh);
        println!("COINBASE_MASS={}", cb.storage_mass());
        print!("SUBNETWORK_COINBASE=");
        for b in SUBNETWORK_ID_COINBASE.as_bytes() {
            print!("{:02x}", b);
        }
        println!();

        let header = crate::header::Header::new_finalized(
            1,
            vec![vec![crate::Hash::from_bytes([0x01; 32])]].try_into().unwrap(),
            txh, // hash_merkle_root = coinbase (single tx)
            crate::Hash::from_bytes([0x02; 32]),
            crate::Hash::from_bytes([0x03; 32]),
            234,
            23,
            567,
            42,
            5.into(),
            99,
            crate::Hash::from_bytes([0x04; 32]),
        );
        println!("HEADER_HASH={}", crate::hashing::header::hash(&header));
        println!("HEADER_BLUE_WORK={:x}", 5u128);
    }
}
