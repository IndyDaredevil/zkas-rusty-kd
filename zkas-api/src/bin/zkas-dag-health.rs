//! Measure how wide the live DAG actually is, and why.
//!
//! The chain-level numbers said something was wrong (virtualDaaScore == blockCount with a
//! selected chain half that size ⇒ average mergeset ~2 at 1 BPS), but they cannot say
//! whether the siblings are network latency or stale mining templates. This walks the
//! recent selected chain and, for every merged sibling, reports how far its timestamp sits
//! from its merging chain block's — stale-template siblings cluster at 0-3 s, and genuine
//! propagation races sit under ~1 s.
//!
//! Usage: zkas-dag-health [host:port] [chain_blocks_to_walk]   (default 127.0.0.1:16110, 300)

use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::api::rpc::RpcApi;
use kaspa_rpc_core::notify::mode::NotificationMode;

#[tokio::main]
async fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:16110".to_string());
    let walk: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(300);
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
    .unwrap_or_else(|e| {
        eprintln!("failed to connect to {addr}: {e}");
        std::process::exit(1);
    });

    let info = client.get_block_dag_info().await.expect("getBlockDagInfo");
    println!("network            : {}", info.network);
    println!("blocks (DAG)       : {}", info.block_count);
    println!("virtual DAA score  : {}", info.virtual_daa_score);
    let mut cursor = info.sink;

    // Histogram of mergeset sizes (1 = no sibling merged) and sibling timestamp offsets.
    let mut mergeset_hist = std::collections::BTreeMap::<usize, u64>::new();
    let mut offsets_ms = Vec::<i64>::new();
    let mut by_payee = std::collections::BTreeMap::<String, (u64, Vec<i64>)>::new();
    let mut walked = 0usize;
    while walked < walk {
        let b = match client.get_block(cursor, false).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("getBlock({cursor}) failed after {walked} blocks: {e}");
                break;
            }
        };
        let Some(v) = b.verbose_data.as_ref() else {
            eprintln!("node returned no verbose data (needs --utxoindex? older build?)");
            break;
        };
        let merged: Vec<_> =
            v.merge_set_blues_hashes.iter().chain(v.merge_set_reds_hashes.iter()).filter(|h| **h != v.selected_parent_hash).cloned().collect();
        *mergeset_hist.entry(1 + merged.len()).or_default() += 1;
        for m in merged {
            if let Ok(mb) = client.get_block(m, true).await {
                let off = b.header.timestamp as i64 - mb.header.timestamp as i64;
                offsets_ms.push(off);
                // Attribute the sibling to its producer via the coinbase payout script
                // prefix - enough to tell the pools apart without deanonymising anyone
                // beyond what the chain already publishes.
                let payee = mb
                    .transactions
                    .first()
                    .and_then(|cb| cb.outputs.first())
                    .map(|o| hex::encode(&o.script_public_key.script()[..o.script_public_key.script().len().min(6)]))
                    .unwrap_or_else(|| "?".into());
                let e = by_payee.entry(payee).or_insert((0u64, Vec::new()));
                e.0 += 1;
                e.1.push(off);
            }
        }
        cursor = v.selected_parent_hash;
        walked += 1;
        if cursor == Default::default() {
            break;
        }
    }

    println!("chain blocks walked: {walked}");
    let total: u64 = mergeset_hist.values().sum();
    let weighted: u64 = mergeset_hist.iter().map(|(k, v)| *k as u64 * v).sum();
    println!("avg mergeset       : {:.2}", weighted as f64 / total.max(1) as f64);
    for (k, v) in &mergeset_hist {
        println!("  mergeset {k:>2}: {v:>5}  {}", "#".repeat((*v as usize * 60 / total.max(1) as usize).max(1)));
    }
    if !offsets_ms.is_empty() {
        offsets_ms.sort_unstable();
        let n = offsets_ms.len();
        let med = offsets_ms[n / 2];
        let p90 = offsets_ms[(n * 9 / 10).min(n - 1)];
        let within_1s = offsets_ms.iter().filter(|d| d.abs() <= 1_000).count();
        let within_3s = offsets_ms.iter().filter(|d| d.abs() <= 3_000).count();
        println!("siblings measured  : {n}");
        println!("timestamp offset ms (chain block - sibling): median {med}, p90 {p90}");
        println!("  |offset| <= 1s: {:.0}%   <= 3s: {:.0}%", within_1s as f64 * 100.0 / n as f64, within_3s as f64 * 100.0 / n as f64);
        println!();
        println!("reading: offsets clustered in 1-3s with mergesets ~2 on most blocks = producers");
        println!("build on seconds-stale tips (template refresh / peering), not random 1-BPS races.");
    } else {
        println!("no siblings in the walked range - DAG is narrow here.");
    }
    for (p, (n, mut offs)) in by_payee {
        offs.sort_unstable();
        println!("stale-sibling producer {p}: {n} siblings, median offset {} ms", offs[offs.len() / 2]);
    }
}
