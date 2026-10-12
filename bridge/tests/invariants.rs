//! Consensus invariant 7 (merged-bridge-v2-spec §5): the commitment magic the
//! bridge embeds MUST equal the node consensus crate's `MERGE_MINE_MAGIC`,
//! asserted against the constant itself — never a string literal.
//!
//! Why this exists: the FCMM/ZKMM incident. zkas-pool's committed Cargo.lock
//! pinned pre-rename kaspa crates, so its bridge embedded stale "FCMM" magic
//! while the node verified against "ZKMM" — 100% deterministic rejection of
//! every aux submission, masked by a healthy KAS leg. In-workspace builds make
//! that drift structurally impossible (bridge and node compile the constant
//! from the same file), and this test is the tripwire for the next rename:
//! if the magic ever changes upstream, the commitment path must follow in the
//! same commit or this fails.

use kaspa_consensus_core::auxpow::{AuxPow, MERGE_MINE_MAGIC};
use kaspa_hashes::Hash;
use kaspa_stratum_bridge::merged;

/// The workspace magic must match the magic the DEPLOYED network verifies.
/// The other tests in this file prove bridge == workspace-consensus — they
/// compile from the same file and structurally cannot drift apart, which also
/// means they cannot catch bridge+consensus drifting from the LIVE network
/// together (the FCMM failure mode, one level up: a future upstream magic
/// rename + rebase keeps every constant-based test green while live
/// zkas-v1.0.5 nodes reject every aux submission). Only this literal pin
/// covers that direction. If it ever fails, do NOT "fix" the literal until
/// the live network's nodes actually verify the new value.
#[test]
fn merge_mine_magic_matches_live_network() {
    assert_eq!(MERGE_MINE_MAGIC, *b"ZKMM");
}

/// The embedded payload contains the consensus constant exactly once, at the
/// commitment site, followed immediately by H_fc as ASCII lowercase hex
/// (64 text bytes, not 32 raw bytes — the wire format the FCMM forensics
/// showed as human-readable coinbase text: "…/FCMM6bc3694…").
#[test]
fn embedded_commitment_magic_matches_consensus_constant() {
    let h_fc = Hash::from_bytes([0xAB; 32]);
    let prefix = [1u8, 2, 3];
    let suffix = [9u8];
    let payload = AuxPow::embed_commitment(&prefix, h_fc, &suffix);

    let occurrences: Vec<usize> =
        payload.windows(MERGE_MINE_MAGIC.len()).enumerate().filter(|(_, w)| *w == MERGE_MINE_MAGIC).map(|(i, _)| i).collect();
    assert_eq!(occurrences.len(), 1, "magic must appear exactly once in the coinbase payload");

    let at = occurrences[0];
    assert_eq!(&payload[at..at + 4], &MERGE_MINE_MAGIC, "commitment magic != consensus MERGE_MINE_MAGIC");
    let hex = h_fc.to_string();
    assert_eq!(
        &payload[at + 4..at + 4 + hex.len()],
        hex.as_bytes(),
        "H_fc (ASCII lowercase hex) must immediately follow the magic"
    );
}

/// End-to-end through the bridge's own parent construction: a parent built by
/// merged::build_parent_block carries a commitment the extraction path
/// round-trips, and that commitment's magic is the consensus constant.
#[test]
fn parent_block_commitment_roundtrips_with_consensus_magic() {
    use kaspa_consensus_core::{
        block::Block,
        header::Header,
        subnets::SUBNETWORK_ID_COINBASE,
        tx::Transaction,
    };

    // Minimal zkas-shaped block: one coinbase tx, arbitrary precomputed header.
    let coinbase = Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, vec![0u8; 8]);
    let header = Header::from_precomputed_hash(Hash::from_bytes([0x11; 32]), vec![]);
    let fc_block = Block::new(header, vec![coinbase]);

    let (parent, h_fc) = merged::build_parent_block(&fc_block);
    assert_eq!(h_fc, fc_block.header.hash, "H_fc is the zkas block hash");

    let committed = merged::committed_h_fc(&parent).expect("parent coinbase must carry a commitment");
    assert_eq!(committed, h_fc, "extraction must round-trip the committed hash");

    let payload = &parent.transactions[0].payload;
    assert!(
        payload.windows(4).any(|w| w == MERGE_MINE_MAGIC),
        "parent coinbase payload must contain consensus MERGE_MINE_MAGIC"
    );
}

/// Security fork (zkas-v1.0.11) twin of `merge_mine_magic_matches_live_network`:
/// the bound-form tag the DEPLOYED network verifies from the fork. Literal on
/// purpose, for the same reason as the legacy pin above.
#[test]
fn merge_mine_magic_v1_matches_live_network() {
    assert_eq!(kaspa_consensus_core::auxpow::MERGE_MINE_MAGIC_V1, *b"ZKM1");
}

/// The mainnet activation score this binary switches forms at must be the one
/// the live network forks at. Upstream moved it twice before release
/// (8,700,500 in 23767ef, 7,000,500 in 1e68493, 6,983,500 in the tagged
/// 01ac186); a bridge built against either unreleased commit keeps every
/// constant-based test green and loses every merged block between the two
/// scores. Literal on purpose: do NOT "fix" it unless the live network's
/// nodes run a release with a different score.
#[test]
fn mainnet_security_fork_score_matches_live_network() {
    use kaspa_consensus_core::config::params::MAINNET_PARAMS;
    assert_eq!(MAINNET_PARAMS.security_fork_activation.daa_score(), 6_983_500);
}

/// ZKM1 twin of `embedded_commitment_magic_matches_consensus_constant`: the
/// bound payload carries the v1 constant exactly once, followed immediately by
/// the commitment as ASCII lowercase hex, and carries NO legacy magic (the
/// bound rule refuses a parent that has one).
#[test]
fn embedded_bound_commitment_magic_matches_consensus_constant() {
    use kaspa_consensus_core::auxpow::MERGE_MINE_MAGIC_V1;
    let commitment = Hash::from_bytes([0xAB; 32]);
    let prefix = [1u8, 2, 3];
    let suffix = [9u8];
    let payload = AuxPow::embed_commitment_v1(&prefix, commitment, &suffix);

    let occurrences: Vec<usize> =
        payload.windows(MERGE_MINE_MAGIC_V1.len()).enumerate().filter(|(_, w)| *w == MERGE_MINE_MAGIC_V1).map(|(i, _)| i).collect();
    assert_eq!(occurrences.len(), 1, "v1 magic must appear exactly once in the coinbase payload");
    assert_eq!(payload.windows(MERGE_MINE_MAGIC.len()).filter(|w| *w == MERGE_MINE_MAGIC).count(), 0, "no legacy magic beside it");

    let at = occurrences[0];
    let hex = commitment.to_string();
    assert_eq!(
        &payload[at + 4..at + 4 + hex.len()],
        hex.as_bytes(),
        "the commitment (ASCII lowercase hex) must immediately follow the magic"
    );
}
