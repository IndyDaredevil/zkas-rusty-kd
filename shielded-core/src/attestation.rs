//! Relayer side of the bridge: encoding a burn attestation for the Kaspa peg-out guest.
//!
//! This is the **producer** of the wire format that `vprogs_zk_abi::zkas_bridge` consumes. The
//! relayer watches this chain, and for each burn it wants to settle on Kaspa it assembles:
//!
//! - the [`ExitReceipt`](crate::burn::ExitReceipt) that was burned,
//! - its Merkle branch from the [`BurnAccumulator`](crate::burn::BurnAccumulator),
//! - the rest of the shielded-state preimage as of the block whose state root it is claiming.
//!
//! The guest **recomputes** the state root from those parts, so the relayer cannot pair a real burn
//! with an unrelated state — it can only relay burns that genuinely sit in the state it names.
//!
//! # This file is one half of a contract
//!
//! Every byte here must match the guest's decoder exactly. The two live in different repositories,
//! so they are pinned to a shared byte-level test vector (see [`tests`] and the guest's
//! `decodes_the_zkas_relayer_vector`), not merely to agreeing hash functions. A field-order or
//! endianness slip would otherwise surface as "every peg-out silently fails to verify".
//!
//! # What this does not do
//!
//! It does not establish that the claimed state root is **canonical** — that it was ever mined.
//! The guest verifies internal consistency only. See the bridge design docs for the
//! merged-mining witness that closes that gap.

use crate::burn::{BurnAccumulator, ExitReceipt};
use crate::commitment::shielded_state_root;

/// Wire version of the attestation format. Must equal the guest's `ATTESTATION_V2`.
pub const ATTESTATION_V2: u8 = 2;

/// Byte length of the fixed header preceding the Merkle branch.
///
/// `version(1) | v(8) | recipient(32) | n(32) | r(32) | leaf_index(4)
///  | anchor(32) | nullifier_root(32) | cumulative_coinbase(16) | cumulative_fees(16)`
pub const HEADER_LEN: usize = 1 + 8 + 32 + 32 + 32 + 4 + 32 + 32 + 16 + 16;

/// The shielded state a burn is attested against — the other four inputs to the state root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedStateRef {
    /// Note-commitment tree root.
    pub anchor: [u8; 32],
    /// Nullifier-set accumulator root.
    pub nullifier_root: [u8; 32],
    /// Turnstile: total subsidy ever minted into the pool.
    pub cumulative_coinbase: u128,
    /// Turnstile: total fees ever removed from the pool.
    pub cumulative_fees: u128,
}

/// Why an attestation could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationError {
    /// The requested leaf index is not in the accumulator.
    LeafOutOfRange {
        /// The requested index.
        index: u32,
        /// Number of receipts in the accumulator.
        len: usize,
    },
    /// The accumulator is deeper than the guest's `MAX_BRANCH_DEPTH` (32).
    BranchTooDeep {
        /// The accumulator's depth.
        depth: usize,
    },
}

/// Maximum branch depth the guest accepts.
pub const MAX_BRANCH_DEPTH: usize = 32;

/// Build the `ix_data` bytes attesting that the receipt at `leaf_index` was burned in `state`.
///
/// The claimed state root is *derived*, never supplied, so an encoder cannot emit an attestation
/// whose root disagrees with its own contents.
pub fn encode_attestation(
    accumulator: &BurnAccumulator,
    leaf_index: u32,
    state: &ShieldedStateRef,
) -> Result<Vec<u8>, AttestationError> {
    let receipt = *accumulator
        .receipts()
        .get(leaf_index as usize)
        .ok_or(AttestationError::LeafOutOfRange { index: leaf_index, len: accumulator.len() })?;

    let depth = accumulator.depth();
    if depth > MAX_BRANCH_DEPTH {
        return Err(AttestationError::BranchTooDeep { depth });
    }
    let branch = accumulator.branch(leaf_index).expect("index checked above");

    let r = shielded_state_root(
        &state.anchor,
        &state.nullifier_root,
        state.cumulative_coinbase,
        state.cumulative_fees,
        &accumulator.root(),
    );

    Ok(encode_parts(&receipt, &r, leaf_index, state, &branch))
}

