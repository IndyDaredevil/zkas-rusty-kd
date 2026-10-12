// Map wallet files to their zkas: address, to locate a user's wallet by address.
// Reads a dir of <token>.json wallet files; prints "<token> <address> <kind>".
// Never prints key material.

use kaspa_addresses::{Address, Prefix, Version};
use kaspa_shielded_core::walletdb::WalletDb;

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect())
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: addr-of <wallet-dir>");
    let mut rd: Vec<_> = std::fs::read_dir(&dir).expect("read dir").flatten().collect();
    rd.sort_by_key(|e| e.file_name());
    for e in rd {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.ends_with(".json") {
            continue;
        }
        let token = name.trim_end_matches(".json").to_string();
        let Ok(text) = std::fs::read_to_string(e.path()) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let seed_hex = v.get("seed_hex").and_then(|s| s.as_str()).unwrap_or("");
        let fvk_hex = v.get("fvk_hex").and_then(|s| s.as_str()).unwrap_or("");
        let (kind, db) = if let Some(db) =
            unhex(fvk_hex).and_then(|b| <[u8; 96]>::try_from(b.as_slice()).ok()).and_then(|f| WalletDb::from_fvk(&f))
        {
            ("fvk", db)
        } else if let Some(db) = unhex(seed_hex).and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok()).and_then(WalletDb::from_seed) {
            ("seed", db)
        } else {
            println!("{token} <encrypted-or-unknown> -");
            continue;
        };
        let addr = Address::new(Prefix::Mainnet, Version::ShieldedOrchard, &db.my_address_bytes());
        println!("{token} {addr} {kind}");
    }
}
