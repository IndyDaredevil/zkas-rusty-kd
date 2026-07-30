//! Offline IP → country lookup for the network-map endpoint.
//!
//! The tables are built from the five RIRs' public `delegated-extended` stats
//! files (ARIN, RIPE NCC, APNIC, LACNIC, AFRINIC) by `scripts/gen-geo.py`, so
//! there is no licence key, no runtime network call, and no third-party service
//! ever learns which peers this node is connected to.
//!
//! This is **registry** geolocation: it reports the country an address block is
//! *allocated to*, which is normally where it is routed but can differ from the
//! physical location of the host. Resolution is deliberately country-level — the
//! explorer never publishes a peer's address, only the country it falls in, so
//! the map cannot be used to locate an individual node operator.
//!
//! Layout (all little-endian, records sorted by `start`, ranges merged):
//!   `geoip.bin`     `[start: u32][end: u32][cc: u16]`  — IPv4 addresses
//!   `geoip6.bin`    `[start: u64][end: u64][cc: u16]`  — IPv6, indexed by the
//!                                                        high 64 bits (/64)
//!   `countries.bin` `[cc: u16][lat: f32][lon: f32][len: u8][name: utf8]`
//!
//! `cc` packs the two ASCII letters of the ISO-3166-1 alpha-2 code into a u16,
//! so lookups compare integers rather than strings.

use std::{collections::HashMap, net::IpAddr, sync::LazyLock};

const V4_TABLE: &[u8] = include_bytes!("../data/geoip.bin");
const V6_TABLE: &[u8] = include_bytes!("../data/geoip6.bin");
const COUNTRY_TABLE: &[u8] = include_bytes!("../data/countries.bin");

const V4_REC: usize = 10;
const V6_REC: usize = 18;

/// A country's ISO code, display name and centroid.
#[derive(Debug, Clone)]
pub struct Country {
    pub code: String,
    pub name: String,
    pub lat: f32,
    pub lon: f32,
}

fn cc_str(cc: u16) -> String {
    let a = (cc & 0xff) as u8 as char;
    let b = (cc >> 8) as u8 as char;
    format!("{a}{b}")
}

static COUNTRIES: LazyLock<HashMap<u16, Country>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    let mut o = 0usize;
    while o + 11 <= COUNTRY_TABLE.len() {
        let cc = u16::from_le_bytes([COUNTRY_TABLE[o], COUNTRY_TABLE[o + 1]]);
        let lat = f32::from_le_bytes(COUNTRY_TABLE[o + 2..o + 6].try_into().unwrap());
        let lon = f32::from_le_bytes(COUNTRY_TABLE[o + 6..o + 10].try_into().unwrap());
        let len = COUNTRY_TABLE[o + 10] as usize;
        o += 11;
        if o + len > COUNTRY_TABLE.len() {
            break;
        }
        let name = String::from_utf8_lossy(&COUNTRY_TABLE[o..o + len]).into_owned();
        o += len;
        map.insert(cc, Country { code: cc_str(cc), name, lat, lon });
    }
    map
});

/// Binary-search a sorted, non-overlapping range table for the record covering `key`.
///
/// `rec` is the record stride and `read` pulls `(start, end, cc)` out of record `i`.
fn search(table: &[u8], rec: usize, key: u128, read: impl Fn(usize) -> (u128, u128, u16)) -> Option<u16> {
    let n = table.len() / rec;
    if n == 0 {
        return None;
    }
    // Largest record whose start <= key.
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if read(mid).0 <= key { lo = mid + 1 } else { hi = mid }
    }
    if lo == 0 {
        return None;
    }
    let (start, end, cc) = read(lo - 1);
    (start <= key && key <= end).then_some(cc)
}

/// Resolve an address to its allocated country, or `None` if the address is
/// unallocated, private, or in a block no RIR reports (e.g. legacy IANA space).
pub fn lookup(ip: IpAddr) -> Option<&'static Country> {
    let cc = match ip {
        IpAddr::V4(v4) => {
            if v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified() {
                return None;
            }
            let key = u32::from(v4) as u128;
            search(V4_TABLE, V4_REC, key, |i| {
                let o = i * V4_REC;
                let s = u32::from_le_bytes(V4_TABLE[o..o + 4].try_into().unwrap()) as u128;
                let e = u32::from_le_bytes(V4_TABLE[o + 4..o + 8].try_into().unwrap()) as u128;
                (s, e, u16::from_le_bytes([V4_TABLE[o + 8], V4_TABLE[o + 9]]))
            })?
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() || v6.is_unspecified() {
                return None;
            }
            // An IPv4-mapped address (::ffff:a.b.c.d) is really an IPv4 peer.
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return lookup(IpAddr::V4(mapped));
            }
            let key = (u128::from(v6) >> 64) as u128;
            search(V6_TABLE, V6_REC, key, |i| {
                let o = i * V6_REC;
                let s = u64::from_le_bytes(V6_TABLE[o..o + 8].try_into().unwrap()) as u128;
                let e = u64::from_le_bytes(V6_TABLE[o + 8..o + 16].try_into().unwrap()) as u128;
                (s, e, u16::from_le_bytes([V6_TABLE[o + 16], V6_TABLE[o + 17]]))
            })?
        }
    };
    COUNTRIES.get(&cc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cc_of(s: &str) -> Option<String> {
        lookup(s.parse().unwrap()).map(|c| c.code.clone())
    }

    #[test]
    fn tables_are_well_formed() {
        assert_eq!(V4_TABLE.len() % V4_REC, 0, "v4 table must be a whole number of records");
        assert_eq!(V6_TABLE.len() % V6_REC, 0, "v6 table must be a whole number of records");
        assert!(V4_TABLE.len() / V4_REC > 100_000, "v4 table looks truncated");
        assert!(COUNTRIES.len() > 200, "country centroid table looks truncated");
    }

    #[test]
    fn v4_table_is_sorted_and_disjoint() {
        let n = V4_TABLE.len() / V4_REC;
        let mut prev_end = 0u32;
        for i in 0..n {
            let o = i * V4_REC;
            let s = u32::from_le_bytes(V4_TABLE[o..o + 4].try_into().unwrap());
            let e = u32::from_le_bytes(V4_TABLE[o + 4..o + 8].try_into().unwrap());
            assert!(s <= e, "record {i} is inverted");
            if i > 0 {
                assert!(s > prev_end, "record {i} overlaps its predecessor");
            }
            prev_end = e;
        }
    }

    #[test]
    fn known_addresses_resolve() {
        // Well-known anycast/public ranges with stable registry allocations.
        assert_eq!(cc_of("8.8.8.8").as_deref(), Some("US"));
        assert_eq!(cc_of("1.1.1.1").as_deref(), Some("AU")); // APNIC research block
        // Every resolved country must carry a usable centroid.
        for ip in ["8.8.8.8", "51.210.219.1", "94.50.212.1"] {
            let c = lookup(ip.parse().unwrap()).unwrap();
            assert!(c.lat.abs() <= 90.0 && c.lon.abs() <= 180.0, "{ip} centroid out of range");
            assert!(!c.name.is_empty());
        }
    }

    #[test]
    fn private_and_unroutable_addresses_are_unmapped() {
        for ip in ["127.0.0.1", "10.0.0.1", "192.168.1.1", "169.254.1.1", "::1"] {
            assert!(lookup(ip.parse().unwrap()).is_none(), "{ip} should not geolocate");
        }
    }

    #[test]
    fn ipv4_mapped_v6_follows_the_v4_table() {
        assert_eq!(cc_of("::ffff:8.8.8.8"), cc_of("8.8.8.8"));
    }
}
