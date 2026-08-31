//! Optional GPU acceleration for the one part of a wallet scan that cannot be shared
//! between wallets: `ivk · epk`, the Pallas key agreement that decides whether a
//! shielded note belongs to you.
//!
//! Measured on an RTX 2080 Ti: **1.567 µs/mult against 81.7 µs on one CPU core (52×)
//! and 19.9 µs across twelve (12.7×)**. The CPU figure is orchard's own prepared-Wnaf
//! path, so that is a comparison against what the daemon actually runs.
//!
//! # Loaded, not linked
//!
//! `libzkas_gpu.so` is opened with `dlopen` at startup rather than linked at build
//! time. A wallet must build and run on hosts with no GPU and no CUDA toolkit, so a
//! build dependency on CUDA would be unacceptable. No library, no device, or any
//! runtime failure — every one of those falls back to the CPU, and the caller cannot
//! tell the difference except in speed.
//!
//! # Trust
//!
//! The kernel is not trusted because the arithmetic looks right. Its outputs were
//! compared byte-for-byte against `pasta_curves` — the same implementation consensus
//! uses — over 100,000 points including planted edge cases (the generator, forcing the
//! `H == 0` "this addition is really a doubling" branch that naive mixed addition gets
//! silently wrong, and its negation, forcing the identity). See `gpu/README.md`.
//!
//! On top of that, [`Gpu::batch_agree`] re-checks every point it gets back: a result
//! that is not a valid curve point, or that fails to convert, disables the GPU for the
//! process. A wrong answer here is a missed note, which a user experiences as their
//! coins vanishing, so the failure mode is "fall back to the CPU", never "carry on".

use group::ff::PrimeField;
use group::{Curve, Group};
use pasta_curves::pallas;
use std::sync::atomic::{AtomicBool, Ordering};

const LIMBS: usize = 8;

type DeviceCountFn = unsafe extern "C" fn() -> i32;
type BatchKaFn = unsafe extern "C" fn(*const u32, i32, *const u32, *mut u32, i32) -> i32;

/// A loaded CUDA backend. Absent means "use the CPU", which is always correct.
pub struct Gpu {
    _lib: libloading::Library,
    batch_ka: BatchKaFn,
    devices: i32,
    /// Latches on any failure. Once the GPU has misbehaved once we stop asking it:
    /// a device that returned one bad answer has no claim to the next one.
    poisoned: AtomicBool,
}

/// Where to look for the kernel. An explicit path wins so an operator can point at a
/// build without installing anything.
fn candidates() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("ZKAS_GPU_LIB") {
        v.push(p);
    }
    v.push("libzkas_gpu.so".into());
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join("libzkas_gpu.so").to_string_lossy().into_owned());
        }
    }
    v.push("/root/zkas/gpu/libzkas_gpu.so".into());
    v
}

impl Gpu {
    /// Try to bring up the GPU backend. `None` on any problem, and the caller uses the
    /// CPU — which is the same answer, only slower.
    pub fn load() -> Option<Self> {
        for path in candidates() {
            let lib = match unsafe { libloading::Library::new(&path) } {
                Ok(l) => l,
                Err(_) => continue,
            };
            let devices = unsafe {
                let f: libloading::Symbol<DeviceCountFn> = lib.get(b"zkas_gpu_device_count").ok()?;
                f()
            };
            if devices <= 0 {
                log::info!("GPU library at {path} loaded but reports no usable device; using the CPU");
                continue;
            }
            let batch_ka = unsafe {
                let f: libloading::Symbol<BatchKaFn> = lib.get(b"zkas_gpu_batch_ka").ok()?;
                *f
            };
            log::info!("GPU acceleration enabled: {devices} device(s) via {path}");
            return Some(Gpu { _lib: lib, batch_ka, devices, poisoned: AtomicBool::new(false) });
        }
        None
    }

    pub fn devices(&self) -> i32 {
        self.devices
    }

    pub fn is_usable(&self) -> bool {
        !self.poisoned.load(Ordering::Relaxed)
    }

