// What batch size does the GPU actually need to beat the CPU?
// The integration calls it per TRANSACTION — a handful of actions — so the answer
// decides whether it can be used there at all, or whether the ingest must batch first.
use group::ff::Field;
use group::{Curve, Group};
use pasta_curves::pallas;
use std::time::Instant;

fn main() {
    let gpu = zkas_gpu::Gpu::load().expect("gpu");
    let mut rng = rand::rng();
    let ivk = pallas::Scalar::random(&mut rng);
    println!("{:>8}  {:>12}  {:>12}  {:>8}", "batch", "GPU us/pt", "CPU us/pt", "speedup");
    for n in [2usize, 5, 10, 50, 200, 1000, 5000, 20000, 100000] {
        let epks: Vec<Option<pallas::Point>> = (0..n).map(|_| Some(pallas::Point::random(&mut rng))).collect();
        // warm
        let _ = gpu.batch_agree(&ivk, &epks);
        let reps = if n <= 200 { 200 } else if n <= 5000 { 20 } else { 3 };
        let t = Instant::now();
        for _ in 0..reps { std::hint::black_box(gpu.batch_agree(&ivk, &epks)); }
        let g = t.elapsed().as_secs_f64() * 1e6 / (n * reps) as f64;
        let t = Instant::now();
        for _ in 0..reps { std::hint::black_box(zkas_gpu::cpu_batch_agree(&ivk, &epks)); }
        let c = t.elapsed().as_secs_f64() * 1e6 / (n * reps) as f64;
        println!("{:>8}  {:>12.2}  {:>12.2}  {:>7.2}x", n, g, c, c / g);
    }
}
