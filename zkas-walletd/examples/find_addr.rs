//! Offline: map wallet tokens to receive addresses so an operator can find the wallet
//! behind an address a user reports. Reads the wallet files only — no daemon, no node.
//!
//!   cargo run --release --example find_addr -- <wallet-dir> [address-substring]

use kaspa_addresses::{Address, Prefix, Version};
use kaspa_shielded_core::walletdb::WalletDb;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: find_addr <wallet-dir> [needle]");
    let needle = args.next();
    let mut scanned = 0usize;

    for entry in std::fs::read_dir(&dir).expect("read wallet dir").flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let token = path.file_stem().unwrap().to_string_lossy().to_string();
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
        let fvk_hex = v.get("fvk_hex").and_then(|f| f.as_str()).unwrap_or("");
        if fvk_hex.is_empty() {
            continue; // seed-bearing wallet; not what the hosted daemon holds
        }
        let Ok(raw) = hex::decode(fvk_hex) else { continue };
        let Ok(fvk) = <[u8; 96]>::try_from(raw.as_slice()) else { continue };
        let Some(db) = WalletDb::from_fvk(&fvk) else { continue };
        let addr = String::from(&Address::new(Prefix::Mainnet, Version::ShieldedOrchard, &db.my_address_bytes()));
        scanned += 1;
        match &needle {
            Some(n) if addr.contains(n.as_str()) => println!("MATCH {token} {addr}"),
            Some(_) => {}
            None => println!("{token} {addr}"),
        }
    }
    eprintln!("scanned {scanned} watch-only wallet file(s)");
}
