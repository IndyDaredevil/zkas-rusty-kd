//! Canonical-`R`: proving a ZKas state root is committed by Kaspa's own proof-of-work.
//!
//! The peg-out guest recomputes the ZKas state root `R` and proves a burn sits under it
//! ([`crate::attestation`] / the guest's `zkas_bridge`). That establishes *internal consistency* —
//! the burn belongs to the state it names — but not that the state was ever **mined**. A checkpoint
//! publisher could still assert a fabricated `R`. This module closes that gap.
//!
//! # The binding chain
//!
//! ZKas is merge-mined into Kaspa. Every ZKas block hash `H_fc` is embedded in the coinbase payload
//! of the Kaspa block that mined it (`MERGE_MINE_MAGIC || hex(H_fc)`, exactly once — see
//! `kaspa_consensus_core::auxpow`). Kaspa's KIP-21 sequencing commitment folds every mergeset
//! block's coinbase payload into the block header's `seq_commit`:
//!
//! ```text
//!   coinbase payload  ─(embed)→  H_fc  ─(ZKas header → coinbase)→  R
//!         │
//!         └─ miner_payload_leaf → miner_payload_root → payload_and_ctx_digest
//!                                → seq_state_root → seq_commit   (in the Kaspa header)
//! ```
//!
//! and `seq_commit` is readable on-chain for a buried chain block via the `OpChainblockSeqCommit`
//! opcode. So a covenant can verify — with **no trusted operator** — that a ZKas block was mined by
//! Kaspa's own hashpower:
//!
//! 1. the guest proves `R → H_fc → Kaspa payload → seq_commit` (this module), committing
//!    `(seq_commit, kaspa_block_hash)` to its journal;
//! 2. the covenant requires `journal.seq_commit == OpChainblockSeqCommit(journal.kaspa_block)` and
//!    that the block is buried past a finality depth.
//!
//! Then Kaspa trusts `R` because *Kaspa's own chain* committed the ZKas block that produced it.
//!
//! # Reuse
//!
//! The `seq_commit` recomputation is entirely `kaspa_seq_commit` (a `no_std` crate, so this same
//! code compiles into the RISC0 guest). The `H_fc` extraction mirrors
//! `kaspa_consensus_core::auxpow` byte-for-byte (asserted by a reverse-dependency test in the consensus crate).
//! Because both sides call the *same* `kaspa_seq_commit`, there is nothing to keep in sync with a
//! hand-rolled vector — the shared crate is the pin.
//!
//! # Honest limits (unchanged by this module)
//!
//! - **Finality depth is an assumption.** "Buried past N" is an SPV-style parameter; soundness rests
//!   on an attacker not out-mining Kaspa to that depth. Enforced by the covenant, not here.
//! - **Liveness depends on the merge-mining pool.** Only Kaspa blocks that carry an `H_fc` witness a
//!   ZKas block, so peg-out latency tracks how often the pool lands Kaspa blocks. Not a soundness
//!   hole.


use kaspa_hashes::Hash;
use kaspa_seq_commit::hashing::{
    miner_payload_leaf, miner_payload_root, payload_and_context_digest, seq_commit, seq_state_root,
};
use kaspa_seq_commit::types::{MinerPayloadLeafInput, SeqCommitInput, SeqState};

/// The 4-byte tag marking the 32-byte ZKas commitment in a parent coinbase payload ("ZKas Merged
/// Mining"). Must equal `kaspa_consensus_core::auxpow::MERGE_MINE_MAGIC`.
pub const MERGE_MINE_MAGIC: [u8; 4] = *b"ZKMM";

/// Length of the lowercase-hex commitment following the magic (32 bytes → 64 hex chars).
const COMMITMENT_HEX_LEN: usize = 64;

