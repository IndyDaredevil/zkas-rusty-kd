//! The burn-exit seam: the authorised way value leaves the shielded pool for another chain.
//!
//! # What a burn is
//!
//! Fees are the only *existing* way value leaves the pool ([`crate::turnstile`]). A **burn** is a
//! second, equally public exit: value is destroyed on the ZKas side and a public [`ExitReceipt`]
//! records who may claim the mirrored value on Kaspa. The supply invariant widens from
//! `pool = coinbase − fees` to:
//!
//! ```text
//! pool = cumulative_coinbase − cumulative_fees − cumulative_burns  ≥ 0
//! ```
//!
//! Global supply is conserved across the two chains: exactly the value that leaves here is the
//! value that becomes claimable there. Nothing is minted anywhere.
//!
//! # Why this is safe to add to the most safety-critical invariant in the chain
//!
//! A burn is *mechanically the same operation as paying a fee* — an amount leaving the pool —
//! with a recorded destination. It rides the same crypto pinning: a bundle's `value_balance` is
//! the amount the binding signature proves is leaving, and consensus splits that declared amount
//! into `fee + burn`. So a burn cannot move value the binding signature did not authorise, exactly
//! as a fee cannot. This is a new *authorised seam*, not a hole in the turnstile.
//!
//! # The wire contract is shared with the Kaspa side
//!
//! [`ExitReceipt::leaf_hash`] and [`BurnAccumulator`] must stay **byte-identical** to the peg-out
//! guest's `vprogs_zk_abi::zkas_bridge`, because the guest proves Merkle inclusion against the
//! root computed here. Any drift silently breaks every peg-out. Both sides are pinned to the same
//! independently-computed test vectors (see the tests below and the guest's own suite), so drift
//! fails a test rather than a withdrawal.
//!
//! Hashing is SHA-256 rather than the chain's usual blake2b **because the verifier is the risc0
//! guest**, and SHA-256 is what the shared ABI's `Hasher` uses.

use sha2::{Digest, Sha256};

/// **Master switch for the KAS⇄ZKAS bridge (both peg directions).**
///
/// The bridge is *deactivated* while this is `false`: consensus rejects any transaction that
/// declares a peg-out burn (see `check_shielded_in_isolation` and [`crate::state::ShieldedTx::from_bundle`]),
/// and the peg-in mint seam ([`crate::turnstile::SupplyLedger::peg_in`], `kaspa_pow::pegin`) is never
/// wired into the state transition. With no burn ever admitted, the [`BurnAccumulator`] stays empty
/// and its root is a constant, so a chain with the bridge off is byte-identical to one that never had
/// the seam — flipping this flag is therefore a *tightening*, safe to deploy without a chain reset as
/// long as no burn has ever been mined.
///
/// **Why off:** a trustless peg needs a settlement primitive Kaspa does not yet expose natively (a
/// covenant cannot mint/track a token amount, only KAS value). Shipping a half-built peg-out would let
/// users irrevocably burn ZKAS against a bridge that cannot honour it. Keep this `false` until Kaspa
/// ships the settlement support, then flip to `true` (or replace with an activation-score gate) and
/// re-enable in one place.
pub const BRIDGE_ENABLED: bool = false;

/// Domain byte for exit-receipt leaves.
pub const LEAF_DOMAIN: u8 = 0x00;
/// Domain byte for interior nodes.
pub const NODE_DOMAIN: u8 = 0x01;

/// Hash of an empty subtree at level 0. Levels above are folded from this.
const EMPTY_LEAF: [u8; 32] = [0u8; 32];

/// A single peg-out authorised by a burn: `v` sompi left the pool, claimable by `recipient` on
/// Kaspa, keyed by the burned note's nullifier `n`.
///
/// `n` is the replay key. The Kaspa-side guest binds the claiming transaction's resource slot to
/// it, so one burn yields at most one payout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitReceipt {
    /// Amount burned, in sompi.
    pub v: u64,
    /// Kaspa recipient: the 32-byte Schnorr public key the mirrored value pays out to.
    pub recipient: [u8; 32],
    /// Exit-nullifier of the burned note.
    pub n: [u8; 32],
}

impl ExitReceipt {
    /// Leaf hash: `sha256(LEAF_DOMAIN || v_le || recipient || n)`.
    pub fn leaf_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update([LEAF_DOMAIN]);
        h.update(self.v.to_le_bytes());
        h.update(self.recipient);
        h.update(self.n);
        h.finalize().into()
    }
}

/// Interior node hash: `sha256(NODE_DOMAIN || left || right)`.
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([NODE_DOMAIN]);
    h.update(left);
    h.update(right);
    h.finalize().into()
}

