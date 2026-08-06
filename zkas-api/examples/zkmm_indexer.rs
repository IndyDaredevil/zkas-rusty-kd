//! ZKMM indexer — the Kaspa side of the merge-mining attribution pipeline.
//!
//! Follows a Kaspa node forward, and for every block whose coinbase carries a
//! `ZKMM<zkas_block_hash>` tag, records `zkas_block_hash -> kaspa_payout_address`.
//! The payout is parsed from the coinbase PAYLOAD script (the authoritative miner
//! field), NOT the first output (which pays mergeset blues — the classic trap).
//!
//! Output: appends `H \t kaspa_addr \t unix_secs` to a TSV, kept to a bounded tail.
//! The join job pairs these against our ZKas node's per-block first-relayer log to
//! map miner IP -> Kaspa address. Read-only; touches nothing.
//!
//!   zkmm_indexer <kaspa grpc host:port> <out.tsv>

use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{RpcHash, api::rpc::RpcApi, notify::mode::NotificationMode};
use std::{
    collections::VecDeque,
    io::Write,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// Kaspa cashaddr-style encoding (version 0 = schnorr P2PK, 1 = ecdsa).
const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
fn polymod(values: &[u8]) -> u64 {
    let mut c: u64 = 1;
    for &d in values {
        let c0 = (c >> 35) as u8;
        c = ((c & 0x07ffffffff) << 5) ^ d as u64;
        if c0 & 0x01 != 0 {
            c ^= 0x98f2bc8e61
        }
        if c0 & 0x02 != 0 {
            c ^= 0x79b76d99e2
        }
        if c0 & 0x04 != 0 {
            c ^= 0xf33e5fb3c4
        }
        if c0 & 0x08 != 0 {
            c ^= 0xae2eabe2a8
        }
        if c0 & 0x10 != 0 {
            c ^= 0x1e4f43e470
        }
    }
    c ^ 1
}
fn encode_addr(version: u8, pubkey: &[u8]) -> String {
    let mut data = vec![version];
    data.extend_from_slice(pubkey);
    // 8-bit -> 5-bit
    let (mut acc, mut bits) = (0u32, 0u32);
    let mut five = Vec::new();
    for &b in &data {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            five.push(((acc >> bits) & 0x1f) as u8);
        }
    }
    if bits > 0 {
        five.push(((acc << (5 - bits)) & 0x1f) as u8);
    }
    let prefix = b"kaspa";
    let mut chk_input: Vec<u8> = prefix.iter().map(|c| c & 0x1f).collect();
    chk_input.push(0);
    chk_input.extend_from_slice(&five);
    chk_input.extend_from_slice(&[0u8; 8]);
    let chk = polymod(&chk_input);
    let cks: Vec<u8> = (0..8).map(|i| ((chk >> (5 * (7 - i))) & 0x1f) as u8).collect();
    let body: String = five.iter().chain(cks.iter()).map(|&d| CHARSET[d as usize] as char).collect();
    format!("kaspa:{body}")
}

/// Miner Kaspa address from a coinbase payload, or None.
/// Payload: blue_score(8) subsidy(8) script_ver(2) script_len(1) script extra.
fn payload_address(payload: &[u8]) -> Option<String> {
    if payload.len() < 19 {
        return None;
    }
    let slen = payload[18] as usize;
    let script = payload.get(19..19 + slen)?;
    match script {
        [0x20, rest @ .., 0xac] if rest.len() == 32 => Some(encode_addr(0, rest)),
        [0x21, rest @ .., 0xab] if rest.len() == 33 => Some(encode_addr(1, rest)),
        _ => None,
    }
}

/// The ZKas block hash tagged in a coinbase payload, if any (`ZKMM<64 hex>`).
fn zkmm_hash(payload: &[u8]) -> Option<String> {
    // extra_data lives after the script; scan the whole tail for the marker.
    let s = String::from_utf8_lossy(payload);
    let idx = s.find("ZKMM")?;
    let hex: String = s[idx + 4..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() >= 64).then(|| hex[..64].to_string())
}

#[tokio::main]
async fn main() {
    let rpc = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:16215".into());
    let out = std::env::args().nth(2).unwrap_or_else(|| "/root/zkmm.tsv".into());

    let client = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{rpc}"),
        None,
        false,
        None,
        false,
        None,
        Default::default(),
    )
    .await
    .expect("connect kaspa grpc");

    // NB: get_block_dag_info parses the network name, which this ZKas-fork client
    // rejects for "kaspa-mainnet". get_sink returns just the sink hash — no network
    // field — so it works against a real Kaspa node.
    let mut low = client.get_sink().await.expect("get_sink").sink;
    eprintln!("zkmm_indexer: following {rpc} from sink {}", &low.to_string()[..12]);

    // Ring of hashes already emitted, so a restart / overlap doesn't double-write.
    let mut seen: VecDeque<String> = VecDeque::new();
    let mut seen_set = std::collections::HashSet::new();

    loop {
        let resp = match client.get_blocks(Some(low), true, true).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("get_blocks: {e}");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let mut hits = 0;
        let mut rows = String::new();
        for block in &resp.blocks {
            let Some(cb) = block.transactions.first() else { continue };
            let payload = &cb.payload;
            let (Some(h), Some(addr)) = (zkmm_hash(payload), payload_address(payload)) else { continue };
            if seen_set.contains(&h) {
                continue;
            }
            seen_set.insert(h.clone());
            seen.push_back(h.clone());
            if seen.len() > 300_000 {
                if let Some(old) = seen.pop_front() {
                    seen_set.remove(&old);
                }
            }
            let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            rows.push_str(&format!("{h}\t{addr}\t{t}\n"));
            hits += 1;
        }
        if !rows.is_empty() {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&out) {
                let _ = f.write_all(rows.as_bytes());
            }
        }
        // Advance the cursor to the last block we saw.
        if let Some(last) = resp.block_hashes.last() {
            if *last != RpcHash::default() {
                low = *last;
            }
        }
        if hits > 0 {
            eprintln!("zkmm_indexer: +{hits} ZKMM blocks (cursor {})", &low.to_string()[..12]);
        }
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
}