/// Extract the single `H_fc` committed in a Kaspa coinbase payload.
///
/// Returns `None` unless [`MERGE_MINE_MAGIC`] occurs **exactly once**, followed by a full
/// 64-char lowercase-hex encoding of a 32-byte hash. Mirrors
/// `kaspa_consensus_core::auxpow::AuxPow::committed_hash` — the "exactly once" rule is the AuxPoW
/// anti-ambiguity hardening (two commitments would let one parent PoW back two aux blocks).
pub fn extract_hfc(payload: &[u8]) -> Option<Hash> {
    let mut found: Option<Hash> = None;
    let mut i = 0usize;
    while i + MERGE_MINE_MAGIC.len() <= payload.len() {
        if payload[i..i + MERGE_MINE_MAGIC.len()] == MERGE_MINE_MAGIC {
            let start = i + MERGE_MINE_MAGIC.len();
            let end = start + COMMITMENT_HEX_LEN;
            if end > payload.len() {
                return None; // magic present but truncated commitment → malformed
            }
            let bytes = decode_hex32(&payload[start..end])?;
            if found.is_some() {
                return None; // second occurrence → ambiguous → reject
            }
            found = Some(Hash::from_bytes(bytes));
            i = end;
        } else {
            i += 1;
        }
    }
    found
}

/// Decode 64 lowercase-hex ASCII bytes into a 32-byte array, or `None` if not valid hex.
fn decode_hex32(hex: &[u8]) -> Option<[u8; 32]> {
    if hex.len() != COMMITMENT_HEX_LEN {
        return None;
    }
    let nibble = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    };
    let mut out = [0u8; 32];
    for (j, byte) in out.iter_mut().enumerate() {
        let hi = nibble(hex[2 * j])?;
        let lo = nibble(hex[2 * j + 1])?;
        *byte = (hi << 4) | lo;
    }
    Some(out)
}

/// Everything needed to recompute one Kaspa block's `seq_commit` and check it commits `H_fc`.
///
/// The opaque fields (`activity_root`, `context_hash`, `parent_seq_commit`) are carried verbatim:
/// the guest does not need to understand the rest of the sequencing state, only that folding these
/// exact values with the miner payload reproduces the header's `seq_commit`. Tampering with any of
/// them yields a different `seq_commit`, which then fails the covenant's `OpChainblockSeqCommit`
/// equality — so the guest need not police them.
#[derive(Clone, Debug)]
pub struct SeqCommitWitness {
    /// The Kaspa coinbase payload of *our* mergeset block (embeds `H_fc`).
    pub kaspa_payload: Vec<u8>,
    /// Our Kaspa block hash (the `block_hash` of its miner-payload leaf).
    pub block_hash: Hash,
    /// Big-endian blue-work bytes of our block, as fed to `miner_payload_leaf`.
    pub blue_work_be: Vec<u8>,
    /// The full ordered list of miner-payload leaves in the mergeset, **excluding ours**. Combined
    /// with our recomputed leaf spliced in at [`Self::leaf_index`], this reproduces the
    /// `miner_payload_root` without needing Kaspa's merkle-branch convention.
    pub other_leaves: Vec<Hash>,
    /// The position our leaf occupies in the ordered mergeset leaf list.
    pub leaf_index: usize,
    /// The sequencing activity root (opaque; carried through).
    pub activity_root: Hash,
    /// The mergeset context hash (opaque; carried through).
    pub context_hash: Hash,
    /// The parent block's `seq_commit` (opaque; carried through).
    pub parent_seq_commit: Hash,
}

/// Why a witness did not recompute to a valid `seq_commit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WitnessError {
    /// The payload does not embed exactly one `H_fc`.
    NoCommitment,
    /// `leaf_index` is past the reconstructed leaf list.
    LeafIndexOutOfRange,
}

impl SeqCommitWitness {
    /// The `H_fc` this witness's payload commits to.
    pub fn hfc(&self) -> Option<Hash> {
        extract_hfc(&self.kaspa_payload)
    }

    /// Reconstruct the full ordered mergeset leaf list with our recomputed leaf spliced in.
    fn all_leaves(&self) -> Result<Vec<Hash>, WitnessError> {
        if self.leaf_index > self.other_leaves.len() {
            return Err(WitnessError::LeafIndexOutOfRange);
        }
        let our_leaf = miner_payload_leaf(MinerPayloadLeafInput {
            block_hash: &self.block_hash,
            blue_work_be_bytes: &self.blue_work_be,
            payload: &self.kaspa_payload,
        });
        let mut leaves = self.other_leaves.clone();
        leaves.insert(self.leaf_index, our_leaf);
        Ok(leaves)
    }