/// Byte-level encoder, split out so tests can exercise it with hand-built parts.
fn encode_parts(
    receipt: &ExitReceipt,
    r: &[u8; 32],
    leaf_index: u32,
    state: &ShieldedStateRef,
    branch: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + branch.len());
    out.push(ATTESTATION_V2);
    out.extend_from_slice(&receipt.v.to_le_bytes());
    out.extend_from_slice(&receipt.recipient);
    out.extend_from_slice(&receipt.n);
    out.extend_from_slice(r);
    out.extend_from_slice(&leaf_index.to_le_bytes());
    out.extend_from_slice(&state.anchor);
    out.extend_from_slice(&state.nullifier_root);
    out.extend_from_slice(&state.cumulative_coinbase.to_le_bytes());
    out.extend_from_slice(&state.cumulative_fees.to_le_bytes());
    out.extend_from_slice(branch);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ShieldedStateRef {
        ShieldedStateRef {
            anchor: [0x11; 32],
            nullifier_root: [0x22; 32],
            cumulative_coinbase: 1_000_000_000,
            cumulative_fees: 12_345,
        }
    }

    fn two_burns() -> BurnAccumulator {
        BurnAccumulator::from_receipts([
            ExitReceipt { v: 5_000_000, recipient: [0xA1; 32], n: [0x11; 32] },
            ExitReceipt { v: 250_000, recipient: [0xB2; 32], n: [0x22; 32] },
        ])
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// **The cross-repository wire vector.**
    ///
    /// The Kaspa-side guest asserts it decodes *these exact bytes* and that inclusion verifies
    /// (`vprogs_zk_abi::zkas_bridge::tests::decodes_the_zkas_relayer_vector`). Encoder and decoder
    /// live in separate repositories, so this byte string — not a shared hash function — is what
    /// keeps them from drifting. If this test changes, the guest's must change with it.
    #[test]
    fn relayer_wire_vector_is_stable() {
        let acc = two_burns();
        let bytes = encode_attestation(&acc, 0, &state()).expect("leaf 0 exists");

        // header + one sibling (a two-leaf tree has depth 1)
        assert_eq!(bytes.len(), HEADER_LEN + 32);
        assert_eq!(
            hex(&bytes),
            concat!(
                "02",                                                                 // version
                "404b4c0000000000",                                                   // v = 5_000_000 LE
                "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",   // recipient
                "1111111111111111111111111111111111111111111111111111111111111111",   // n
                "4c803ec34ec41afe760122a9e84802fa93d41e38bb84f5d369a9616a8f91dfc7",   // r (state root)
                "00000000",                                                           // leaf_index = 0
                "1111111111111111111111111111111111111111111111111111111111111111",   // anchor
                "2222222222222222222222222222222222222222222222222222222222222222",   // nullifier_root
                "00ca9a3b000000000000000000000000",                                   // coinbase = 1e9, u128 LE
                "39300000000000000000000000000000",                                   // fees = 12_345, u128 LE
                "8d3b19448c8d4dd0a44fd1f75b82ff606ef048f2b121c95aeef6953a17ff271f",   // sibling = leaf 1
            )
        );
    }

    /// The claimed root is derived from the accumulator, so it always matches the burn set.
    #[test]
    fn claimed_root_matches_the_accumulator() {
        let acc = two_burns();
        let st = state();
        let bytes = encode_attestation(&acc, 1, &st).unwrap();

        let expected_r =
            shielded_state_root(&st.anchor, &st.nullifier_root, st.cumulative_coinbase, st.cumulative_fees, &acc.root());
        assert_eq!(&bytes[73..105], &expected_r, "encoded r must be the derived state root");
    }

    /// Every receipt in the accumulator must be encodable, at every size.
    #[test]
    fn every_leaf_is_encodable() {
        for n in 1..=9u32 {
            let acc = BurnAccumulator::from_receipts((0..n).map(|i| ExitReceipt {
                v: 1_000 + i as u64,
                recipient: [i as u8; 32],
                n: [(i as u8).wrapping_add(0x80); 32],
            }));
            for i in 0..n {
                let bytes = encode_attestation(&acc, i, &state()).expect("in range");
                assert_eq!(bytes.len(), HEADER_LEN + acc.depth() * 32, "n={n} i={i}");
                // The branch the guest will replay must verify against the accumulator root.
                assert!(BurnAccumulator::verify_branch(
                    &acc.receipts()[i as usize],
                    i,
                    &bytes[HEADER_LEN..],
                    &acc.root()
                ));
            }
            assert_eq!(
                encode_attestation(&acc, n, &state()),
                Err(AttestationError::LeafOutOfRange { index: n, len: n as usize })
            );
        }
    }
}
