# GPU trial decryption — measured, verified, not yet wired

`ivk · epk` is the operation that decides whether a shielded note belongs to a wallet.
It is the one part of a wallet scan that **cannot** be shared between wallets: two
wallets have different viewing keys by definition. Measured on the live daemon it is
75.6% of trial-decryption cost, and after the shared tree lands it is essentially the
whole cost of a scan. So it is the only thing on the GPU's menu worth ordering.

## Results — RTX 2080 Ti (Turing, sm_75), 100,000 points, scalar 254 bits

| | per mult | throughput | vs GPU |
|---|---:|---:|---:|
| **GPU** | **1.567 µs** | **638,354 /s** | — |
| CPU, 1 thread | 81.723 µs | 12,237 /s | **52.2× slower** |
| CPU, 12 threads | 19.858 µs | 50,357 /s | **12.7× slower** |

The CPU baseline is orchard's own prepared-Wnaf path (`WnafBase` + `WnafScalar`,
window 4), not a naive scalar multiply — i.e. the comparison is against what the
daemon actually runs. The 12-thread figure is on a shared box also running walletd,
kaspad and other services, so it reflects real available capacity rather than a
dedicated machine.

**Correctness: 100,000 / 100,000 results byte-identical to `pasta_curves`**, the same
implementation consensus uses. Including planted edge cases the random corpus would
never produce: the generator (forces `H == 0`, the "this addition is really a
doubling" branch that a naive implementation gets silently wrong) and its negation
(forces the identity result).

## What it means for a scan

Trial decryption is 105.6 µs/action, of which 79.9 µs is this curve work. Replacing
that with 1.567 µs:

```
105.6 µs  ->  27.3 µs per action        3.87x on decryption
```

which is close to the 4.11× Amdahl ceiling measured earlier — the remaining 25.7 µs
is KDF, ChaCha20, parsing and dispatch, none of which this kernel touches.

End to end, per cold wallet:

```
today                          398 s   (tree 319 s + decrypt 79 s)
+ shared tree (borrow-tree)     79 s   5.0x
+ this kernel                 ~20.5 s  19.4x
```

**Sequencing matters more than the kernel.** Today decrypt is 20% of a scan, so this
GPU alone would give ~1.2× overall. It only pays *after* the tree work is shared. Do
not wire it in first.

## Why this is not the same call as GPU proving

GPU proving was assessed and rejected: Orchard is Pasta/IPA, where MSM/FFT kernels
barely exist, working sets are large, and there are real data dependencies. This is
the opposite shape — N independent scalar multiplications, no communication, and
every thread multiplies by the *same* scalar, so the ladder's branches are uniform
across a warp. Zero divergence by construction.

## Deliberately not done yet

- **Not wired into walletd.** The shared tree comes first (above), and integration
  needs FFI, page batching, and a CPU fallback for hosts with no GPU.
- **Not optimised.** The field multiply is schoolbook + REDC in plain CUDA C, chosen
  so it could be *read* and verified. Inline PTX carry chains (`mad.lo.cc`/`madc.hi.cc`)
  typically give 2–3× on bignum, and a windowed/NAF ladder would cut the ~127 additions.
  638 K/s is the floor, not the ceiling.
- **Not constant-time.** The ladder branches on scalar bits, so kernel duration
  correlates with the Hamming weight of the ivk. Upstream orchard accepts the same
  property for a wallet-local key ("variable-time with respect to the wallet-local
  viewing key scalar"). On a **multi-tenant** daemon holding 548 viewing keys that
  assumption is weaker, and a fixed-window ladder with dummy additions would close it.
  Worth deciding before this serves other people's wallets.

## Reproducing

```
gpucheck gen   vec.bin 100000      # CPU writes inputs + expected answers
pallas_gpu     vec.bin out.bin 100000 5
gpucheck verify vec.bin out.bin    # every result, byte for byte
gpucheck bench vec.bin 100000 12   # CPU baseline, orchard's own path
```

Toolchain lives entirely under `/root/zkas` (nvcc 12.6.85 from NVIDIA's component
redistributable, driver 580 already present). Nothing was installed system-wide.