    /// Recompute the Kaspa header `seq_commit` this witness implies.
    ///
    /// Byte-identical to what `kaspa_seq_commit` computes for the real block, so it can be compared
    /// against the value `OpChainblockSeqCommit` returns on-chain.
    pub fn recompute_seq_commit(&self) -> Result<Hash, WitnessError> {
        let leaves = self.all_leaves()?;
        let payload_root = miner_payload_root(leaves.into_iter());
        let pd = payload_and_context_digest(&self.context_hash, &payload_root);
        let state_root = seq_state_root(&SeqState { activity_root: &self.activity_root, payload_and_ctx_digest: &pd });
        Ok(seq_commit(&SeqCommitInput { parent_seq_commit: &self.parent_seq_commit, state_root: &state_root }))
    }

    /// The full canonical-`R` check the guest performs: the payload commits exactly `expected_hfc`,
    /// and the recomputed `seq_commit` equals the one the covenant will read on-chain.
    ///
    /// `expected_hfc` is the ZKas header hash the guest independently derived on the `R → H_fc`
    /// side; requiring the payload to commit *that* hash is what ties Kaspa's witnessed block to the
    /// very state root the burn was proven against.
    pub fn verify(&self, expected_hfc: Hash, expected_seq_commit: Hash) -> Result<bool, WitnessError> {
        let hfc = self.hfc().ok_or(WitnessError::NoCommitment)?;
        if hfc != expected_hfc {
            return Ok(false);
        }
        Ok(self.recompute_seq_commit()? == expected_seq_commit)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec;

    use super::*;

    fn hfc_sample() -> Hash {
        Hash::from_bytes([0x7c; 32])
    }

    /// Build a coinbase payload the way `kaspa_consensus_core::auxpow::AuxPow::embed_commitment`
    /// does: `prefix || MERGE_MINE_MAGIC || lowercase_hex(hfc) || suffix`. Replicated inline (not
    /// imported) because kaspa-consensus-core depends on this crate — the reverse dependency is
    /// pinned by a test in the consensus crate (`auxpow_commitment_matches_witness_extract`), which
    /// CAN depend on both.
    fn embed(prefix: &[u8], hfc: Hash, suffix: &[u8]) -> std::vec::Vec<u8> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = std::vec::Vec::new();
        out.extend_from_slice(prefix);
        out.extend_from_slice(&MERGE_MINE_MAGIC);
        for b in hfc.as_bytes() {
            out.push(HEX[(b >> 4) as usize]);
            out.push(HEX[(b & 0x0f) as usize]);
        }
        out.extend_from_slice(suffix);
        out
    }

    /// A payload built exactly as `kaspa_consensus_core::auxpow::AuxPow::embed_commitment` builds it
    /// must parse back to the same hash here — the two extractors cannot drift.
    #[test]
    fn extract_matches_auxpow_embed() {
        let hfc = hfc_sample();
        let payload = embed(&[0xaa, 0xbb], hfc, &[0xcc, 0xdd]);
        assert_eq!(extract_hfc(&payload), Some(hfc));
    }

    /// Zero or two commitments are rejected (the AuxPoW anti-ambiguity rule).
    #[test]
    fn extract_requires_exactly_one_commitment() {
        assert_eq!(extract_hfc(b"no magic here"), None);

        let hfc = hfc_sample();
        let one = embed(&[], hfc, &[]);
        let mut two = one.clone();
        two.extend_from_slice(&one);
        assert_eq!(extract_hfc(&two), None, "two commitments must be rejected");
    }

