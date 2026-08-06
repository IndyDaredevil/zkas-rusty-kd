//! Kaspa first-arrival timing observer — who mines to the target IP?
//!
//! Merge miners run a Kaspa node to pull templates, mine kHeavyHash, and submit
//! solved blocks to Kaspa (transparent coinbase) AND ZKas. We cannot read a ZKas
//! miner (shielded) and cannot reach a firewalled node — but Kaspa's coinbase names
//! the miner in the clear, and a node relays its OWN freshly-mined blocks first.
//!
//! So: peer directly with the target's Kaspa node(s) and with a set of unrelated
//! control peers, and for every block record who announced it first. If one Kaspa
//! miner address is announced-first by the target far above the network baseline,
//! that address is mining behind the target IP.
//!
//! This is a standalone OBSERVER. It touches no production node and changes nothing
//! about consensus — it just connects as an ordinary Kaspa peer and listens.
//!
//!   kaspa_timing <out.jsonl> <secs> <target1,target2,...> <control1,control2,...>
//!
//! Each output line: {"mono_ns","peer","target":bool,"hash"}. Enrich + analyse
//! offline (see kaspa_timing_analyze.py).

use kaspa_p2p_lib::{
    Adaptor, ConnectionInitializer, KaspadMessagePayloadType, Router,
    common::ProtocolError,
    dequeue_with_timeout, make_message,
    pb::{
        AddressesMessage, KaspadMessage, PingMessage, PongMessage, ReadyMessage, RequestAddressesMessage, VerackMessage,
        VersionMessage, kaspad_message::Payload,
    },
};
use std::{
    collections::HashSet,
    io::Write,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{Sender, channel};

/// One announcement: which peer told us about which block, and when (monotonic).
struct Inv {
    mono_ns: u128,
    peer: String,
    hash: String,
}

struct Observer {
    tx: Sender<Inv>,
    start: Instant,
}

#[tonic::async_trait]
impl ConnectionInitializer for Observer {
    async fn initialize_connection(&self, router: Arc<Router>) -> Result<(), ProtocolError> {
        let mut vr = router.subscribe(vec![KaspadMessagePayloadType::Version]);
        let mut ar = router.subscribe(vec![KaspadMessagePayloadType::Verack]);
        let mut rr = router.subscribe(vec![KaspadMessagePayloadType::Ready]);
        // A real Kaspa node drops a peer that ignores the address-exchange and
        // keepalive flows, so we subscribe to and answer them — that is what keeps
        // us connected long enough to actually receive block relays.
        let mut live = router.subscribe(vec![
            KaspadMessagePayloadType::InvRelayBlock,
            KaspadMessagePayloadType::Ping,
            KaspadMessagePayloadType::RequestAddresses,
            KaspadMessagePayloadType::Addresses,
            KaspadMessagePayloadType::Pong,
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
                    user_agent: "/kaspa-timing-observer/".into(),
                    disable_relay_tx: true,
                    subnetwork_id: None,
                    network: peer.network.clone(),
                }
            ))
            .await?;
        let _: VerackMessage = dequeue_with_timeout!(ar, Payload::Verack, Duration::from_secs(8))?;
        router.enqueue(make_message!(Payload::Ready, ReadyMessage {})).await?;
        let _: ReadyMessage = dequeue_with_timeout!(rr, Payload::Ready, Duration::from_secs(8))?;

        // Prompt the address exchange the peer expects from a live node.
        router
            .enqueue(make_message!(
                Payload::RequestAddresses,
                RequestAddressesMessage { include_all_subnetworks: false, subnetwork_id: None }
            ))
            .await?;

        // Tag every announcement with the peer it came from (this router's address).
        let addr = router.net_address().to_string();
        let tx = self.tx.clone();
        let start = self.start;
        let r2 = router.clone();
        // Keepalive: our own ping every 30s so the peer sees us as live.
        let r3 = router.clone();
        tokio::spawn(async move {
            let mut nonce = 1u64;
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                if r3.enqueue(make_message!(Payload::Ping, PingMessage { nonce })).await.is_err() {
                    break;
                }
                nonce += 1;
            }
        });
        tokio::spawn(async move {
            while let Some(m) = live.recv().await {
                match m.payload {
                    Some(Payload::InvRelayBlock(inv)) => {
                        if let Some(h) = &inv.hash {
                            let hash = h.bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                            let _ = tx.send(Inv { mono_ns: start.elapsed().as_nanos(), peer: addr.clone(), hash }).await;
                        }
                    }
                    // Answer the peer's keepalive so it does not drop us.
                    Some(Payload::Ping(p)) => {
                        let _ = r2.enqueue(make_message!(Payload::Pong, PongMessage { nonce: p.nonce })).await;
                    }
                    // Answer address requests with an empty list — enough to be polite.
                    Some(Payload::RequestAddresses(_)) => {
                        let _ = r2.enqueue(make_message!(Payload::Addresses, AddressesMessage { address_list: vec![] })).await;
                    }
                    _ => {}
                }
            }
        });
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "/tmp/kaspa_timing.jsonl".into());
    let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1800);
    let targets: Vec<String> = args.get(3).map(|s| s.split(',').map(String::from).collect()).unwrap_or_default();
    let controls: Vec<String> = args.get(4).map(|s| s.split(',').map(String::from).collect()).unwrap_or_default();
    let target_set: HashSet<String> = targets.iter().cloned().collect();

    let start = Instant::now();
    let (tx, mut rx) = channel::<Inv>(4096);
    let adaptor = Adaptor::client_only(Default::default(), Arc::new(Observer { tx, start }), Default::default());

    let all_addrs: Vec<String> = targets.iter().chain(controls.iter()).cloned().collect();
    let mut connected = 0usize;
    for addr in &all_addrs {
        match adaptor.connect_peer(addr.clone()).await {
            Ok(_) => {
                connected += 1;
                eprintln!("connected {addr}{}", if target_set.contains(addr) { "  [TARGET]" } else { "" });
            }
            Err(e) => eprintln!("skip {addr}: {e}"),
        }
    }
    eprintln!("{connected} peers connected; observing {secs}s -> {out}");
    if connected == 0 {
        return;
    }

    // Reconnection: peers drop over a long run (evictions, restarts, blips) and a
    // dead observer logs nothing. Every 60s re-dial every configured address —
    // a peer we are still connected to rejects the duplicate harmlessly, while a
    // dropped one gets re-established. This is what lets a multi-hour run survive.
    {
        let adaptor = adaptor.clone();
        let addrs = all_addrs.clone();
        let targets = target_set.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                for addr in &addrs {
                    if let Ok(_) = adaptor.connect_peer(addr.clone()).await {
                        eprintln!("re-connected {addr}{}", if targets.contains(addr) { "  [TARGET]" } else { "" });
                    }
                }
            }
        });
    }

    let mut file = std::io::BufWriter::new(std::fs::File::create(&out).expect("create out"));
    let wall0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut n = 0u64;
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline.into()) => break,
            inv = rx.recv() => {
                let Some(inv) = inv else { break };
                let target = target_set.contains(&inv.peer)
                    // Peers dialed by ip:port may report a canonicalized address; match on ip too.
                    || target_set.iter().any(|t| t.rsplit_once(':').map(|(ip,_)| inv.peer.starts_with(ip)).unwrap_or(false));
                let _ = writeln!(file, "{{\"wall_ns\":{},\"mono_ns\":{},\"peer\":\"{}\",\"target\":{},\"hash\":\"{}\"}}",
                    wall0 + inv.mono_ns, inv.mono_ns, inv.peer, target, inv.hash);
                n += 1;
                // Flush often so a long run is observable on disk mid-flight.
                if n % 100 == 0 { let _ = file.flush(); }
                if n % 1000 == 0 { eprintln!("  {n} invs logged"); }
            }
        }
    }
    let _ = file.flush();
    adaptor.close().await;
    eprintln!("done: {n} invs from {connected} peers over {secs}s -> {out}");
}