/// Append-only Merkle accumulator over [`ExitReceipt`]s.
///
/// The tree is padded to the next power of two with empty subtrees, so the root is a pure function
/// of the receipt sequence. A [`branch`](Self::branch) authenticates one receipt against
/// [`root`](Self::root), and is exactly what the peg-out guest replays.
#[derive(Debug, Clone, Default)]
pub struct BurnAccumulator {
    leaves: Vec<[u8; 32]>,
    receipts: Vec<ExitReceipt>,
}

impl BurnAccumulator {
    /// An empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild from a persisted receipt sequence (replay / restart).
    pub fn from_receipts(receipts: impl IntoIterator<Item = ExitReceipt>) -> Self {
        let mut acc = Self::new();
        for r in receipts {
            acc.push(r);
        }
        acc
    }

    /// Append a receipt, returning its leaf index.
    pub fn push(&mut self, receipt: ExitReceipt) -> u32 {
        let index = self.leaves.len() as u32;
        self.leaves.push(receipt.leaf_hash());
        self.receipts.push(receipt);
        index
    }

    /// Number of receipts accumulated.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether no receipts have been accumulated.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// The receipts, in order.
    pub fn receipts(&self) -> &[ExitReceipt] {
        &self.receipts
    }

    /// Tree depth: `ceil(log2(len))`, and 0 for an empty or single-leaf tree.
    pub fn depth(&self) -> usize {
        let n = self.leaves.len();
        if n <= 1 {
            return 0;
        }
        (usize::BITS - (n - 1).leading_zeros()) as usize
    }

