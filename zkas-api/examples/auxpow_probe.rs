//! Research probe: who is merge-mining ZKas into Kaspa?
//!
//! Every merge-mined ZKas block carries `Header.aux_pow`, and inside it the
//! PARENT (Kaspa) coinbase transaction. Kaspa's coinbase is transparent, so that
//! transaction names its own payout address in the clear — which means a
//! merge-mined ZKas block identifies its miner on the Kaspa side without any
//! inference at all.
//!
//! This prints, per recent block: whether it is merge-mined, the Kaspa payout
//! address(es) from the parent coinbase, and the ZKas-side coinbase payout
//! address, so the two identities can be linked.
//!
//! Read-only. Run against any ZKas node's gRPC.

use kaspa_addresses::{Address, Prefix, Version};
use kaspa_consensus_core::auxpow::AuxPow;
use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use std::collections::HashMap;

/// Orchard (shielded) recipient script length in a ZKas coinbase output.
const ORCHARD_SCRIPT_LEN: usize = 43;

/// Best-effort transparent Kaspa address from a script_public_key.
/// Kaspa coinbases pay either P2PK (schnorr/ecdsa) or P2SH.
fn kaspa_address(script: &[u8]) -> Option<Address> {
    match script {
        // OP_DATA_32 <32-byte pubkey> OP_CHECKSIG
        [0x20, rest @ ..] if rest.len() == 33 && rest[32] == 0xac => {
            Some(Address::new(Prefix::Mainnet, Version::PubKey, &rest[..32]))
        }
        // OP_DATA_33 <33-byte pubkey> OP_CHECKSIGECDSA
        [0x21, rest @ ..] if rest.len() == 34 && rest[33] == 0xab => {
            Some(Address::new(Prefix::Mainnet, Version::PubKeyECDSA, &rest[..33]))
        }
        // OP_BLAKE2B <32-byte hash> OP_EQUAL
        [0xaa, 0x20, rest @ ..] if rest.len() == 33 && rest[32] == 0x87 => {
            Some(Address::new(Prefix::Mainnet, Version::ScriptHash, &rest[..32]))
        }
        _ => None,
    }
}

#[tokio::main]
async fn main() {
    let rpc = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:16110".into());
    let count: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(40);

    let client = GrpcClient::connect_with_args(NotificationMode::Direct, format!("grpc://{rpc}"), None, false, None, false, None, Default::default())
        .await
        .expect("connect");

    let dag = client.get_block_dag_info().await.expect("dag info");
    println!("node {rpc} — {} tips, daa {}", dag.tip_hashes.len(), dag.virtual_daa_score);

    // Walk back from the tips over the recent chain.
    let sel = client.get_virtual_chain_from_block(dag.pruning_point_hash, false).await;
    let mut hashes: Vec<_> = match sel {
        Ok(v) => v.added_chain_block_hashes.into_iter().rev().take(count).collect(),
        Err(_) => dag.tip_hashes.clone(),
    };
    if hashes.is_empty() {
        hashes = dag.tip_hashes.clone();
    }

    let mut merged = 0usize;
    let mut plain = 0usize;
    // Kaspa payout address -> (blocks, set of ZKas payout addresses)
    let mut by_kaspa: HashMap<String, (usize, Vec<String>)> = HashMap::new();

    for h in hashes.iter().take(count) {
        let Ok(block) = client.get_block(*h, true).await else { continue };

        // ZKas-side payout: first Orchard-scripted coinbase output.
        let zkas_addr = block
            .transactions
            .first()
            .and_then(|cb| {
                cb.outputs.iter().find(|o| o.script_public_key.script().len() == ORCHARD_SCRIPT_LEN)
            })
            .map(|o| {
                Address::new(Prefix::Mainnet, Version::ShieldedOrchard, o.script_public_key.script()).to_string()
            })
            .unwrap_or_else(|| "-".into());

        let hex = &block.header.aux_pow;
        if hex.is_empty() {
            plain += 1;
            println!("{}  blue {:<8} NOT merge-mined              zkas={}", &h.to_string()[..12], block.header.blue_score, &zkas_addr[..28.min(zkas_addr.len())]);
            continue;
        }
        merged += 1;

        let bytes = match (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16)).collect::<Result<Vec<u8>, _>>() {
            Ok(b) => b,
            Err(e) => {
                println!("{}  aux_pow hex undecodable: {e}", &h.to_string()[..12]);
                continue;
            }
        };
        let aux: AuxPow = match borsh::from_slice(&bytes) {
            Ok(a) => a,
            Err(e) => {
                println!("{}  aux_pow borsh undecodable: {e}", &h.to_string()[..12]);
                continue;
            }
        };

        let kaspa_addrs: Vec<String> = aux
            .parent_coinbase
            .outputs
            .iter()
            .filter_map(|o| kaspa_address(o.script_public_key.script()).map(|a| a.to_string()))
            .collect();
        let primary = kaspa_addrs.first().cloned().unwrap_or_else(|| "<unparsed script>".into());

        // Kaspa miners tag their coinbase payload; that string is often the pool name.
        let tag: String =
            String::from_utf8_lossy(&aux.parent_coinbase.payload).chars().filter(|c| c.is_ascii_graphic() || *c == ' ').collect();

        let e = by_kaspa.entry(primary.clone()).or_default();
        e.0 += 1;
        if !e.1.contains(&zkas_addr) {
            e.1.push(zkas_addr.clone());
        }

        println!(
            "{}  blue {:<8} MERGED  kaspa_daa={:<10} kaspa={}  tag={:?}",
            &h.to_string()[..12],
            block.header.blue_score,
            aux.parent_header.daa_score,
            &primary[..38.min(primary.len())],
            tag.chars().rev().take(40).collect::<String>().chars().rev().collect::<String>()
        );
    }

    println!("\n=== summary over {} blocks ===", merged + plain);
    println!("merge-mined into Kaspa: {merged}   not merge-mined: {plain}");
    println!("\ndistinct Kaspa payout addresses (= distinct merged miners):");
    let mut rows: Vec<_> = by_kaspa.into_iter().collect();
    rows.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for (kaddr, (n, zaddrs)) in rows {
        println!("  {n:>3} blocks  {kaddr}");
        for z in zaddrs {
            println!("            zkas payout: {z}");
        }
    }
}
