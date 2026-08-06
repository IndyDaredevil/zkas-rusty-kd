//! c.7 — Console observability for merged mining (WS3 increment).
//!
//! Std-only, no bridge dependencies: drop into `bridge/src/merged_obs.rs`,
//! add `pub mod merged_obs;` to lib.rs. All counters are Relaxed atomics —
//! the stats printer is the single windowed consumer, nothing here is
//! consensus-relevant, and the hot-path cost is one fetch_add per event.
//!
//! Increment-site map (upstream refs = firecash/zkas-rusty main @ 3049252;
//! [PRIVATE c.5] = merged-ws1-port settlement code):
//!   K block  -> share_handler.rs:848 (existing blocks_found site)
//!   Z block  -> [PRIVATE c.5] zkas settlement arm, on submit-accepted
//!   D block  -> ShareOutcome, see below (both arms)
//!   jobs     -> client_handler.rs:432/:729 (mining.notify send sites)
//!   kas rpc  -> client_handler.rs:216/:541 (get_block_template) or kaspaapi
//!   zkas rpc -> [PRIVATE c.4] current_zkas_template() refill success
//!   tpl dec  -> [PRIVATE c.4] decoration branch (the e3b4072 debug site)
//!   submit   -> share_handler.rs:313 handle_submit entry/exit

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn epoch_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// #1 — per-share double detection
// ---------------------------------------------------------------------------

/// One per share that clears EITHER target; Arc-clone into both settlement
/// arms. Each arm calls its `record_*_accept` on its own submit-accepted.
/// Return value `true` == this call completed a DOUBLE (increment D exactly
/// once, in whichever arm finished second).
#[derive(Default)]
pub struct ShareOutcome {
    zkas_accepted: AtomicBool,
    kas_accepted: AtomicBool,
}

impl ShareOutcome {
    pub fn new() -> Self {
        Self::default()
    }

    /// KAS arm: returns true iff the sibling zKAS accept already landed.
    pub fn record_kas_accept(&self) -> bool {
        self.kas_accepted.store(true, Relaxed);
        self.zkas_accepted.load(Relaxed)
    }

