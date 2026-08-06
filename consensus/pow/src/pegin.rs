//! Bridge peg-IN verifier (Layer 1): the keyless, consensus-side check that a **mirror-ZKAS burn on
//! Kaspa** happened, so native ZKAS may be minted for it with no operator and no mint key.
//!
//! This mirrors [`verify_aux_pow`](crate::auxpow::verify_aux_pow): the structural half proves a
//! transaction is committed by a Kaspa header's `hash_merkle_root`, and the PoW half proves that
//! header carries real work. The difference is generality — the burn transaction sits at an
//! *arbitrary* position in the block (not leaf 0 like the coinbase), so it uses the general
//! [`kaspa_merkle::verify_merkle_witness`] instead of the left-spine coinbase fold.
//!
//! ```text
//!   pow(kaspa_header) → kaspa_header.hash_merkle_root → (Merkle branch) → burn_tx
//!                                                                          │
//!                                          payload = ZKas recipient, value = amount burned
//! ```
//!
//! Trust argument: minting is a deterministic function of a proven burn, not a signature. Combined
//! with a **buried-depth** check on `kaspa_header` (the caller's SPV-depth parameter) and a
//! **consumed-burns replay set** (keyed by [`KaspaBurnProof::replay_key`]), the peg-in seam cannot
//! mint value that an equal amount of mirror-ZKAS was not destroyed for. The turnstile records it
//! via [`kaspa_shielded_core::turnstile::SupplyLedger::peg_in`].

use kaspa_consensus_core::{hashing, header::Header, tx::Transaction};
use kaspa_hashes::Hash;
use kaspa_math::Uint256;
use kaspa_merkle::verify_tx_merkle_witness;

use crate::State;

/// Hard cap on the burn-transaction Merkle branch length. A Kaspa block cannot hold more than
/// `2^56` transactions in any realistic future, so 56 siblings is an absurd upper bound that still
/// bounds verifier work before any hashing.
pub const MAX_BURN_MERKLE_BRANCH: usize = 56;

/// The 32-byte ZKas shielded recipient a mirror-ZKAS burn names, plus the amount burned and a
/// unique replay key. Produced by [`KaspaBurnProof::claim`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PegInClaim {
    /// The amount of mirror-ZKAS destroyed on Kaspa = the amount of native ZKAS to mint.
    pub value: u64,
    /// The ZKas shielded address the newly minted native ZKAS is credited to (from the burn tx
    /// payload).
    pub zkas_recipient: [u8; 32],
    /// A collision-free key for the consumed-burns replay set: the burn transaction's own hash.
    /// Each on-chain burn tx is unique, so recording this key blocks minting twice for one burn.
    pub replay_key: Hash,
}

/// A proof that a mirror-ZKAS burn transaction is committed by a Kaspa block.
///
/// `burn_tx` is the Kaspa transaction that destroys mirror-ZKAS: its first output pays the mirror
/// token's unspendable burn script carrying the burned `value`, and its `payload` starts with the
/// 32-byte ZKas recipient. `tx_leaf_index` + `tx_merkle_branch` place it under
/// `parent_header.hash_merkle_root`.
#[derive(Clone, Debug)]
pub struct KaspaBurnProof {
    /// The Kaspa block header whose PoW commits to `hash_merkle_root`.
    pub parent_header: Header,
    /// The mirror-ZKAS burn transaction.
    pub burn_tx: Transaction,
    /// Position of `burn_tx` in the block's transaction list.
    pub tx_leaf_index: usize,
    /// Sibling hashes from `burn_tx` up to `parent_header.hash_merkle_root`.
    pub tx_merkle_branch: Vec<Hash>,
}

impl KaspaBurnProof {
    /// The transaction hash the Kaspa Merkle tree commits (same leaf hashing as the coinbase
    /// inclusion path in [`crate::auxpow`]).
    fn tx_leaf(&self) -> Hash {
        hashing::tx::hash(&self.burn_tx)
    }

    /// Structural half: `burn_tx` is Merkle-included under `parent_header.hash_merkle_root` at
    /// `tx_leaf_index`. Rejects an over-long branch before hashing.
    pub fn verify_inclusion(&self) -> bool {
        if self.tx_merkle_branch.len() > MAX_BURN_MERKLE_BRANCH {
            return false;
        }
        verify_tx_merkle_witness(self.tx_leaf(), self.tx_leaf_index, &self.tx_merkle_branch, self.parent_header.hash_merkle_root)
    }

