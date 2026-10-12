//! Ask a node how far back it can actually serve shielded note history.
//!
//! `GetShieldedTreeState` is the call a wallet already makes to anchor a scan, and it now
//! carries the node's history floor. This probe exists because that floor is the difference
//! between a correct balance and a silently partial one, and "the compiler accepted the struct
//! field" is not evidence that a real node reports it correctly over the wire.
//!
//! Usage: zkas-history-status [host:port]   (default 127.0.0.1:16110)
use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::api::rpc::RpcApi;
use kaspa_rpc_core::notify::mode::NotificationMode;

#[tokio::main]
async fn main() {
    let addr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:16110".to_string());
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

    let r = client.get_shielded_tree_state(None).await.unwrap_or_else(|e| {
        eprintln!("getShieldedTreeState failed: {e}");
        std::process::exit(1);
    });

    println!("checkpoint block      : {}", r.block_hash);
    println!("checkpoint daa_score  : {}", r.daa_score);
    println!("frontier size (leaves): {}", r.size);
    println!("history_from_daa_score: {}", r.history_from_daa_score);
    println!("history_complete      : {}", r.history_complete);
    if r.history_complete {
        println!("\nVERDICT: this node can serve shielded history from genesis.");
    } else {
        println!(
            "\nVERDICT: PARTIAL — this node cannot answer for any wallet whose birthday is below \
             DAA {}. A balance it reports for such a wallet is not final.",
            r.history_from_daa_score
        );
    }
}