    /// Build a witness whose `seq_commit` we compute independently with the seq-commit crate the way
    /// a real Kaspa block does, then confirm the witness reproduces it — for single- and
    /// multi-block mergesets — and that `verify` ties it to the right `H_fc`.
    #[test]
    fn recompute_matches_a_reference_seq_commit() {
        use kaspa_seq_commit::hashing::{miner_payload_leaf, miner_payload_root};

        let hfc = hfc_sample();
        let payload = embed(b"kaspa-cb", hfc, b"tail");
        let block_hash = Hash::from_bytes([0x33; 32]);
        let blue_work_be = vec![0x01, 0x02, 0x03];
        let activity_root = Hash::from_bytes([0x44; 32]);
        let context_hash = Hash::from_bytes([0x55; 32]);
        let parent_seq_commit = Hash::from_bytes([0x66; 32]);

        // Two sibling mergeset blocks (opaque leaves), ours inserted at index 1.
        let sib0 = Hash::from_bytes([0xA0; 32]);
        let sib1 = Hash::from_bytes([0xA1; 32]);

        let our_leaf = miner_payload_leaf(MinerPayloadLeafInput {
            block_hash: &block_hash,
            blue_work_be_bytes: &blue_work_be,
            payload: &payload,
        });

        // Reference seq_commit, computed the canonical way over the full ordered leaf list.
        let reference = {
            let ordered = vec![sib0, our_leaf, sib1];
            let payload_root = miner_payload_root(ordered.into_iter());
            let pd = payload_and_context_digest(&context_hash, &payload_root);
            let sr = seq_state_root(&SeqState { activity_root: &activity_root, payload_and_ctx_digest: &pd });
            seq_commit(&SeqCommitInput { parent_seq_commit: &parent_seq_commit, state_root: &sr })
        };

        let witness = SeqCommitWitness {
            kaspa_payload: payload,
            block_hash,
            blue_work_be,
            other_leaves: vec![sib0, sib1],
            leaf_index: 1,
            activity_root,
            context_hash,
            parent_seq_commit,
        };

        assert_eq!(witness.hfc(), Some(hfc));
        assert_eq!(witness.recompute_seq_commit().unwrap(), reference, "witness must reproduce the header seq_commit");
        assert_eq!(witness.verify(hfc, reference), Ok(true), "full canonical-R check must pass");

        // Wrong H_fc: the payload does not commit the state root the burn was proven against.
        let other_hfc = Hash::from_bytes([0x99; 32]);
        assert_eq!(witness.verify(other_hfc, reference), Ok(false));

        // Tampered opaque field ⇒ different seq_commit ⇒ would fail the on-chain OpChainblockSeqCommit.
        let mut tampered = witness.clone();
        tampered.parent_seq_commit = Hash::from_bytes([0x00; 32]);
        assert_ne!(tampered.recompute_seq_commit().unwrap(), reference);
    }

    /// The single-merged-block case: a one-leaf payload tree has root == leaf.
    #[test]
    fn single_block_mergeset_recomputes() {
        let hfc = hfc_sample();
        let payload = embed(b"", hfc, b"");
        let witness = SeqCommitWitness {
            kaspa_payload: payload,
            block_hash: Hash::from_bytes([0x33; 32]),
            blue_work_be: vec![0xFF],
            other_leaves: vec![],
            leaf_index: 0,
            activity_root: Hash::from_bytes([0x44; 32]),
            context_hash: Hash::from_bytes([0x55; 32]),
            parent_seq_commit: Hash::from_bytes([0x66; 32]),
        };
        assert!(witness.recompute_seq_commit().is_ok());
        assert_eq!(witness.verify(hfc, witness.recompute_seq_commit().unwrap()), Ok(true));
    }

    #[test]
    fn leaf_index_out_of_range_is_rejected() {
        let payload = embed(b"", hfc_sample(), b"");
        let witness = SeqCommitWitness {
            kaspa_payload: payload,
            block_hash: Hash::from_bytes([0x33; 32]),
            blue_work_be: vec![0x01],
            other_leaves: vec![Hash::from_bytes([0xA0; 32])],
            leaf_index: 5,
            activity_root: Hash::from_bytes([0x44; 32]),
            context_hash: Hash::from_bytes([0x55; 32]),
            parent_seq_commit: Hash::from_bytes([0x66; 32]),
        };
        assert_eq!(witness.recompute_seq_commit(), Err(WitnessError::LeafIndexOutOfRange));
    }
}
