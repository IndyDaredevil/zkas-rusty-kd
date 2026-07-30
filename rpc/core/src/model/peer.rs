use borsh::{BorshDeserialize, BorshSerialize};
use kaspa_utils::networking::{ContextualNetAddress, IpAddress, NetAddress, PeerId};
use serde::{Deserialize, Serialize};

pub type RpcNodeId = PeerId;
pub type RpcIpAddress = IpAddress;
pub type RpcPeerAddress = NetAddress;
pub type RpcContextualPeerAddress = ContextualNetAddress;

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct RpcPeerInfo {
    pub id: RpcNodeId,
    pub address: RpcPeerAddress,
    pub last_ping_duration: u64, // NOTE: i64 in gRPC protowire

    pub is_outbound: bool,
    pub time_offset: i64,
    pub user_agent: String,

    pub advertised_protocol_version: u32,
    pub time_connected: u64, // NOTE: i64 in gRPC protowire
    pub is_ibd_peer: bool,

    /// Blocks this peer was the FIRST to relay to us, since the node started.
    ///
    /// Gossip is a race: a block reaches several of our peers, and whichever one
    /// wins the request for its body gets the credit here. So this measures who
    /// keeps us supplied with blocks first — not who produced them. On a shielded
    /// chain the producer is deliberately unidentifiable.
    pub blocks_relayed: u64,
}
