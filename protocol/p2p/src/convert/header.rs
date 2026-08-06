use crate::pb as protowire;
use kaspa_consensus_core::{
    BlueWorkType,
    auxpow::AuxPow,
    header::{CompressedParents, Header},
};
use kaspa_hashes::Hash;

use super::error::ConversionError;
use super::option::TryIntoOptionEx;

#[derive(Copy, Clone)]
pub enum HeaderFormat {
    Legacy,
    Compressed,
}

/// Determines the header format based on the protocol version.
impl From<u32> for HeaderFormat {
    fn from(version: u32) -> Self {
        if version >= 9 { Self::Compressed } else { Self::Legacy }
    }
}

// ----------------------------------------------------------------------------
// consensus_core to protowire
// ----------------------------------------------------------------------------

impl From<(HeaderFormat, &Header)> for protowire::BlockHeader {
    fn from(value: (HeaderFormat, &Header)) -> Self {
        let (header_type, item) = value;

        Self {
            version: item.version.into(),
            parents: match header_type {
                HeaderFormat::Legacy => item.parents_by_level.expanded_iter().map(protowire::BlockLevelParents::from).collect(),
                HeaderFormat::Compressed => item
                    .parents_by_level
                    .raw()
                    .iter()
                    .map(|(cum, hashes)| protowire::BlockLevelParents {
                        cumulative_level: (*cum).into(),
                        parent_hashes: hashes.iter().map(|h| h.into()).collect(),
                    })
                    .collect(),
            },
            hash_merkle_root: Some(item.hash_merkle_root.into()),
            accepted_id_merkle_root: Some(item.accepted_id_merkle_root.into()),
            utxo_commitment: Some(item.utxo_commitment.into()),
            timestamp: item.timestamp.try_into().expect("timestamp is always convertible to i64"),
            bits: item.bits,
            nonce: item.nonce,
            daa_score: item.daa_score,
            // We follow the golang specification of variable big-endian here
            blue_work: item.blue_work.to_be_bytes_var(),
            blue_score: item.blue_score,
            pruning_point: Some(item.pruning_point.into()),
            // Merged-mining witness, borsh-encoded; empty for natively-mined blocks.
            aux_pow: item.aux_pow.as_ref().map(|a| borsh::to_vec(a).expect("AuxPow is always borsh-serializable")).unwrap_or_default(),
        }
    }
}

impl From<&[Hash]> for protowire::BlockLevelParents {
    fn from(item: &[Hash]) -> Self {
        // When converting to legacy p2p header, cumulative_level is set to 0
        Self { parent_hashes: item.iter().map(|h| h.into()).collect(), cumulative_level: 0 }
    }
}

// ----------------------------------------------------------------------------
// protowire to consensus_core
// ----------------------------------------------------------------------------

/// A wrapper for P2P header messages indicating the expected header format during conversion.
pub struct Versioned<T>(pub HeaderFormat, pub T);

impl TryFrom<Versioned<protowire::BlockHeader>> for Header {
    type Error = ConversionError;
    fn try_from(value: Versioned<protowire::BlockHeader>) -> Result<Self, Self::Error> {
        let Versioned(header_format, item) = value;

        let parents_by_level = match header_format {
            HeaderFormat::Compressed => item
                .parents
                .into_iter()
                .map(|p| {
                    let cum = u8::try_from(p.cumulative_level)?;
                    let parents = p.parent_hashes.into_iter().map(Hash::try_from).collect::<Result<_, _>>()?;
                    Ok((cum, parents))
                })
                .collect::<Result<Vec<(u8, Vec<Hash>)>, ConversionError>>()?
                .try_into()?,
            HeaderFormat::Legacy => item
                .parents
                .into_iter()
                .map(|p| p.parent_hashes.into_iter().map(Hash::try_from).collect::<Result<Vec<Hash>, ConversionError>>())
                .collect::<Result<Vec<Vec<Hash>>, ConversionError>>()?
                .try_into()?,
        };

        let header = Header::new_finalized(
            item.version.try_into()?,
            parents_by_level,
            item.hash_merkle_root.try_into_ex()?,
            item.accepted_id_merkle_root.try_into_ex()?,
            item.utxo_commitment.try_into_ex()?,
            item.timestamp.try_into()?,
            item.bits,
            item.nonce,
            item.daa_score,
            // We follow the golang specification of variable big-endian here
            BlueWorkType::from_be_bytes_var(&item.blue_work)?,
            item.blue_score,
            item.pruning_point.try_into_ex()?,
        );
        // Reattach the merged-mining witness if present. It is not part of the header
        // hash, so `with_aux_pow` does not disturb the `H_fc` computed above.
        if item.aux_pow.is_empty() {
            Ok(header)
        } else {
            let aux: AuxPow = borsh::from_slice(&item.aux_pow).map_err(|e| ConversionError::AuxPowDecodeError(e.to_string()))?;
            // Drop an over-long Merkle branch at the edge, before it can be stored or
            // relayed: no honest parent block needs more than MAX_COINBASE_MERKLE_BRANCH
            // levels, and the field is otherwise bounded only by the p2p message size.
            if aux.coinbase_merkle_branch.len() > kaspa_consensus_core::auxpow::MAX_COINBASE_MERKLE_BRANCH {
                return Err(ConversionError::AuxPowDecodeError(format!(
                    "coinbase merkle branch too long: {} > {}",
                    aux.coinbase_merkle_branch.len(),
                    kaspa_consensus_core::auxpow::MAX_COINBASE_MERKLE_BRANCH
                )));
            }
            // F-07: borsh-decoding the aux witness bypasses the `TryFrom` validation
            // the main header's parents went through above. The aux parent header is
            // later hashed (`parent_pow` in kaspa_pow), and hashing expands
            // `parents_by_level`, whose `expand_rle` PANICS on non-strictly-increasing
            // cumulative counts — a remote zero-work panic in header processing.
            // Re-run the same CompressedParents validation at the edge, before the
            // witness can be stored or relayed.
            CompressedParents::try_from(aux.parent_header.parents_by_level.raw().to_vec())
                .map_err(|e| ConversionError::AuxPowDecodeError(format!("aux parent parents_by_level invalid: {e}")))?;
            Ok(header.with_aux_pow(aux))
        }
    }
}

