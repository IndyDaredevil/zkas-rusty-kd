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
        0x01, 0x44, 0x19, 0x2e, 0x1d, 0x5a, 0x9b, 0x40, 0xf6, 0x11, 0x8b, 0x31, 0x7b, 0xac, 0x72, 0x19, 0xac, 0x61, 0xd0, 0xb7, 0x11,
        0xb5, 0xaf, 0x17, 0x17, 0x81, 0x5e, 0xa6, 0x77, 0xfa, 0x68, 0x27,
    ];

    pub const fn mainnet() -> Self {
        Self { network: Network::Mainnet, genesis: Self::MAINNET_GENESIS, settlement_blue_score: 200, anchor_depth: 600 }
    }

    pub fn custom(name: impl Into<String>, genesis: [u8; 32], settlement_blue_score: u64, anchor_depth: u64) -> Self {
        Self { network: Network::Custom(name.into()), genesis, settlement_blue_score, anchor_depth }
    }
}
