//! Assembles a canonical-`R` seq_commit witness from a **live** ZKas node and checks it against the
//! chain's own committed value.
//!
//! This is the production counterpart of the in-process consensus test: it drives the deployed
//! `GetSeqCommitLaneProof` RPC, rebuilds the witness with the same host assembler the peg-out
//! relayer uses, and asserts the recomputed `seq_commit` equals the one in the queried block's
//! header, which is exactly what the covenant resolves via `OpChainblockSeqCommit`.
//!
//! With `--emit` it also prints the witness fields as `key=value` lines, which
//! `canonical-r-encode` (in the vprogs workspace, where the wire encoder lives) turns into the
//! peg-out `ix_data`. Splitting it this way keeps the encoder single-sourced: this crate pins our
//! fork for the RPC, that one pins the ABI.
//!
//! Usage: `canonical-r-probe [rpc_address] [blocks_back] [--emit]` (defaults `127.0.0.1:16110`, `10`).

use kaspa_grpc_client::GrpcClient;
use kaspa_hashes::Hash;
use kaspa_rpc_core::{api::rpc::RpcApi, notify::mode::NotificationMode};
use kaspa_seq_commit::{hashing::miner_payload_leaf, types::MinerPayloadLeafInput};
use kaspa_shielded_core::witness_chain::SeqCommitWitness;

/// Byte offset of the #24 shielded state root `R` in a ZKas coinbase payload.
const SHIELDED_COMMITMENT_OFFSET: usize = 16;

#[tokio::main]
async fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let emit = argv.iter().any(|a| a == "--emit");
    let mut positional = argv.iter().filter(|a| !a.starts_with("--"));
    let address = positional.next().cloned().unwrap_or_else(|| "127.0.0.1:16110".to_string());
    let blocks_back: u64 = positional.next().and_then(|a| a.parse().ok()).unwrap_or(10);

    let client = GrpcClient::connect_with_args(
        NotificationMode::Direct,
        format!("grpc://{address}"),
        None,
        true,
        None,
        false,
        Some(500_000),
        Default::default(),
    )
    .await
    .unwrap_or_else(|e| fatal(&format!("connect {address}: {e}")));

    // Walk back from the sink so the queried block is buried (its SMT state is still readable and
    // the lane proof's canonicality/pruning checks pass).
    let info = client.get_block_dag_info().await.unwrap_or_else(|e| fatal(&format!("dag info: {e}")));
    let mut queried = info.sink;
    for _ in 0..blocks_back {
        let b = client.get_block(queried, false).await.unwrap_or_else(|e| fatal(&format!("get_block: {e}")));
        let sp = b.verbose_data.as_ref().unwrap_or_else(|| fatal("missing verbose data")).selected_parent_hash;
        if sp == Hash::default() {
            break;
        }
        queried = sp;
    }

    // Node-side witness fields for the queried (merging) block.
    let proof = client
        .get_seq_commit_lane_proof(queried, Hash::from_bytes([0u8; 32]))
        .await
        .unwrap_or_else(|e| fatal(&format!("get_seq_commit_lane_proof({queried}): {e}")));
    if proof.miner_payload_leaves.is_empty() {
        fatal(&format!("block {queried} merges no non-selected-parent block; try a different depth"));
    }

    let block = client.get_block(queried, false).await.unwrap_or_else(|e| fatal(&format!("get_block: {e}")));
    let vd = block.verbose_data.as_ref().unwrap_or_else(|| fatal("missing verbose data"));
    let expected_seq_commit = block.header.accepted_id_merkle_root;

    // Locate the mergeset member whose miner-payload leaf the node emitted. Its coinbase is what
    // the seq_commit actually commits, and it carries the #24 `R`.
    let mut member = None;
    for cand in vd.merge_set_blues_hashes.iter().chain(vd.merge_set_reds_hashes.iter()).copied() {
        let cb = client.get_block(cand, true).await.unwrap_or_else(|e| fatal(&format!("get_block({cand}): {e}")));
        let payload = cb.transactions.first().unwrap_or_else(|| fatal("merged block has no coinbase")).payload.clone();
        let blue_work_be = cb.header.blue_work.to_be_bytes().to_vec();
        let leaf = miner_payload_leaf(MinerPayloadLeafInput {
            block_hash: &cand,
            blue_work_be_bytes: &blue_work_be,
            payload: &payload,
        });
        if proof.miner_payload_leaves.contains(&leaf) {
            member = Some((cand, payload, blue_work_be));
            break;
        }
    }
    let (member_hash, member_payload, member_blue_work) =
        member.unwrap_or_else(|| fatal("no mergeset member matched the node's leaf list"));

    // Assemble exactly as the peg-out relayer would, then recompute.
    let witness = SeqCommitWitness::assemble(
        member_hash,
        member_blue_work,
        member_payload.clone(),
        &proof.miner_payload_leaves,
        proof.context_hash,
        proof.lanes_root,
        proof.inactivity_shortcut,
        proof.parent_seq_commit,
    )
    .unwrap_or_else(|e| fatal(&format!("assemble: {e:?}")));

    let recomputed = witness.recompute_seq_commit().unwrap_or_else(|e| fatal(&format!("recompute: {e:?}")));

    println!("queried block   : {queried}");
    println!("mergeset member : {member_hash}");
    println!("mergeset leaves : {}", proof.miner_payload_leaves.len());
    println!("header seq_commit: {expected_seq_commit}");
    println!("recomputed      : {recomputed}");
    if let Some(r) = member_payload.get(SHIELDED_COMMITMENT_OFFSET..SHIELDED_COMMITMENT_OFFSET + 32) {
        println!("member R (#24)  : {}", hex(r));
    }

    if recomputed != expected_seq_commit {
        fatal("MISMATCH: recomputed seq_commit differs from the block header");
    }
    println!("\nOK: witness assembled from live chain data reproduces the committed seq_commit");

    // Machine-readable fields for `canonical-r-encode`. `activity_root` is already folded from
    // inactivity_shortcut + lanes_root by the assembler, so the encoder gets the value verbatim.
    if emit {
        println!("---BEGIN CANONICAL-R WITNESS---");
        println!("queried_block={queried}");
        println!("member_block={member_hash}");
        println!("member_payload={}", hex(&witness.kaspa_payload));
        println!("member_blue_work={}", hex(&witness.blue_work_be));
        println!("leaf_index={}", witness.leaf_index);
        for leaf in &witness.other_leaves {
            println!("other_leaf={leaf}");
        }
        println!("activity_root={}", witness.activity_root);
        println!("context_hash={}", witness.context_hash);
        println!("parent_seq_commit={}", witness.parent_seq_commit);
        println!("seq_commit={recomputed}");
        println!("---END CANONICAL-R WITNESS---");
    }
}

/// Lowercase hex of a byte slice.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Prints `msg` to stderr and exits non-zero.
fn fatal(msg: &str) -> ! {
    eprintln!("canonical-r-probe: {msg}");
    std::process::exit(1);
}
