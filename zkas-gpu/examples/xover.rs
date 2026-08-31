//! At the batch sizes this daemon ACTUALLY uses, is the GPU faster than the CPU?
//!
//! `GPU_MIN_BATCH = 256` was chosen from `sizebench`, which compares the device against
//! `cpu_batch_agree` — a naive, single-threaded `p * ivk` at ~192 µs/pt. That is not the
//! code the daemon would otherwise run. Orchard uses a PREPARED-Wnaf multiply (74.98
//! µs/pt measured), and `scan_compact_auto` would spread it across every core.
//!
//! So the honest baseline is 74.98 / n_cores ≈ 3.1 µs/pt on this 24-core box, not 192.
//! Against that baseline the device has to clear a bar ~60x higher than the one it was
//! actually measured against.
//!
//! This matters because the timing model changed too. The ~12 ms that looked like fixed
//! per-call overhead is really the LATENCY OF ONE LADDER: it stays flat from n=2 to
//! n=1000 and only grows once n exceeds the core count, in waves. A live page is ~1000
//! actions, so the daemon pays a full ladder latency for a quarter-full device.
//!
//! Measured here, at the real page size, on the real alternative.

use group::ff::Field;
use group::{Group, WnafBase, WnafScalar};
use pasta_curves::pallas;
use rayon::prelude::*;
use std::time::Instant;

const W: usize = 4; // orchard's PREPARED_WINDOW_SIZE

fn main() {
    let gpu = match zkas_gpu::Gpu::load() {
        Some(g) => g,
        None => {
            eprintln!("no GPU; set ZKAS_GPU_LIB");
            return;
        }
    };
    let mut rng = rand::rng();
    let ivk = pallas::Scalar::random(&mut rng);
    let threads = rayon::current_num_threads();
    println!("{threads} rayon threads\n");
    println!("{:>7}  {:>12}  {:>14}  {:>10}", "batch", "GPU µs/pt", "CPU-par µs/pt", "GPU wins?");

    for n in [500usize, 1000, 2000, 4000, 8000, 16000, 32000] {
        let epks: Vec<Option<pallas::Point>> = (0..n).map(|_| Some(pallas::Point::random(&mut rng))).collect();
        let reps = if n <= 2000 { 20 } else { 6 };

        let _ = gpu.batch_agree(&ivk, &epks); // warm
        let t = Instant::now();
        for _ in 0..reps {
            std::hint::black_box(gpu.batch_agree(&ivk, &epks));
        }
        let g = t.elapsed().as_secs_f64() * 1e6 / (n * reps) as f64;

        // The real alternative: orchard's prepared-Wnaf multiply, across all cores.
        let t = Instant::now();
        for _ in 0..reps {
            let ivk_prep: WnafScalar<pallas::Scalar, W> = WnafScalar::new(&ivk);
            let out: Vec<Option<pallas::Point>> = epks
                .par_iter()
                .map(|e| e.map(|p| &WnafBase::<pallas::Point, W>::new(p) * &ivk_prep))
                .collect();
            std::hint::black_box(out);
        }
        let c = t.elapsed().as_secs_f64() * 1e6 / (n * reps) as f64;

        println!("{n:>7}  {g:>12.2}  {c:>14.2}  {:>10}", if g < c { format!("{:.2}x", c / g) } else { "NO".into() });
    }
    println!("\nA live page is ~1000 actions. Whatever wins at 1000 is what should run.");
}