    fn poison(&self, why: &str) {
        if !self.poisoned.swap(true, Ordering::Relaxed) {
            log::error!("GPU disabled for the rest of this process: {why}. Falling back to the CPU — results are unaffected, only speed.");
        }
    }

    /// `ivk · epk` for a whole batch. `None` means "I could not do this" — never a
    /// partial or approximate answer — and the caller must use the CPU path.
    ///
    /// `epks[i] == None` marks an ephemeral key that was not a valid encoding; those
    /// positions come back `None` too, matching what the CPU path produces.
    pub fn batch_agree(&self, ivk: &pallas::Scalar, epks: &[Option<pallas::Point>]) -> Option<Vec<Option<pallas::Affine>>> {
        if !self.is_usable() || epks.is_empty() {
            return None;
        }
        // Pack the scalar as canonical little-endian limbs, and find its top set bit so
        // the ladder does not spend 255 doublings on a short scalar.
        let sb = ivk.to_repr();
        let mut scalar = [0u32; LIMBS];
        for i in 0..LIMBS {
            scalar[i] = u32::from_le_bytes([sb[i * 4], sb[i * 4 + 1], sb[i * 4 + 2], sb[i * 4 + 3]]);
        }
        let mut bits = 0i32;
        for i in (0..LIMBS * 32).rev() {
            if (scalar[i >> 5] >> (i & 31)) & 1 == 1 {
                bits = i as i32 + 1;
                break;
            }
        }
        if bits == 0 {
            return None; // a zero scalar is not something to hand a kernel
        }

        // Only the valid keys are sent; the invalid slots are re-inserted afterwards, so
        // the GPU never has to represent "absent" and the batch stays dense.
        let idx: Vec<usize> = epks.iter().enumerate().filter(|(_, e)| e.is_some()).map(|(i, _)| i).collect();
        if idx.is_empty() {
            return Some(vec![None; epks.len()]);
        }
        // ONE inversion for the whole batch, not one per point.
        //
        // `to_affine()` performs a modular inversion each time it is called, so mapping
        // it over the batch cost ~10 us/point — dwarfing the 1.567 us the kernel takes
        // and making the whole GPU path look like a marshalling problem. Montgomery's
        // trick turns N inversions into one plus 3N multiplications.
        let proj: Vec<pallas::Point> = idx.iter().map(|&i| epks[i].unwrap()).collect();
        let mut affine = vec![pallas::Affine::default(); proj.len()];
        pallas::Point::batch_normalize(&proj, &mut affine);

        let mut input = vec![0u32; idx.len() * 2 * LIMBS];
        for (k, a) in affine.iter().enumerate() {
            let c = match Option::<pasta_curves::arithmetic::Coordinates<pallas::Affine>>::from(
                <pallas::Affine as pasta_curves::arithmetic::CurveAffine>::coordinates(a),
            ) {
                Some(c) => c,
                None => return None, // the identity has no affine coordinates
            };
            let xb = <pallas::Base as group::ff::PrimeField>::to_repr(c.x());
            let yb = <pallas::Base as group::ff::PrimeField>::to_repr(c.y());
            for l in 0..LIMBS {
                input[k * 2 * LIMBS + l] = u32::from_le_bytes([xb[l * 4], xb[l * 4 + 1], xb[l * 4 + 2], xb[l * 4 + 3]]);
                input[k * 2 * LIMBS + LIMBS + l] =
                    u32::from_le_bytes([yb[l * 4], yb[l * 4 + 1], yb[l * 4 + 2], yb[l * 4 + 3]]);
            }
        }

        let mut out = vec![0u32; idx.len() * 3 * LIMBS];
        let rc = unsafe {
            (self.batch_ka)(scalar.as_ptr(), bits, input.as_ptr(), out.as_mut_ptr(), idx.len() as i32)
        };
        if rc != 0 {
            self.poison(&format!("kernel returned {rc}"));
            return None;
        }

        // Jacobian -> affine for the whole batch: ONE field inversion via Montgomery's
        // trick rather than one per point, done by pasta_curves so this step rests on an
        // implementation that is already trusted.
        let mut jac = Vec::with_capacity(idx.len());
        for k in 0..idx.len() {
            let rd = |off: usize| -> Option<pallas::Base> {
                let mut b = [0u8; 32];
                for l in 0..LIMBS {
                    b[l * 4..l * 4 + 4].copy_from_slice(&out[k * 3 * LIMBS + off + l].to_le_bytes());
                }
                Option::from(<pallas::Base as group::ff::PrimeField>::from_repr(b))
            };
            // Any limb that is not a field element means the kernel produced garbage.
            let (x, y, z) = match (rd(0), rd(LIMBS), rd(2 * LIMBS)) {
                (Some(x), Some(y), Some(z)) => (x, y, z),
                _ => {
                    self.poison("kernel returned a value outside the field");
                    return None;
                }
            };
            jac.push((x, y, z));
        }

        // Batch-invert every Z at once.
        let zs: Vec<pallas::Base> = jac.iter().map(|(_, _, z)| *z).collect();
        let mut zinv = zs.clone();
        group::ff::BatchInverter::invert_with_external_scratch(&mut zinv, &mut zs.clone());

        let mut dense = Vec::with_capacity(idx.len());
        for (k, (x, y, z)) in jac.iter().enumerate() {
            if bool::from(group::ff::Field::is_zero(z)) {
                dense.push(None); // identity: this ephemeral key agrees to nothing
                continue;
            }
            let zi = zinv[k];
            let zi2 = zi * zi;
            let zi3 = zi2 * zi;
            let ax = *x * zi2;
            let ay = *y * zi3;
            match Option::<pallas::Affine>::from(<pallas::Affine as pasta_curves::arithmetic::CurveAffine>::from_xy(ax, ay)) {
                Some(p) => dense.push(Some(p)),
                None => {
                    // Not on the curve — the one outcome that must never be used.
                    self.poison("kernel returned a point that is not on the curve");
                    return None;
                }
            }
        }

        let mut result = vec![None; epks.len()];
        for (k, &i) in idx.iter().enumerate() {
            result[i] = dense[k];
        }
        Some(result)
    }
}

