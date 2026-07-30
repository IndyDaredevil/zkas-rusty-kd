use kaspa_p2p_lib::{
    Adaptor, ConnectionInitializer, KaspadMessagePayloadType, Router, common::ProtocolError,
    dequeue_with_timeout, make_message,
    pb::{KaspadMessage, PingMessage, PongMessage, ReadyMessage, VerackMessage, VersionMessage, kaspad_message::Payload},
};
use std::{sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};
use tokio::sync::mpsc::{Sender, channel};

struct Dump { tx: Sender<KaspadMessage> }
#[tonic::async_trait]
impl ConnectionInitializer for Dump {
    async fn initialize_connection(&self, router: Arc<Router>) -> Result<(), ProtocolError> {
        let mut vr = router.subscribe(vec![KaspadMessagePayloadType::Version]);
        let mut ar = router.subscribe(vec![KaspadMessagePayloadType::Verack]);
        let mut rr = router.subscribe(vec![KaspadMessagePayloadType::Ready]);
        // Subscribe to everything else BEFORE returning.
        use KaspadMessagePayloadType::*;
        let mut all = router.subscribe(vec![
            InvRelayBlock, Block, InvTransactions, Transaction, Ping, Addresses, RequestAddresses,
            BlockHeaders, IbdBlock, RequestRelayBlocks,
        ]);
        router.start();
        let peer: VersionMessage = dequeue_with_timeout!(vr, Payload::Version, Duration::from_secs(8))?;
        router.enqueue(make_message!(Payload::Verack, VerackMessage {})).await?;
        router.enqueue(make_message!(Payload::Version, VersionMessage{
            protocol_version: peer.protocol_version, services: peer.services,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
            address: None, id: Vec::from(uuid::Uuid::new_v4().as_bytes()),
            user_agent: "/dump/".into(), disable_relay_tx: false, subnetwork_id: None, network: peer.network.clone(),
        })).await?;
        let _: VerackMessage = dequeue_with_timeout!(ar, Payload::Verack, Duration::from_secs(8))?;
        router.enqueue(make_message!(Payload::Ready, ReadyMessage {})).await?;
        let _: ReadyMessage = dequeue_with_timeout!(rr, Payload::Ready, Duration::from_secs(8))?;
        let tx = self.tx.clone();
        let r2 = router.clone();
        tokio::spawn(async move {
            while let Some(m) = all.recv().await {
                // answer pings to stay alive
                if let Some(Payload::Ping(p)) = &m.payload {
                    let _ = r2.enqueue(make_message!(Payload::Pong, PongMessage{ nonce: p.nonce })).await;
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
    let (tx, mut rx) = channel::<KaspadMessage>(256);
    let a = Adaptor::client_only(Default::default(), Arc::new(Dump{tx}), Default::default());
    a.connect_peer(node.clone()).await.unwrap();
    println!("connected {node}, dumping 15s");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut counts = std::collections::HashMap::<String,usize>::new();
    loop {
        tokio::select!{
            _ = tokio::time::sleep_until(deadline) => break,
            m = rx.recv() => { let Some(m)=m else {break}; let name = format!("{:?}", m.payload.as_ref().map(std::mem::discriminant)); let key = match &m.payload { Some(p)=>format!("{p:?}").split('(').next().unwrap().to_string(), None=>"none".into() }; *counts.entry(key).or_default()+=1; let _=name; }
        }
    }
    a.close().await;
    let mut v: Vec<_> = counts.into_iter().collect(); v.sort_by_key(|(_,n)| std::cmp::Reverse(*n));
    for (k,n) in v { println!("  {n:>4}  {k}"); }
}
