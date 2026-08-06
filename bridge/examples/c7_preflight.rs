//! c.7 preflight: does the zKAS node answer GetCurrentBlockColor?
//!
//! Gates hook E's design (Z = blue-confirmed). Run from repo root:
//!
//!   cargo run -p kaspa-stratum-bridge --example c7_preflight -- <zkas_block_hash>
//!   cargo run -p kaspa-stratum-bridge --example c7_preflight -- <hash> grpc://127.0.0.1:16810
//!
//! Use any recent zKAS block hash (node log "accepted block" lines, or
//! explorer.zkas.info). Outcomes:
//!   BLUE / RED        -> RPC supported: wire hook E exactly as written.
//!   RPC error mentioning unsupported/unimplemented method
//!                     -> v1.0.5 predates the op: STOP, use get_block fallback.
//!   connect failure   -> node down or wrong port; nothing proven either way.

use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{RpcHash, api::rpc::RpcApi};
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let hash_str = match args.next() {
        Some(h) => h,
        None => {
            eprintln!("usage: c7_preflight <zkas_block_hash> [grpc_url]");
            std::process::exit(2);
        }
    };
    let url = args.next().unwrap_or_else(|| "grpc://127.0.0.1:16810".to_string());

    let hash = match RpcHash::from_str(hash_str.trim()) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("PREFLIGHT: bad hash '{hash_str}': {e}");
            std::process::exit(2);
        }
    };

    println!("PREFLIGHT: connecting {url} ...");
    let client = match GrpcClient::connect(url.clone()).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("PREFLIGHT: CONNECT FAILED ({e}) — node down or wrong port; RPC support unproven.");
            std::process::exit(3);
        }
    };

    println!("PREFLIGHT: calling get_current_block_color({hash}) ...");
    match client.get_current_block_color(hash).await {
        Ok(resp) => {
            let color = if resp.blue { "BLUE" } else { "RED" };
            println!("PREFLIGHT: OK — node answered: block is {color}.");
            println!("PREFLIGHT: GetCurrentBlockColor SUPPORTED. Wire hook E as written.");
        }
        Err(e) => {
            eprintln!("PREFLIGHT: RPC ERROR: {e}");
            eprintln!("PREFLIGHT: if this reads as unimplemented/unknown-method, v1.0.5");
            eprintln!("PREFLIGHT: lacks the op — do NOT wire hook E; use get_block fallback.");
            std::process::exit(1);
        }
    }
}
