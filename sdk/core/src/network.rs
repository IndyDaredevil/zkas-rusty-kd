use serde::{Deserialize, Serialize};

/// Supported ZKas network names. Custom networks must supply their own trusted
/// genesis domain through [`NetworkConfig::custom`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Simnet,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    pub network: Network,
    /// Trusted shielded sighash domain (the chain genesis hash).
    pub genesis: [u8; 32],
    /// Hold this many blue-score units behind the sink before committing wallet
    /// effects. The current production default is 200.
    pub settlement_blue_score: u64,
    /// Consensus shielded-anchor maturity depth. Current mainnet value is 600.
    pub anchor_depth: u64,
}

impl NetworkConfig {
    // ZKas mainnet reset (2026-07-26, Bitcoin-anchored fair-launch), genesis
    // f6131c051593df7c631794a58e6dcab5e5b8864181b9368084675f1caaeb7703.
    // MUST equal `MAINNET_PARAMS.genesis.hash` in consensus; re-cut at launch.
    pub const MAINNET_GENESIS: [u8; 32] = [
        0xb6, 0x3f, 0x7f, 0xe8, 0xe5, 0x04, 0x02, 0xaf, 0x34, 0x79, 0x02, 0x65, 0xe2, 0x99, 0xbb, 0x1b, 0xa6, 0x3e, 0x94, 0x3b, 0x91,
        0xa5, 0x9a, 0x67, 0x0e, 0x59, 0x71, 0xb7, 0xa9, 0xe8, 0x4e, 0x6f,
    ];

    pub const fn mainnet() -> Self {
        Self { network: Network::Mainnet, genesis: Self::MAINNET_GENESIS, settlement_blue_score: 200, anchor_depth: 600 }
    }

    pub fn custom(name: impl Into<String>, genesis: [u8; 32], settlement_blue_score: u64, anchor_depth: u64) -> Self {
        Self { network: Network::Custom(name.into()), genesis, settlement_blue_score, anchor_depth }
    }
}