impl TryFrom<protowire::BlockLevelParents> for Vec<Hash> {
    type Error = ConversionError;
    fn try_from(item: protowire::BlockLevelParents) -> Result<Self, Self::Error> {
        item.parent_hashes.into_iter().map(|x| x.try_into()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaspa_consensus_core::{auxpow::AuxPow, subnets::SUBNETWORK_ID_COINBASE, tx::Transaction};

    fn finalized(parent_seed: u8) -> Header {
        let mut h = Header::from_precomputed_hash(Default::default(), vec![Hash::from_bytes([parent_seed; 32])]);
        h.finalize();
        h
    }

    #[test]
    fn aux_pow_survives_p2p_round_trip() {
        let header = finalized(1);
        let hfc = header.hash;
        // Minimal valid-shaped aux: coinbase commits to H_fc (structural checks live in
        // the consensus crates; here we only exercise transport).
        let cb = Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, AuxPow::embed_commitment(&[], hfc, &[]));
        let aux = AuxPow { parent_header: finalized(9), parent_coinbase: cb, coinbase_merkle_branch: vec![] };
        let header = header.with_aux_pow(aux);

        let pb: protowire::BlockHeader = (HeaderFormat::Compressed, &header).into();
        assert!(!pb.aux_pow.is_empty(), "aux is borsh-encoded into the protobuf bytes field");

        let back: Header = Versioned(HeaderFormat::Compressed, pb).try_into().unwrap();
        assert_eq!(back.hash, hfc, "H_fc is stable across the p2p round trip");
        assert_eq!(back.aux_pow.as_ref().expect("aux survives p2p").committed_hash(), Some(hfc));
    }

    #[test]
    fn native_header_round_trips_without_aux() {
        let native = finalized(2);
        let pb: protowire::BlockHeader = (HeaderFormat::Compressed, &native).into();
        assert!(pb.aux_pow.is_empty(), "native header carries no aux bytes");
        let back: Header = Versioned(HeaderFormat::Compressed, pb).try_into().unwrap();
        assert!(back.aux_pow.is_none());
        assert_eq!(back.hash, native.hash);
    }

    /// F-07 regression: the aux witness is borsh-decoded, which bypasses the
    /// `CompressedParents` validation the main header's parents go through. An aux
    /// parent whose `parents_by_level` cumulative counts are not strictly increasing
    /// later PANICS `expand_rle` during parent_pow hashing — a remote zero-work
    /// panic. The p2p edge must reject it with a ConversionError instead.
    #[test]
    fn aux_parent_with_invalid_parents_by_level_is_rejected() {
        let header = finalized(1);
        let h1 = Hash::from_bytes([7u8; 32]);
        let h2 = Hash::from_bytes([8u8; 32]);

        let cases: Vec<Vec<(u8, Vec<Hash>)>> = vec![
            vec![(0, vec![h1])],                // first cumulative count is 0
            vec![(5, vec![h1]), (2, vec![h1])], // decreasing cumulative counts
            vec![(3, vec![h1]), (3, vec![h2])], // repeated cumulative count
        ];
        for raw in cases {
            // Craft the invalid CompressedParents exactly the way the wire path
            // receives it: borsh decode performs no validation.
            let bad: CompressedParents = borsh::from_slice(&borsh::to_vec(&raw).unwrap()).unwrap();
            let mut parent = finalized(9);
            parent.parents_by_level = bad;
            let cb =
                Transaction::new(0, vec![], vec![], 0, SUBNETWORK_ID_COINBASE, 0, AuxPow::embed_commitment(&[], header.hash, &[]));
            let aux = AuxPow { parent_header: parent, parent_coinbase: cb, coinbase_merkle_branch: vec![] };

            let mut pb: protowire::BlockHeader = (HeaderFormat::Compressed, &header).into();
            pb.aux_pow = borsh::to_vec(&aux).unwrap();

            let res: Result<Header, ConversionError> = Versioned(HeaderFormat::Compressed, pb).try_into();
            assert!(
                matches!(res, Err(ConversionError::AuxPowDecodeError(_))),
                "case {raw:?} must be rejected with AuxPowDecodeError, not accepted or panicked"
            );
        }
    }
}
