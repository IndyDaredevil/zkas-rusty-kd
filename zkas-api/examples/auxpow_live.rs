//! Read-only: do live ZKas blocks carry AuxPoW, and if so, who mined them on Kaspa?
//!
//! Merged mining is active from genesis (dual acceptance: PoW satisfied natively OR
//! via an AuxPoW witness). A block carrying AuxPoW embeds the parent Kaspa coinbase,
//! which is transparent and names the Kaspa miner in the clear; the pool's
//! stratum-bridge mines natively and carries none. So AuxPoW on a block both proves
//! it was merge-mined and reveals whose Kaspa work it rode — without reaching the
//! miner's node, which is what we cannot do for a firewalled peer.
//!
//!   cargo run --release -p zkas-api --example auxpow_live -- <ip:port> <secs>

use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::block::Block;
use kaspa_p2p_lib::{
    Adaptor, ConnectionInitializer, KaspadMessagePayloadType, Router,
    common::ProtocolError,
    convert::header::{HeaderFormat, Versioned},
    dequeue_with_timeout, make_message,
    pb::{
        Hash as PbHash, KaspadMessage, PongMessage, ReadyMessage, RequestRelayBlocksMessage, VerackMessage, VersionMessage,
        kaspad_message::Payload,
    },
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{Sender, channel};

fn kaspa_address(s: &[u8]) -> Option<Address> {
    match s {
        [0x20, r @ ..] if r.len() == 33 && r[32] == 0xac => Some(Address::new(Prefix::Mainnet, Version::PubKey, &r[..32])),
        [0x21, r @ ..] if r.len() == 34 && r[33] == 0xab => Some(Address::new(Prefix::Mainnet, Version::PubKeyECDSA, &r[..33])),
        [0xaa, 0x20, r @ ..] if r.len() == 33 && r[32] == 0x87 => Some(Address::new(Prefix::Mainnet, Version::ScriptHash, &r[..32])),
        _ => None,
    }
}

struct Listen {
    tx: Sender<KaspadMessage>,
}
#[tonic::async_trait]
impl ConnectionInitializer for Listen {
    async fn initialize_connection(&self, router: Arc<Router>) -> Result<(), ProtocolError> {
        let mut vr = router.subscribe(vec![KaspadMessagePayloadType::Version]);
        let mut ar = router.subscribe(vec![KaspadMessagePayloadType::Verack]);
        let mut rr = router.subscribe(vec![KaspadMessagePayloadType::Ready]);
        // Subscribe to the live stream BEFORE returning, or the routes are missed.
        let mut live = router.subscribe(vec![
            KaspadMessagePayloadType::InvRelayBlock,
            KaspadMessagePayloadType::Block,
            KaspadMessagePayloadType::Ping,
        ]);
        router.start();
        let peer: VersionMessage = dequeue_with_timeout!(vr, Payload::Version, Duration::from_secs(8))?;
        router.enqueue(make_message!(Payload::Verack, VerackMessage {})).await?;
        router
            .enqueue(make_message!(
                Payload::Version,
                VersionMessage {
                    protocol_version: peer.protocol_version,
                    services: peer.services,
                    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
                    address: None,
                    id: Vec::from(uuid::Uuid::new_v4().as_bytes()),
                    user_agent: "/zkas-auxpow-probe/".into(),
                    disable_relay_tx: false,
                    subnetwork_id: None,
                    network: peer.network.clone(),
                }
            ))
            .await?;
        let _: VerackMessage = dequeue_with_timeout!(ar, Payload::Verack, Duration::from_secs(8))?;
        router.enqueue(make_message!(Payload::Ready, ReadyMessage {})).await?;
        let _: ReadyMessage = dequeue_with_timeout!(rr, Payload::Ready, Duration::from_secs(8))?;
        let tx = self.tx.clone();
        let r2 = router.clone();
        tokio::spawn(async move {
            while let Some(m) = live.recv().await {
                if let Some(Payload::InvRelayBlock(inv)) = &m.payload {
                    if let Some(h) = &inv.hash {
                        let _ = r2
                            .enqueue(make_message!(
                                Payload::RequestRelayBlocks,
                                RequestRelayBlocksMessage { hashes: vec![PbHash { bytes: h.bytes.clone() }] }
                            ))
                            .await;
                    }
                } else if let Some(Payload::Ping(p)) = &m.payload {
                    let _ = r2.enqueue(make_message!(Payload::Pong, PongMessage { nonce: p.nonce })).await;
                }
                let _ = tx.send(m).await;
            }
        });
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let node = std::env::args().nth(1).unwrap_or("127.0.0.1:16111".into());
    let secs: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let (tx, mut rx) = channel::<KaspadMessage>(512);
    let a = Adaptor::client_only(Default::default(), Arc::new(Listen { tx }), Default::default());
    if let Err(e) = a.connect_peer(node.clone()).await {
        eprintln!("connect failed: {e}");
        return;
    }
    println!("connected {node}, watching relayed blocks {secs}s\n");
    let (mut seen, mut aux_n) = (0usize, 0usize);
    let mut by_kaspa: HashMap<String, usize> = HashMap::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            m = rx.recv() => {
                let Some(KaspadMessage { payload: Some(Payload::Block(b)), .. }) = m else { continue };
                let block: Block = match Versioned(HeaderFormat::Compressed, b).try_into() { Ok(x) => x, Err(_) => continue };
                seen += 1;
                let Some(aux) = block.header.aux_pow.as_ref() else { continue };
                aux_n += 1;
                let addr = aux.parent_coinbase.outputs.iter()
                    .find_map(|o| kaspa_address(o.script_public_key.script()).map(|a| a.to_string()))
                    .unwrap_or_else(|| "<unparsed>".into());
                *by_kaspa.entry(addr.clone()).or_default() += 1;
                println!("block {} blue {:<8} AUXPOW -> {}", &block.header.hash.to_string()[..12], block.header.blue_score, addr);
            }
        }
    }
    a.close().await;
    println!("\n=== {seen} relayed blocks in {secs}s: {aux_n} AuxPoW, {} native ===", seen - aux_n);
    if by_kaspa.is_empty() {
        println!("No AuxPoW witness seen — these blocks are natively mined (kHeavyHash),");
        println!("so there is no embedded Kaspa coinbase to read.");
    } else {
        let mut v: Vec<_> = by_kaspa.into_iter().collect();
        v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        println!("Kaspa payout addresses in AuxPoW witnesses:");
        for (a, n) in v {
            println!("  {n:>3}  {a}");
        }
    }
}
