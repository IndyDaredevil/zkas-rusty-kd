//! Cross-stack vectors: a real spend bundle proven on one Orchard stack must verify on the other.
//! Run with `--features circuit -- --ignored --nocapture`. Stack name from XVER_NAME env.
#![cfg(feature = "circuit")]
use kaspa_shielded_core::bundle::ShieldedBundle;
use kaspa_shielded_core::verify::{sighash, verify_bundle, verify_bundles_batched};
use kaspa_shielded_core::wallet::{address_bytes_from_seed, build::build_singleleaf_coinbase_spend};
use std::time::Instant;

const NET: [u8; 32] = [0x77; 32];
const CTX: &[u8] = b"xver-e2e";
const DIR: &str = "/root/zkas/mobile-test/xver";

#[test]
#[ignore]
fn emit() {
    let name = std::env::var("XVER_NAME").expect("XVER_NAME");
    let recipient = address_bytes_from_seed([6u8; 32]).unwrap();
    let t = Instant::now();
    let bytes = build_singleleaf_coinbase_spend([5u8; 32], [9u8; 32], 0, 10_000, recipient, 8_000, &NET, CTX).unwrap();
    println!("[{name}] built 1-spend bundle: {} bytes in {:.2?} (includes proving-key build)", bytes.len(), t.elapsed());
    std::fs::create_dir_all(DIR).unwrap();
    std::fs::write(format!("{DIR}/{name}.bin"), &bytes).unwrap();
    let wire = ShieldedBundle::from_bytes(&bytes).unwrap();
    verify_bundle(&wire, &sighash(&wire, &NET, CTX)).expect("self-verify");
    println!("[{name}] self-verify OK; proof len {}", wire.proof.len());
}

#[test]
#[ignore]
fn verify_all() {
    let name = std::env::var("XVER_NAME").expect("XVER_NAME");
    let mut ok = 0;
    for e in std::fs::read_dir(DIR).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|x| x != "bin").unwrap_or(true) { continue; }
        let bytes = std::fs::read(&p).unwrap();
        let wire = ShieldedBundle::from_bytes(&bytes).unwrap();
        let msg = sighash(&wire, &NET, CTX);
        let t = Instant::now();
        let r = verify_bundle(&wire, &msg);
        let single = t.elapsed();
        let t = Instant::now();
        let rb = verify_bundles_batched(&[(&wire, msg)], rand::rng());
        println!("[{name} verifier] vector {:?}: single={:?} ({:.2?}) batched={:?} ({:.2?})", p.file_name().unwrap(), r, single, rb, t.elapsed());
        assert!(r.is_ok() && rb.is_ok(), "cross-verification FAILED for {:?}", p);
        ok += 1;
    }
    println!("[{name} verifier] {ok} vectors verified");
    assert!(ok > 0);
}

/// The scan-side cost centre: Sinsemilla Merkle hashing (80% of a wallet scan).
#[test]
#[ignore]
fn tree_bench() {
    use incrementalmerkletree::{Hashable, Level};
    use kaspa_shielded_core::MerkleHashOrchard;
    let name = std::env::var("XVER_NAME").expect("XVER_NAME");
    let a = MerkleHashOrchard::empty_leaf();
    let b = MerkleHashOrchard::empty_root(Level::from(3));
    let n = 50_000u32;
    let t = Instant::now();
    let mut acc = a;
    for i in 0..n { acc = MerkleHashOrchard::combine(Level::from((i % 32) as u8), &acc, &b); }
    let dt = t.elapsed();
    println!("[{name}] sinsemilla combine: {n} ops in {:.2?} = {:.1} µs/op  (chk {:?})", dt, dt.as_secs_f64() * 1e6 / n as f64, &acc.to_bytes()[..4]);
}


/// Deterministic fields (anchor, nullifier) must be byte-identical across stacks — proves the
/// note-commitment / nullifier / tree derivations did not change, only the proof randomness.
#[test]
#[ignore]
fn compare_deterministic_fields() {
    let a = ShieldedBundle::from_bytes(&std::fs::read(format!("{DIR}/pristine.bin")).unwrap()).unwrap();
    let b = ShieldedBundle::from_bytes(&std::fs::read(format!("{DIR}/zakura.bin")).unwrap()).unwrap();
    assert_eq!(a.anchor, b.anchor, "anchor differs");
    assert_eq!(a.flags, b.flags); assert_eq!(a.value_balance, b.value_balance);
    assert_eq!(a.actions.len(), b.actions.len());
    let na: Vec<_> = a.actions.iter().map(|x| x.nullifier).collect();
    let nb: Vec<_> = b.actions.iter().map(|x| x.nullifier).collect();
    // dummy-spend nullifiers are random; the real spend's nullifier must appear in both.
    let shared = na.iter().filter(|n| nb.contains(n)).count();
    println!("[compare] anchor identical; {} of {} nullifiers shared (real spend), proof len {} / {}", shared, na.len(), a.proof.len(), b.proof.len());
    assert!(shared >= 1, "real-spend nullifier differs between stacks");
}
