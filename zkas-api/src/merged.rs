//! Who is mining KAS **and** ZKAS?
//!
//! Merged mining needs Kaspa block templates, so an operator merge-mining the two
//! chains has to run a Kaspa node as well as a ZKas one — in practice on the same
//! host. That leaves a signature we can read directly instead of guessing: open a
//! Kaspa p2p handshake against a ZKas peer's own address and see whether anything
//! answers `kaspa-mainnet`.
//!
//! This is evidence, not inference. A version reply is the peer's own node
//! identifying its network and client. Nothing here is statistical, and nothing
//! about consensus changes.
//!
//! A tell worth knowing: ZKas p2p and Kaspa p2p share the default port 16111, so a
//! dual operator must move one of them. Peers found running ZKas on a *non*-default
//! port and Kaspa on 16111 are exactly the shape merged mining leaves behind.
//!
//! The scan runs on a timer in the background and the result is cached — probing
//! peers on every HTTP request would be slow and rude.

use kaspa_p2p_lib::{
    Adaptor, ConnectionInitializer, KaspadMessagePayloadType, Router,
    common::ProtocolError,
    dequeue_with_timeout, make_message,
    pb::{VerackMessage, VersionMessage, kaspad_message::Payload},
};
use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{Sender, channel};

/// Ports to sweep. 16111 is the Kaspa mainnet default and what a dual operator
/// usually leaves to Kaspa; the rest are the shifts people actually reach for when
/// the two chains collide (our own VPS1 uses 16211). Cheap to widen because every
/// port gets a 1-second TCP knock before any handshake is attempted — only an open
/// port costs a real connection.
pub const KASPA_PORTS: [u16; 10] = [16111, 16211, 16311, 16411, 16511, 16611, 17111, 26111, 36111, 16011];

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(6);
/// Whole-probe ceiling, so one unresponsive host cannot stall a scan.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// TCP pre-check budget. A closed or filtered port fails (or times out) here and
/// never reaches the handshake, which is what makes a ten-port sweep affordable.
const KNOCK_TIMEOUT: Duration = Duration::from_millis(1200);

/// What answered at an address.
///
/// `address` is the Kaspa node's own network address — the closest thing to a
/// "Kaspa address" a handshake can yield. A miner's payout address (`kaspa:q…`)
/// appears only in blocks that miner produced and is NOT carried here; nothing in
/// the p2p handshake reveals it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Found {
    pub port: u16,
    /// `ip:port` of the Kaspa node that answered.
    pub address: String,
    /// The node's advertised p2p id (its own identifier, stable across the session).
    #[serde(rename = "nodeId")]
    pub node_id: String,
    /// What the node says its own reachable address is, when it offers one. Nodes
    /// frequently advertise a port they do not actually serve, so this is reported
    /// separately from `address` rather than trusted over it.
    #[serde(rename = "advertised")]
    pub advertised: Option<String>,
    pub network: String,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
}

/// Minimal initializer: complete the handshake, hand back the peer's version, stop.
/// We mirror the peer's own `network` string so the handshake cannot be rejected on
/// a network mismatch — we are asking what it is, not claiming to be its kin.
struct Probe {
    tx: Sender<VersionMessage>,
}

#[tonic::async_trait]
impl ConnectionInitializer for Probe {
    async fn initialize_connection(&self, router: Arc<Router>) -> Result<(), ProtocolError> {
        let mut version_route = router.subscribe(vec![KaspadMessagePayloadType::Version]);
        let mut verack_route = router.subscribe(vec![KaspadMessagePayloadType::Verack]);
        router.start();

        let peer: VersionMessage = dequeue_with_timeout!(version_route, Payload::Version, HANDSHAKE_TIMEOUT)?;
        router.enqueue(make_message!(Payload::Verack, VerackMessage {})).await?;
        router.enqueue(make_message!(Payload::Version, mirror(&peer))).await?;
        let _: VerackMessage = dequeue_with_timeout!(verack_route, Payload::Verack, HANDSHAKE_TIMEOUT)?;

        let _ = self.tx.send(peer).await;
        // We have what we came for; `Ready` is never sent, so the peer drops us.
        Ok(())
    }
}

