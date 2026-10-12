//! Measure the block→template lag on a live node: the time between the virtual sink
//! changing (a new block was fully processed) and the block template's parent set
//! reflecting it. This is the half of the network's ~3.2 s common-mode staleness the
//! node itself is responsible for — the other half is p2p propagation + miner pickup.
//!
//! Usage: zkas-template-lag <host:port> <pay_address> [seconds_to_watch]

use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::api::rpc::RpcApi;
use kaspa_rpc_core::notify::mode::NotificationMode;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:16110".into());
    let pay = std::env::args().nth(2).expect("pay address required");
    let watch: u64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(120);
    let client = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{addr}"),
        None,
        true,
        None,
        false,
        Some(500_000),
        Default::default(),
    )
    .await
    .expect("connect");

    let pay_addr = kaspa_addresses::Address::try_from(pay.as_str()).expect("bad address");
    let mut last_sink = kaspa_rpc_core::RpcHash::default();
    let mut sink_seen_at: Option<(kaspa_rpc_core::RpcHash, Instant)> = None;
    let mut lags_ms = Vec::<u128>::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(watch) {
        if let Ok(info) = client.get_block_dag_info().await {
            if info.sink != last_sink {
                last_sink = info.sink;
                sink_seen_at = Some((info.sink, Instant::now()));
            }
        }
        if let Some((sink, t0)) = sink_seen_at {
            if let Ok(tpl) = client.get_block_template(pay_addr.clone(), Vec::new()).await {
                if tpl.block.header.parents_by_level.first().is_some_and(|p| p.contains(&sink)) {
                    lags_ms.push(t0.elapsed().as_millis());
                    sink_seen_at = None;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    lags_ms.sort_unstable();
    let n = lags_ms.len();
    if n == 0 {
        println!("no sink→template transitions observed in {watch}s");
        return;
    }
    println!("sink→template lag over {n} blocks: median {} ms, p90 {} ms, max {} ms", lags_ms[n / 2], lags_ms[(n * 9 / 10).min(n - 1)], lags_ms[n - 1]);
    println!("(measured with a 50 ms poll, so subtract up to ~100 ms of probe noise)");
}