    /// zKAS arm: returns true iff the sibling KAS accept already landed.
    pub fn record_zkas_accept(&self) -> bool {
        self.zkas_accepted.store(true, Relaxed);
        self.kas_accepted.load(Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Bridge-global observability state
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct MergedObs {
    /// Set once at boot iff BOTH env vars present (c.6 gate). Drives zk=OFF.
    pub merged_enabled: AtomicBool,

    // #1 bridge-global block totals (K lives in existing WorkStats; these two
    // are the new legs; per-worker twins live in WorkStats — see patch).
    pub zkas_blocks: AtomicU64,
    pub double_blocks: AtomicU64,

    // #2 template decoration state
    pub last_zkas_tpl_ok_ms: AtomicU64,
    pub tpl_decorated_window: AtomicU64,
    pub tpl_total_window: AtomicU64,

    // #3 job cadence
    pub jobs_sent_window: AtomicU64,

    // #4 submit processing latency (µs)
    pub submit_us_sum: AtomicU64,
    pub submit_us_max: AtomicU64,
    pub submit_count_window: AtomicU64,

    // #5 last-observed RPC latency (µs); 0 == never observed
    pub kas_rpc_last_us: AtomicU64,
    pub zkas_rpc_last_us: AtomicU64,
}

/// Bridge-global instance; const-init, zero-cost until first record.
pub static MERGED_OBS: MergedObs = MergedObs::const_new();

/// RAII timer for `handle_submit`: create at fn entry, records processing
/// time into MERGED_OBS on drop — covers every early-return path with one
/// line: `let _t = merged_obs::SubmitTimer::start();`
pub struct SubmitTimer(std::time::Instant);

impl SubmitTimer {
    pub fn start() -> Self {
        Self(std::time::Instant::now())
    }
}

impl Drop for SubmitTimer {
    fn drop(&mut self) {
        MERGED_OBS.record_submit_us(self.0.elapsed().as_micros() as u64);
    }
}

impl MergedObs {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn const_new() -> Self {
        Self {
            merged_enabled: AtomicBool::new(false),
            zkas_blocks: AtomicU64::new(0),
            double_blocks: AtomicU64::new(0),
            last_zkas_tpl_ok_ms: AtomicU64::new(0),
            tpl_decorated_window: AtomicU64::new(0),
            tpl_total_window: AtomicU64::new(0),
            jobs_sent_window: AtomicU64::new(0),
            submit_us_sum: AtomicU64::new(0),
            submit_us_max: AtomicU64::new(0),
            submit_count_window: AtomicU64::new(0),
            kas_rpc_last_us: AtomicU64::new(0),
            zkas_rpc_last_us: AtomicU64::new(0),
        }
    }

    // ---- hot-path recording ----

    pub fn record_job_sent(&self) {
        self.jobs_sent_window.fetch_add(1, Relaxed);
    }

    /// Call at every decoration decision. `decorated=false` == plain parent.
    pub fn record_template(&self, decorated: bool) {
        self.tpl_total_window.fetch_add(1, Relaxed);
        if decorated {
            self.tpl_decorated_window.fetch_add(1, Relaxed);
        }
    }

    /// Call on every SUCCESSFUL zkas template fetch (cache refill).
    pub fn record_zkas_tpl_ok(&self, now_ms: u64, rpc_us: u64) {
        self.last_zkas_tpl_ok_ms.store(now_ms, Relaxed);
        self.zkas_rpc_last_us.store(rpc_us.max(1), Relaxed);
    }

    pub fn record_kas_rpc(&self, rpc_us: u64) {
        self.kas_rpc_last_us.store(rpc_us.max(1), Relaxed);
    }

    pub fn record_submit_us(&self, us: u64) {
        self.submit_us_sum.fetch_add(us, Relaxed);
        self.submit_us_max.fetch_max(us, Relaxed);
        self.submit_count_window.fetch_add(1, Relaxed);
    }

    pub fn record_zkas_block(&self) {
        self.zkas_blocks.fetch_add(1, Relaxed);
    }

    pub fn record_double_block(&self) {
        self.double_blocks.fetch_add(1, Relaxed);
    }

    // ---- printer-side (single consumer, 10s tick) ----

    /// c.8: numeric twin of `node_suffix` for Prometheus export — this IS the
    /// single window-drain point; `node_suffix` formats from its output
    /// rather than draining independently, so calling either (not both) per
    /// tick is safe and calling `node_suffix` still drains exactly once.
    pub fn snapshot(&self, now_ms: u64, elapsed_secs: f64) -> ObsSnapshot {
        let jobs = self.jobs_sent_window.swap(0, Relaxed);
        let dec = self.tpl_decorated_window.swap(0, Relaxed);
        let tot = self.tpl_total_window.swap(0, Relaxed);
        let sub_sum = self.submit_us_sum.swap(0, Relaxed);
        let sub_max = self.submit_us_max.swap(0, Relaxed);
        let sub_n = self.submit_count_window.swap(0, Relaxed);

        let jps = if elapsed_secs > 0.0 { jobs as f64 / elapsed_secs } else { 0.0 };
        let (sub_avg_ms, sub_max_ms) =
            if sub_n == 0 { (0.0, 0.0) } else { ((sub_sum as f64 / sub_n as f64) / 1000.0, sub_max as f64 / 1000.0) };

        let state = self.zk_state(now_ms, dec, tot);
        let age_secs = now_ms.saturating_sub(self.last_zkas_tpl_ok_ms.load(Relaxed)) as f64 / 1000.0;
        let dec_pct = if tot == 0 { 100 } else { (dec * 100) / tot };

        ObsSnapshot {
            zk_state: state as u8,
            zk_age_secs: age_secs,
            zk_dec_pct: dec_pct,
            jobs_per_sec: jps,
            kas_rpc_ms: self.kas_rpc_last_us.load(Relaxed) as f64 / 1000.0, // 0.0 == never observed
            zkas_rpc_ms: self.zkas_rpc_last_us.load(Relaxed) as f64 / 1000.0, // 0.0 == never observed
            submit_avg_ms: sub_avg_ms,
            submit_max_ms: sub_max_ms,
            submit_n: sub_n,
        }
    }

    /// Drain window counters and render the NODE-line suffix. `elapsed_secs`
    /// is the wall time since the previous tick. Thin wrapper over
    /// `snapshot()` + `format_obs_suffix()` — kept for callers/tests that
    /// want drain-and-render in one call; share_handler's NODE-tick calls
    /// the two halves separately so the same snapshot can also feed prom.
    pub fn node_suffix(&self, now_ms: u64, elapsed_secs: f64) -> String {
        format_obs_suffix(&self.snapshot(now_ms, elapsed_secs))
    }
}

/// Pure formatter: renders an `ObsSnapshot` into the NODE-line suffix text.
/// No atomic access — safe to call as many times as you like on the same
/// snapshot (unlike `node_suffix`/`snapshot`, which drain on every call).
pub fn format_obs_suffix(s: &ObsSnapshot) -> String {
    let fmt_rpc = |ms: f64| if ms == 0.0 { "-".to_string() } else { format!("{:.1}", ms) };
    let sub = if s.submit_n == 0 { "-".to_string() } else { format!("{:.1}/{:.1}ms", s.submit_avg_ms, s.submit_max_ms) };
    let zk = match ZkState::from_u8(s.zk_state) {
        ZkState::Off => "zk=OFF".to_string(),
        state => {
            let label = match state {
                ZkState::Ok => "ok",
                ZkState::Stale => "stale",
                ZkState::Plain => "PLAIN",
                ZkState::Off => unreachable!(),
            };
            format!("zk={} {:.1}s {}%", label, s.zk_age_secs, s.zk_dec_pct)
        }
    };
    format!("j={:.1}/s | rpc k={} z={} | sub={} | {}", s.jobs_per_sec, fmt_rpc(s.kas_rpc_ms), fmt_rpc(s.zkas_rpc_ms), sub, zk)
}

impl MergedObs {
    pub fn zk_state(&self, now_ms: u64, dec_window: u64, tot_window: u64) -> ZkState {
        if !self.merged_enabled.load(Relaxed) {
            return ZkState::Off;
        }
        let last_ok = self.last_zkas_tpl_ok_ms.load(Relaxed);
        // Never fetched successfully since boot: PLAIN (attach may claim
        // ACTIVE, but no template has ever refilled the cache).
        if last_ok == 0 {
            return ZkState::Plain;
        }
        let age = now_ms.saturating_sub(last_ok);
        if age > 5_000 || (tot_window > 0 && dec_window == 0) {
            ZkState::Plain
        } else if age > 1_000 {
            ZkState::Stale
        } else {
            ZkState::Ok
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZkState {
    Off,
    Ok,
    Stale,
    Plain,
}

impl ZkState {
    /// Inverse of `as u8` (Off=0 Ok=1 Stale=2 Plain=3, enum declaration
    /// order). Only ever fed values this module produced via `snapshot()`.
    fn from_u8(v: u8) -> Self {
        match v {
            0 => ZkState::Off,
            1 => ZkState::Ok,
            2 => ZkState::Stale,
            _ => ZkState::Plain,
        }
    }
}

/// c.8: numeric per-tick observability snapshot for Prometheus export.
/// `zk_state` uses ZkState's declaration order as its discriminant (0-3).
/// `kas_rpc_ms`/`zkas_rpc_ms` of exactly 0.0 mean "never observed" (the
/// recorders floor real latencies at 1µs, so a genuine 0.0 never occurs).
/// `submit_n == 0` means no shares were submitted this window — avg/max
/// are meaningless (left 0.0) in that case, check submit_n before using them.
#[derive(Debug, Clone, Copy)]
pub struct ObsSnapshot {
    pub zk_state: u8,
    pub zk_age_secs: f64,
    pub zk_dec_pct: u64,
    pub jobs_per_sec: f64,
    pub kas_rpc_ms: f64,
    pub zkas_rpc_ms: f64,
    pub submit_avg_ms: f64,
    pub submit_max_ms: f64,
    pub submit_n: u64,
}

// ---------------------------------------------------------------------------
// #1 rendering — K/Z/D column (BLK_W/TBLK_W widen 6 -> 8)
// ---------------------------------------------------------------------------

pub fn format_kzd(k: i64, z: i64, d: i64) -> String {
    format!("{}/{}/{}", k, z, d)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ---- ShareOutcome / double detection ----

    #[test]
    fn double_kas_finishes_second() {
        let s = ShareOutcome::new();
        assert!(!s.record_zkas_accept()); // first finisher: no double yet
        assert!(s.record_kas_accept()); // second finisher completes double
    }

    #[test]
    fn double_zkas_finishes_second() {
        let s = ShareOutcome::new();
        assert!(!s.record_kas_accept());
        assert!(s.record_zkas_accept());
    }

    #[test]
    fn zkas_only_share_never_doubles() {
        let s = ShareOutcome::new();
        assert!(!s.record_zkas_accept());
        // KAS arm never spawned (share below network target) — nothing else
        // ever fires; no double possible.
    }

    #[test]
    fn double_counted_exactly_once_under_concurrency() {
        // 200 rounds of racing both arms on threads: exactly one arm must
        // report the double, every time.
        for _ in 0..200 {
            let s = Arc::new(ShareOutcome::new());
            let s1 = Arc::clone(&s);
            let s2 = Arc::clone(&s);
            let t1 = std::thread::spawn(move || s1.record_kas_accept() as u32);
            let t2 = std::thread::spawn(move || s2.record_zkas_accept() as u32);
            let doubles = t1.join().unwrap() + t2.join().unwrap();
            assert_eq!(doubles, 1, "exactly one arm must complete the double");
        }
    }

    // ---- zk state derivation ----

    fn armed_obs(last_ok_ms: u64) -> MergedObs {
        let o = MergedObs::new();
        o.merged_enabled.store(true, Relaxed);
        o.last_zkas_tpl_ok_ms.store(last_ok_ms, Relaxed);
        o
    }

    #[test]
    fn zk_off_when_merged_disabled() {
        let o = MergedObs::new(); // merged_enabled defaults false
        assert_eq!(o.zk_state(10_000, 5, 5), ZkState::Off);
        assert_eq!(o.node_suffix(10_000, 10.0).contains("zk=OFF"), true);
    }

    #[test]
    fn zk_plain_when_never_fetched() {
        let o = MergedObs::new();
        o.merged_enabled.store(true, Relaxed);
        assert_eq!(o.zk_state(10_000, 0, 0), ZkState::Plain);
    }

    #[test]
    fn zk_ok_within_1s() {
        let o = armed_obs(10_000);
        assert_eq!(o.zk_state(10_500, 3, 3), ZkState::Ok);
        assert_eq!(o.zk_state(11_000, 3, 3), ZkState::Ok); // boundary inclusive
    }

    #[test]
    fn zk_stale_between_1s_and_5s() {
        let o = armed_obs(10_000);
        assert_eq!(o.zk_state(11_001, 3, 3), ZkState::Stale);
        assert_eq!(o.zk_state(15_000, 3, 3), ZkState::Stale); // boundary
    }

    #[test]
    fn zk_plain_past_5s() {
        let o = armed_obs(10_000);
        assert_eq!(o.zk_state(15_001, 3, 3), ZkState::Plain);
    }

    #[test]
    fn zk_plain_when_decorating_nothing_despite_fresh_fetch() {
        // Fresh template fetches but 0% decoration == serving plain: PLAIN.
        let o = armed_obs(10_000);
        assert_eq!(o.zk_state(10_200, 0, 32), ZkState::Plain);
    }

    #[test]
    fn zk_empty_window_does_not_false_plain() {
        // No templates cut this window (idle tick) but fetch is fresh: Ok.
        let o = armed_obs(10_000);
        assert_eq!(o.zk_state(10_200, 0, 0), ZkState::Ok);
    }

    // ---- window semantics / suffix rendering ----

    #[test]
    fn node_suffix_drains_window_and_computes_rates() {
        let o = armed_obs(9_900);
        for _ in 0..32 {
            o.record_job_sent();
        }
        for i in 0..10 {
            o.record_template(i % 2 == 0); // 50% decorated
        }
        o.record_submit_us(400);
        o.record_submit_us(2_200); // avg 1300µs -> "1.3", max -> "2.2"
        o.record_kas_rpc(1_234);
        o.record_zkas_tpl_ok(9_900, 3_456);

        let s = o.node_suffix(10_000, 10.0);
        assert!(s.contains("j=3.2/s"), "cadence: {s}");
        assert!(s.contains("rpc k=1.2 z=3.5"), "rpc: {s}");
        assert!(s.contains("sub=1.3/2.2ms"), "submit avg/max: {s}");
        assert!(s.contains("zk=ok 0.1s 50%"), "zk: {s}");

        // window fully drained; last-observed rpc persists by design
        let s2 = o.node_suffix(20_000, 10.0);
        assert!(s2.contains("j=0.0/s"), "drained: {s2}");
        assert!(s2.contains("sub=-"), "drained submit: {s2}");
        assert!(s2.contains("rpc k=1.2 z=3.5"), "rpc persists: {s2}");
    }

    #[test]
    fn node_suffix_renders_dashes_before_first_observations() {
        let o = MergedObs::new();
        let s = o.node_suffix(1_000, 10.0);
        assert!(s.contains("rpc k=- z=-"), "{s}");
        assert!(s.contains("sub=-"), "{s}");
        assert!(s.contains("zk=OFF"), "{s}");
    }

    // ---- K/Z/D rendering + width ----

    #[test]
    fn kzd_fits_widened_column() {
        const BLK_W: usize = 8; // widened from 6
        assert!(format_kzd(0, 0, 0).len() <= BLK_W);
        assert!(format_kzd(2, 41, 2).len() <= BLK_W); // realistic soak day
        assert!(format_kzd(12, 345, 12).len() > BLK_W); // documented overflow
        let row = format!("{:>BLK_W$}", format_kzd(2, 41, 2));
        assert_eq!(row.len(), BLK_W); // right-aligned, border stays true
    }

    #[test]
    fn totals_double_never_exceeds_min_leg() {
        // Sanity property the printer can assert in debug builds:
        // D <= min(K, Z) always (a double IS one K and one Z).
        let o = MergedObs::new();
        o.record_zkas_block();
        o.record_zkas_block();
        o.record_double_block();
        let z = o.zkas_blocks.load(Relaxed);
        let d = o.double_blocks.load(Relaxed);
        assert!(d <= z);
    }
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    use std::sync::atomic::Ordering::Relaxed;

    #[test]
    fn submit_timer_records_on_every_exit_path() {
        fn multi_exit(early: bool) -> i32 {
            let _t = SubmitTimer::start();
            if early {
                return 1; // early return still records via Drop
            }
            2
        }
        let before = MERGED_OBS.submit_count_window.load(Relaxed);
        multi_exit(true);
        multi_exit(false);
        let after = MERGED_OBS.submit_count_window.load(Relaxed);
        assert_eq!(after - before, 2);
    }
}

#[cfg(test)]
mod c8_tests {
    use super::*;
    use std::sync::atomic::Ordering::Relaxed;

    #[test]
    fn snapshot_and_node_suffix_drain_exactly_once_not_twice() {
        // Regression guard for the c.8 double-drain hazard: node_suffix()
        // must fully consume the window, leaving nothing for a second call.
        let o = MergedObs::new();
        o.merged_enabled.store(true, Relaxed);
        o.record_job_sent();
        o.record_job_sent();
        o.record_template(true);

        let first = o.node_suffix(1_000, 1.0);
        assert!(first.contains("j=2.0/s"), "{first}");

        let second = o.node_suffix(2_000, 1.0);
        assert!(second.contains("j=0.0/s"), "window not drained: {second}");
    }

    #[test]
    fn format_obs_suffix_is_pure_no_drain() {
        // format_obs_suffix takes a snapshot by value/ref and must not touch
        // atomics — calling it twice on the same ObsSnapshot is identical.
        let snap = ObsSnapshot {
            zk_state: ZkState::Ok as u8,
            zk_age_secs: 0.3,
            zk_dec_pct: 100,
            jobs_per_sec: 3.2,
            kas_rpc_ms: 1.2,
            zkas_rpc_ms: 3.4,
            submit_avg_ms: 0.4,
            submit_max_ms: 2.1,
            submit_n: 5,
        };
        assert_eq!(format_obs_suffix(&snap), format_obs_suffix(&snap));
        assert!(format_obs_suffix(&snap).contains("zk=ok 0.3s 100%"));
    }

    #[test]
    fn snapshot_matches_node_suffix_rendering() {
        // The two entry points must describe the same reality: render via
        // node_suffix, independently render via snapshot+format_obs_suffix
        // on a twin instance fed identical events, and compare.
        let a = MergedObs::new();
        let b = MergedObs::new();
        for o in [&a, &b] {
            o.merged_enabled.store(true, Relaxed);
            o.record_zkas_tpl_ok(900, 500);
            o.record_template(true);
            o.record_job_sent();
            o.record_kas_rpc(800);
            o.record_submit_us(1_000);
        }
        let via_suffix = a.node_suffix(1_000, 1.0);
        let via_snapshot = format_obs_suffix(&b.snapshot(1_000, 1.0));
        assert_eq!(via_suffix, via_snapshot);
    }

    #[test]
    fn snapshot_zk_state_discriminant_roundtrips_through_from_u8() {
        for state in [ZkState::Off, ZkState::Ok, ZkState::Stale, ZkState::Plain] {
            assert_eq!(ZkState::from_u8(state as u8) as u8, state as u8);
        }
    }

    #[test]
    fn snapshot_never_observed_rpc_renders_dash_not_zero() {
        let o = MergedObs::new();
        o.merged_enabled.store(true, Relaxed);
        let snap = o.snapshot(1_000, 1.0);
        assert_eq!(snap.kas_rpc_ms, 0.0);
        assert_eq!(snap.zkas_rpc_ms, 0.0);
        assert!(format_obs_suffix(&snap).contains("rpc k=- z=-"));
    }
}