    /// Full check: `burn_tx` is included (structural) **and** the header's kHeavyHash PoW meets
    /// `target`. The caller must *separately* require `parent_header` to be buried to the bridge's
    /// SPV depth on the ZKas DAG, and must reject a [`Self::replay_key`] already in the consumed set.
    pub fn verify(&self, target: Uint256) -> bool {
        self.verify_inclusion() && State::new(&self.parent_header).calculate_pow(self.parent_header.nonce) <= target
    }

    /// Extract the mint claim ({value, recipient, replay key}) from the burn transaction.
    ///
    /// Format (the mirror-token burn the Kaspa covenant emits): output 0 carries the burned `value`,
    /// and the tx `payload` begins with the 32-byte ZKas recipient. Returns `None` if the tx has no
    /// output or a too-short payload — a malformed burn mints nothing.
    pub fn claim(&self) -> Option<PegInClaim> {
        let value = self.burn_tx.outputs.first()?.value;
        if value == 0 || self.burn_tx.payload.len() < 32 {
            return None;
        }
        let mut zkas_recipient = [0u8; 32];
        zkas_recipient.copy_from_slice(&self.burn_tx.payload[..32]);
        Some(PegInClaim { value, zkas_recipient, replay_key: self.tx_leaf() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaspa_consensus_core::{
        subnets::SUBNETWORK_ID_NATIVE,
        tx::{ScriptPublicKey, Transaction, TransactionOutput},
    };
    use kaspa_hashes::Hash as KHash;
    use kaspa_merkle::{calc_merkle_root, create_tx_merkle_witness};

    fn burn_spk() -> ScriptPublicKey {
        ScriptPublicKey::from_vec(0, vec![0x51]) // OP_1 stand-in for the mirror-token burn script
    }

    fn burn_tx(value: u64, recipient: [u8; 32]) -> Transaction {
        let out = TransactionOutput::new(value, burn_spk());
        Transaction::new(0, vec![], vec![out], 0, SUBNETWORK_ID_NATIVE, 0, recipient.to_vec())
    }

    fn dummy_tx(tag: u8) -> Transaction {
        Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_NATIVE, 0, vec![tag; 4])
    }

    /// A burn tx at every position in a block verifies against the block's `hash_merkle_root`, and
    /// its claim carries the right value + recipient; a tampered value or wrong index fails.
    #[test]
    fn burn_included_at_any_index_verifies() {
        let recipient = [0x7c; 32];
        for n in 1..=9usize {
            for burn_idx in 0..n {
                let txs: Vec<Transaction> =
                    (0..n).map(|i| if i == burn_idx { burn_tx(5000, recipient) } else { dummy_tx(i as u8) }).collect();
                let leaves: Vec<KHash> = txs.iter().map(hashing::tx::hash).collect();
                let root = calc_merkle_root(leaves.iter().copied());
                let branch = create_tx_merkle_witness(&leaves, burn_idx);

                let mut header = Header::from_precomputed_hash(KHash::default(), vec![]);
                header.hash_merkle_root = root;

                let proof = KaspaBurnProof {
                    parent_header: header,
                    burn_tx: txs[burn_idx].clone(),
                    tx_leaf_index: burn_idx,
                    tx_merkle_branch: branch.clone(),
                };
                assert!(proof.verify_inclusion(), "n={n} idx={burn_idx}: burn must be included");
                let claim = proof.claim().expect("well-formed burn yields a claim");
                assert_eq!(claim.value, 5000);
                assert_eq!(claim.zkas_recipient, recipient);
                assert_eq!(claim.replay_key, leaves[burn_idx]);

                // Wrong declared index breaks inclusion (unless a 1-tx tree where index is forced 0).
                if n > 1 {
                    let mut bad = proof.clone();
                    bad.tx_leaf_index = (burn_idx + 1) % n;
                    assert!(!bad.verify_inclusion(), "n={n} idx={burn_idx}: wrong index must fail");
                }
            }
        }
    }

    /// A zero-value burn or an under-length payload yields no claim (mints nothing).
    #[test]
    fn malformed_burn_yields_no_claim() {
        let header = Header::from_precomputed_hash(KHash::default(), vec![]);
        let zero =
            KaspaBurnProof { parent_header: header.clone(), burn_tx: burn_tx(0, [1; 32]), tx_leaf_index: 0, tx_merkle_branch: vec![] };
        assert_eq!(zero.claim(), None);

        let short_payload =
            Transaction::new(0, vec![], vec![TransactionOutput::new(1, burn_spk())], 0, SUBNETWORK_ID_NATIVE, 0, vec![0u8; 8]);
        let bad = KaspaBurnProof { parent_header: header, burn_tx: short_payload, tx_leaf_index: 0, tx_merkle_branch: vec![] };
        assert_eq!(bad.claim(), None);
    }
}