impl Gpu {
    /// The shape `shielded-core`'s hook expects: scalar and points in, affine shared
    /// secrets out, `None` meaning "use the CPU".
    pub fn batch_agree_points(
        &self,
        ivk: &pallas::Scalar,
        epks: &[Option<pallas::Point>],
    ) -> Option<Vec<Option<pallas::Affine>>> {
        self.batch_agree(ivk, epks)
    }
}

/// The CPU answer, for the differential check and for hosts with no GPU.
pub fn cpu_batch_agree(ivk: &pallas::Scalar, epks: &[Option<pallas::Point>]) -> Vec<Option<pallas::Affine>> {
    epks.iter().map(|e| e.map(|p| (p * ivk).to_affine())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use group::ff::Field;

    /// If a GPU is present its answers must equal the CPU's, exactly. If none is
    /// present the test still passes — this crate's contract is "same answer, maybe
    /// faster", and "no GPU" satisfies it.
    #[test]
    fn gpu_agrees_with_the_cpu_or_is_absent() {
        let Some(gpu) = Gpu::load() else {
            eprintln!("no GPU available — CPU path only, which is a valid configuration");
            return;
        };
        let mut rng = rand::rng();
        let ivk = pallas::Scalar::random(&mut rng);
        let mut epks: Vec<Option<pallas::Point>> = (0..1024).map(|_| Some(pallas::Point::random(&mut rng))).collect();
        // Planted edge cases: an absent key, and the generator (whose ladder hits the
        // "this addition is really a doubling" branch).
        epks[3] = None;
        epks[7] = Some(pallas::Point::generator());

        eprintln!("GPU present: {} device(s) — comparing {} points against the CPU", gpu.devices(), epks.len());
        let got = gpu.batch_agree(&ivk, &epks).expect("gpu answered");
        let want = cpu_batch_agree(&ivk, &epks);
        assert_eq!(got.len(), want.len());
        for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            assert_eq!(g.is_some(), w.is_some(), "presence differs at {i}");
            if let (Some(g), Some(w)) = (g, w) {
                assert_eq!(g, w, "GPU and CPU disagree at {i}");
            }
        }
    }
}