    /// The accumulator root. An empty accumulator has the all-zero root, which doubles as the
    /// "no burns yet" sentinel in the state root.
    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return EMPTY_LEAF;
        }
        let mut level = self.leaves.clone();
        let mut empty = EMPTY_LEAF;
        for _ in 0..self.depth() {
            if level.len() % 2 == 1 {
                level.push(empty);
            }
            level = level.chunks_exact(2).map(|p| node_hash(&p[0], &p[1])).collect();
            empty = node_hash(&empty, &empty);
        }
        level[0]
    }

    /// The sibling path authenticating leaf `index`, leaf-to-root, as concatenated 32-byte hashes.
    ///
    /// Returns `None` if `index` is out of range. The result is exactly `depth() * 32` bytes and
    /// is what goes on the wire to the peg-out guest.
    pub fn branch(&self, index: u32) -> Option<Vec<u8>> {
        if index as usize >= self.leaves.len() {
            return None;
        }
        let mut out = Vec::with_capacity(self.depth() * 32);
        let mut level = self.leaves.clone();
        let mut empty = EMPTY_LEAF;
        let mut idx = index as usize;
        for _ in 0..self.depth() {
            if level.len() % 2 == 1 {
                level.push(empty);
            }
            // Sibling is the other half of our pair.
            let sibling = if idx % 2 == 0 { level[idx + 1] } else { level[idx - 1] };
            out.extend_from_slice(&sibling);
            level = level.chunks_exact(2).map(|p| node_hash(&p[0], &p[1])).collect();
            empty = node_hash(&empty, &empty);
            idx /= 2;
        }
        Some(out)
    }

    /// Replays a branch the way the peg-out guest does, for host-side self-checks and tests.
    pub fn verify_branch(receipt: &ExitReceipt, index: u32, branch: &[u8], root: &[u8; 32]) -> bool {
        if branch.len() % 32 != 0 {
            return false;
        }
        let mut acc = receipt.leaf_hash();
        for (level, sibling) in branch.chunks_exact(32).enumerate() {
            let sib: [u8; 32] = sibling.try_into().expect("32 bytes");
            acc = if (index >> level) & 1 == 1 { node_hash(&sib, &acc) } else { node_hash(&acc, &sib) };
        }
        &acc == root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_a() -> ExitReceipt {
        ExitReceipt { v: 5_000_000, recipient: [0xA1; 32], n: [0x11; 32] }
    }
    fn receipt_b() -> ExitReceipt {
        ExitReceipt { v: 250_000, recipient: [0xB2; 32], n: [0x22; 32] }
    }

    /// **Cross-implementation pin.** These vectors were computed by a third, independent
    /// implementation (Python `hashlib`), and the identical values are asserted by the Kaspa-side
    /// peg-out guest's test suite. If ZKas and the guest ever drift in how they hash a receipt,
    /// every peg-out would silently stop verifying — this test fails first instead.
    #[test]
    fn hashing_matches_the_shared_wire_contract() {
        assert_eq!(hex(&receipt_a().leaf_hash()), "27d9600250d03d6eea797a5a9d4a6c8ba66209121a104c8108412dc968ba7302",);
        assert_eq!(hex(&receipt_b().leaf_hash()), "8d3b19448c8d4dd0a44fd1f75b82ff606ef048f2b121c95aeef6953a17ff271f",);

        let acc = BurnAccumulator::from_receipts([receipt_a(), receipt_b()]);
        assert_eq!(hex(&acc.root()), "4a457e6dd8976c5b52b7d7337244ef6478c87e916bdec6465d730ecd5e7fe5d3");

        // Empty-subtree hash at level 1, used when padding an odd level.
        assert_eq!(hex(&node_hash(&EMPTY_LEAF, &EMPTY_LEAF)), "ae0798d0ecaed2b778eddebf18f071a561c53658c05e76cedecc27cafbdbc577",);
    }

    fn hex(b: &[u8; 32]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn empty_accumulator_has_zero_root() {
        let acc = BurnAccumulator::new();
        assert!(acc.is_empty());
        assert_eq!(acc.root(), EMPTY_LEAF);
        assert_eq!(acc.depth(), 0);
    }

    #[test]
    fn single_leaf_root_is_the_leaf() {
        let acc = BurnAccumulator::from_receipts([receipt_a()]);
        assert_eq!(acc.depth(), 0);
        assert_eq!(acc.root(), receipt_a().leaf_hash());
        // A depth-0 tree needs no siblings.
        assert_eq!(acc.branch(0).unwrap(), Vec::<u8>::new());
    }

    /// Every receipt in a tree must authenticate against the root, at every size — including the
    /// odd sizes where padding kicks in.
    #[test]
    fn every_branch_verifies_at_every_size() {
        for n in 1..=9u64 {
            let receipts: Vec<_> = (0..n)
                .map(|i| ExitReceipt { v: 1_000 + i, recipient: [i as u8; 32], n: [(i as u8).wrapping_add(0x80); 32] })
                .collect();
            let acc = BurnAccumulator::from_receipts(receipts.clone());
            let root = acc.root();
            for (i, r) in receipts.iter().enumerate() {
                let branch = acc.branch(i as u32).expect("in range");
                assert_eq!(branch.len(), acc.depth() * 32, "branch length must equal depth (n={n}, i={i})");
                assert!(BurnAccumulator::verify_branch(r, i as u32, &branch, &root), "receipt {i} of {n} must verify",);
            }
            assert!(acc.branch(n as u32).is_none(), "out-of-range index must have no branch");
        }
    }

    /// A branch must not authenticate a receipt with a tampered amount or recipient — this is what
    /// stops a relayer inflating or redirecting a real burn.
    #[test]
    fn tampered_receipts_do_not_verify() {
        let acc = BurnAccumulator::from_receipts([receipt_a(), receipt_b()]);
        let root = acc.root();
        let branch = acc.branch(0).unwrap();

        let mut inflated = receipt_a();
        inflated.v = 999_999_999;
        assert!(!BurnAccumulator::verify_branch(&inflated, 0, &branch, &root));

        let mut redirected = receipt_a();
        redirected.recipient = [0xFF; 32];
        assert!(!BurnAccumulator::verify_branch(&redirected, 0, &branch, &root));

        // Right receipt, wrong position.
        assert!(!BurnAccumulator::verify_branch(&receipt_a(), 1, &branch, &root));
    }

    /// Appending must change the root: a burn that did not move the root would be invisible to the
    /// state commitment, and so unprovable on the Kaspa side.
    #[test]
    fn appending_changes_the_root() {
        let mut acc = BurnAccumulator::new();
        let r0 = acc.root();
        acc.push(receipt_a());
        let r1 = acc.root();
        acc.push(receipt_b());
        let r2 = acc.root();
        assert_ne!(r0, r1);
        assert_ne!(r1, r2);
    }

    /// Leaf and node domains must be disjoint, or an interior node could be passed off as a
    /// receipt (the classic Merkle second-preimage attack).
    #[test]
    fn leaf_and_node_domains_are_disjoint() {
        let mut leafish = Sha256::new();
        leafish.update([LEAF_DOMAIN]);
        leafish.update([0xCD; 64]);
        let mut nodeish = Sha256::new();
        nodeish.update([NODE_DOMAIN]);
        nodeish.update([0xCD; 64]);
        let l: [u8; 32] = leafish.finalize().into();
        let n: [u8; 32] = nodeish.finalize().into();
        assert_ne!(l, n);
    }
}
