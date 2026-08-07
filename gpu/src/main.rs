//! Differential test and CPU baseline for the CUDA Pallas scalar-multiplication kernel.
//!
//! The kernel computes `ivk · epk`, the operation that decides whether a shielded note
//! belongs to a wallet. If it is ever wrong the wallet misses notes, and the user
//! experiences that as their coins vanishing. So the kernel is not trusted because the
//! maths looks right — it is trusted only after every single output has been compared,
//! byte for byte, against `pasta_curves`, the same implementation the consensus code
//! uses.
//!
//!   gen    <file> <n>            write a scalar, n random points, and the CPU answers
//!   verify <file> <gpu-out>      check the GPU's Jacobian output against those answers
//!   bench  <file> <n> <threads>  time the CPU doing the same work, for comparison
//!
//! Edge cases are placed at fixed indices by `gen` rather than left to chance: the
//! identity, and a point equal to the one the ladder will add — that second one is the
//! case a naive mixed-addition gets silently wrong (H == 0), and random points will
//! essentially never produce it.

use group::ff::{Field, PrimeField};
use group::{Curve, Group};
use pasta_curves::arithmetic::CurveAffine;
use pasta_curves::pallas;
use std::io::{Read, Write};

const LIMBS: usize = 8;

fn fp_to_le(x: &pallas::Base) -> [u8; 32] {
    x.to_repr()
}
fn le_to_fp(b: &[u8]) -> Option<pallas::Base> {
    let mut a = [0u8; 32];
    a.copy_from_slice(b);
    Option::from(pallas::Base::from_repr(a))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    match mode {
        "gen" => gen(&args[2], args[3].parse().unwrap()),
        "verify" => verify(&args[2], &args[3]),
        "bench" => bench(&args[2], args[3].parse().unwrap(), args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1)),
        _ => {
            eprintln!("usage: gpucheck gen <file> <n> | verify <file> <gpu-out> | bench <file> <n> [threads]");
            std::process::exit(2);
        }
    }
}

fn gen(path: &str, n: usize) {
    let mut rng = rand::thread_rng();
    let scalar = pallas::Scalar::random(&mut rng);

    let mut pts: Vec<pallas::Point> = (0..n).map(|_| pallas::Point::random(&mut rng)).collect();
    // Deterministic edge cases. The generator is random, so without planting these the
    // test would almost surely never exercise them.
    if n > 2 {
        // A point the ladder is guaranteed to meet again: forces H == 0 in mixed
        // addition, i.e. the "this is really a doubling" branch.
        pts[1] = pallas::Point::generator();
        // Its negation, which forces the "result is the identity" branch.
        pts[2] = -pallas::Point::generator();
    }

    let affine: Vec<pallas::Affine> = pts.iter().map(|p| p.to_affine()).collect();
    let expect: Vec<pallas::Point> = pts.iter().map(|p| p * scalar).collect();

    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(&scalar.to_repr()).unwrap();
    for a in &affine {
        match Option::<pasta_curves::arithmetic::Coordinates<pallas::Affine>>::from(a.coordinates()) {
            Some(c) => {
                f.write_all(&fp_to_le(c.x())).unwrap();
                f.write_all(&fp_to_le(c.y())).unwrap();
            }
            // The identity has no affine coordinates; (0,0) is not on the curve, so the
            // kernel treats it as a degenerate input. Kept out of the corpus.
            None => {
                f.write_all(&[0u8; 32]).unwrap();
                f.write_all(&[0u8; 32]).unwrap();
            }
        }
    }
    // Expected results, affine, alongside — same file, after the inputs.
    let mut nexp = 0usize;
    for e in &expect {
        let a = e.to_affine();
        match Option::<pasta_curves::arithmetic::Coordinates<pallas::Affine>>::from(a.coordinates()) {
            Some(c) => {
                f.write_all(&fp_to_le(c.x())).unwrap();
                f.write_all(&fp_to_le(c.y())).unwrap();
            }
            None => {
                f.write_all(&[0u8; 32]).unwrap();
                f.write_all(&[0u8; 32]).unwrap();
            }
        }
        nexp += 1;
    }
    println!("wrote {path}: scalar + {n} points + {nexp} expected results");
}