fn mirror(peer: &VersionMessage) -> VersionMessage {
    VersionMessage {
        protocol_version: peer.protocol_version,
        services: peer.services,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64,
        address: None,
        id: Vec::from(uuid::Uuid::new_v4().as_bytes()),
        user_agent: "/zkas-explorer-probe/".to_string(),
        disable_relay_tx: true,
        subnetwork_id: None,
        network: peer.network.clone(),
    }
}

/// `host:port` in the form the p2p client and a reader both want.
fn socket(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(v4) => format!("{v4}:{port}"),
        IpAddr::V6(v6) => format!("[{v6}]:{port}"),
    }
}

/// Is anything listening? A plain TCP connect, so a closed port costs ~nothing and
/// a filtered one costs `KNOCK_TIMEOUT` instead of a full handshake budget.
async fn tcp_open(ip: IpAddr, port: u16) -> bool {
    tokio::time::timeout(KNOCK_TIMEOUT, tokio::net::TcpStream::connect((ip, port))).await.is_ok_and(|r| r.is_ok())
}

/// Handshake `ip:port` and report what answered, or `None` if nothing did.
///
/// Callers that sweep many ports should `tcp_open` first (see `probe_kaspa_all`)
/// so a closed port never reaches the heavier handshake path.
pub async fn probe(ip: IpAddr, port: u16) -> Option<Found> {
    let (tx, mut rx) = channel::<VersionMessage>(1);
    let adaptor = Adaptor::client_only(Default::default(), Arc::new(Probe { tx }), Default::default());
    let addr = socket(ip, port);

    let result = tokio::time::timeout(PROBE_TIMEOUT, async {
        // A port can be open and still not be a kaspad — that is a clean "no",
        // not an error worth surfacing.
        let _ = adaptor.connect_peer(addr.clone()).await;
        rx.recv().await
    })
    .await
    .ok()
    .flatten();

    adaptor.close().await;

    result.map(|v| Found {
        port,
        address: addr,
        node_id: v.id.iter().map(|b| format!("{b:02x}")).collect(),
        // A node's self-reported address is routinely wrong (ours advertises a port
        // it does not serve), so keep it beside the address we actually reached.
        advertised: v.address.as_ref().map(|a| {
            let ip: Vec<String> = a.ip.iter().map(|b| format!("{b:02x}")).collect();
            format!("{}:{}", ip.join(""), a.port)
        }),
        network: v.network.clone(),
        user_agent: v.user_agent.clone(),
        protocol_version: v.protocol_version,
    })
}

/// Outcome of sweeping one host.
pub struct Sweep {
    /// Every Kaspa node that answered a handshake.
    pub found: Vec<Found>,
    /// Did ANY candidate port even accept a TCP connection? When false the host is
    /// unreachable from here — firewalled or inbound-only (it dialed us; we cannot
    /// dial back). That is not "runs no Kaspa"; it is "cannot be probed at all", and
    /// a firewalled host that is also a heavy block relayer is the classic shape of
    /// a miner that does not want to be found.
    pub reachable: bool,
}

/// Sweep every candidate port and return **all** Kaspa nodes found on the host.
///
/// All, not the first: an operator can run several kaspads (mainnet plus a testnet,
/// or a spare), and stopping at the first hit would hide that. A peer answering
/// `zkas-mainnet` is just its own ZKas node and is never evidence of anything, so
/// only `kaspa*` networks are kept.
pub async fn probe_kaspa_all(ip: IpAddr, skip_port: u16) -> Sweep {
    let mut found = Vec::new();
    let mut reachable = false;
    for port in KASPA_PORTS {
        // No point knocking on the port we already know serves ZKas.
        if port == skip_port {
            continue;
        }
        if !tcp_open(ip, port).await {
            continue;
        }
        reachable = true; // something is listening, even if it is not a kaspad
        if let Some(hit) = probe(ip, port).await {
            if hit.network.starts_with("kaspa") {
                found.push(hit);
            }
        }
    }
    Sweep { found, reachable }
}