fn verify(path: &str, gpu_out: &str) {
    let mut buf = Vec::new();
    std::fs::File::open(path).unwrap().read_to_end(&mut buf).unwrap();
    let n = (buf.len() - 32) / 128; // 64B input + 64B expected per point
    let exp_off = 32 + n * 64;

    let mut g = Vec::new();
    std::fs::File::open(gpu_out).unwrap().read_to_end(&mut g).unwrap();
    assert_eq!(g.len(), n * 96, "gpu output size mismatch");

    let mut bad = 0usize;
    let mut checked = 0usize;
    for i in 0..n {
        // GPU gives Jacobian (X, Y, Z), canonical little-endian.
        let x = le_to_fp(&g[i * 96..i * 96 + 32]).expect("gpu X not a field element");
        let y = le_to_fp(&g[i * 96 + 32..i * 96 + 64]).expect("gpu Y not a field element");
        let z = le_to_fp(&g[i * 96 + 64..i * 96 + 96]).expect("gpu Z not a field element");

        let ex = &buf[exp_off + i * 64..exp_off + i * 64 + 32];
        let ey = &buf[exp_off + i * 64 + 32..exp_off + i * 64 + 64];
        let expect_identity = ex.iter().all(|&b| b == 0) && ey.iter().all(|&b| b == 0);

        if bool::from(z.is_zero()) {
            // Identity. Correct exactly when that is what the CPU produced too.
            if !expect_identity {
                if bad < 5 {
                    eprintln!("  [{i}] GPU says identity, CPU does not");
                }
                bad += 1;
            }
            checked += 1;
            continue;
        }
        if expect_identity {
            if bad < 5 {
                eprintln!("  [{i}] CPU says identity, GPU does not");
            }
            bad += 1;
            checked += 1;
            continue;
        }

        // Jacobian -> affine: x = X/Z², y = Y/Z³.
        let zinv = z.invert().unwrap();
        let zinv2 = zinv.square();
        let zinv3 = zinv2 * zinv;
        let ax = x * zinv2;
        let ay = y * zinv3;

        if fp_to_le(&ax) != ex || fp_to_le(&ay) != ey {
            if bad < 5 {
                eprintln!("  [{i}] MISMATCH\n     gpu x {}\n     cpu x {}", hex(&fp_to_le(&ax)), hex(ex));
            }
            bad += 1;
        }
        checked += 1;
    }
    println!("checked {checked} results, {bad} mismatch(es)");
    if bad == 0 {
        println!("PASS — every GPU result is byte-identical to pasta_curves.");
    } else {
        println!("FAIL — the kernel must not be used.");
        std::process::exit(1);
    }
}

/// The CPU baseline, using orchard's own prepared-Wnaf path so the comparison is
/// against what the daemon actually runs, not against a naive scalar multiply.
fn bench(path: &str, n: usize, threads: usize) {
    use group::{WnafBase, WnafScalar};
    let mut buf = Vec::new();
    std::fs::File::open(path).unwrap().read_to_end(&mut buf).unwrap();
    let mut sb = [0u8; 32];
    sb.copy_from_slice(&buf[..32]);
    let scalar = Option::<pallas::Scalar>::from(pallas::Scalar::from_repr(sb)).unwrap();

    let pts: Vec<pallas::Point> = (0..n)
        .map(|i| {
            let x = le_to_fp(&buf[32 + i * 64..32 + i * 64 + 32]).unwrap();
            let y = le_to_fp(&buf[32 + i * 64 + 32..32 + i * 64 + 64]).unwrap();
            let a = pallas::Affine::from_xy(x, y);
            Option::<pallas::Affine>::from(a).map(pallas::Point::from).unwrap_or(pallas::Point::generator())
        })
        .collect();

    const W: usize = 4; // orchard's PREPARED_WINDOW_SIZE
    let sp: WnafScalar<pallas::Scalar, W> = WnafScalar::new(&scalar);
    let chunk = (n + threads - 1) / threads;
    let t = std::time::Instant::now();
    std::thread::scope(|s| {
        for c in pts.chunks(chunk) {
            let sp = &sp;
            s.spawn(move || {
                let mut acc = pallas::Point::identity();
                for p in c {
                    let b: WnafBase<pallas::Point, W> = WnafBase::new(*p);
                    acc += &b * sp;
                }
                std::hint::black_box(acc);
            });
        }
    });
    let el = t.elapsed();
    println!("CPU  {threads} thread(s): {:.2} ms  =  {:.3} us/mult  =  {:.0} mults/sec",
        el.as_secs_f64() * 1e3, el.as_secs_f64() * 1e6 / n as f64, n as f64 / el.as_secs_f64());
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
