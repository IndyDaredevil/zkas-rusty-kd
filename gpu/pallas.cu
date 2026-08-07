// Pallas variable-base scalar multiplication on CUDA.
//
// WHY THIS EXISTS
// ---------------
// Trial decryption is the only part of a wallet scan that cannot be shared between
// wallets: ownership of a note is decided by `ivk · epk`, a Pallas scalar
// multiplication, and two wallets have different ivks by definition. Measured on the
// live daemon it is 105.6 µs/action, of which 79.9 µs (75.6%) is exactly this curve
// operation — so it is the whole of what a GPU could ever accelerate here.
//
// It is also the *ideal* GPU shape, and for a reason worth stating: every thread
// multiplies its own point by the SAME scalar. The scalar's bits drive the ladder's
// control flow, so every thread in a warp takes the same branch at the same step.
// Zero divergence, by construction. (This is what makes it a different proposition
// from GPU proving, which was rejected: that needs MSM/FFT over Pasta with real data
// dependencies and large working sets.)
//
// CORRECTNESS IS NOT NEGOTIABLE
// -----------------------------
// A wrong result here is a missed note, which a user experiences as "my coins
// vanished". This file is therefore paired with a differential test: the CPU
// (pasta_curves, the same library consensus uses) generates inputs and expected
// outputs, and every single GPU result is compared byte-for-byte. A kernel that has
// not passed that test must never be wired into a wallet.
//
// REPRESENTATION
// --------------
// Field elements: 8 × uint32 limbs, little-endian, Montgomery form (R = 2^256 mod p).
// 32-bit limbs rather than 64-bit because GPUs synthesise 64-bit integer multiply
// from 32-bit pieces anyway; going straight to 32 avoids paying for the emulation.
// Points: Jacobian (X, Y, Z) with x = X/Z², y = Y/Z³; Z = 0 marks the identity.
// Pallas is y² = x³ + 5, so a = 0 and the fast a=0 formulas apply.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cuda_runtime.h>

#define LIMBS 8

// p = 0x40000000000000000000000000000000224698fc094cf91b992d30ed00000001
__device__ __constant__ uint32_t P_MOD[LIMBS] = {0x00000001u, 0x992d30edu, 0x094cf91bu, 0x224698fcu,
                                                 0x00000000u, 0x00000000u, 0x00000000u, 0x40000000u};
// R² mod p — converts a canonical value into Montgomery form via one mul.
__device__ __constant__ uint32_t P_R2[LIMBS] = {0x0000000fu, 0x8c78ecb3u, 0x8b0de0e7u, 0xd7d30dbdu,
                                                0xc3c95d18u, 0x7797a99bu, 0x7b9cb714u, 0x096d41afu};
// -p^{-1} mod 2^32. p ≡ 1 (mod 2^32), so this is exactly 2^32 - 1.
__device__ __constant__ uint32_t P_INV = 0xffffffffu;
// 1 in Montgomery form (= R mod p).
__device__ __constant__ uint32_t P_ONE[LIMBS] = {0xfffffffdu, 0x34786d38u, 0xe41914adu, 0x992c350bu,
                                                 0xffffffffu, 0xffffffffu, 0xffffffffu, 0x3fffffffu};

typedef struct { uint32_t v[LIMBS]; } fe;
typedef struct { fe X, Y, Z; } pt;      // Jacobian
typedef struct { fe x, y; } pt_aff;

__device__ __forceinline__ void fe_zero(fe &r) {
#pragma unroll
    for (int i = 0; i < LIMBS; i++) r.v[i] = 0;
}
__device__ __forceinline__ bool fe_is_zero(const fe &a) {
    uint32_t acc = 0;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) acc |= a.v[i];
    return acc == 0;
}
__device__ __forceinline__ bool fe_eq(const fe &a, const fe &b) {
    uint32_t acc = 0;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) acc |= (a.v[i] ^ b.v[i]);
    return acc == 0;
}
/// a >= p ?
__device__ __forceinline__ bool fe_ge_p(const fe &a) {
#pragma unroll
    for (int i = LIMBS - 1; i >= 0; i--) {
        if (a.v[i] < P_MOD[i]) return false;
        if (a.v[i] > P_MOD[i]) return true;
    }
    return true; // equal
}
__device__ __forceinline__ void fe_sub_p(fe &a) {
    uint64_t borrow = 0;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) {
        uint64_t d = (uint64_t)a.v[i] - P_MOD[i] - borrow;
        a.v[i] = (uint32_t)d;
        borrow = (d >> 63) & 1;
    }
}
__device__ __forceinline__ void fe_add(fe &r, const fe &a, const fe &b) {
    uint64_t carry = 0;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) {
        uint64_t s = (uint64_t)a.v[i] + b.v[i] + carry;
        r.v[i] = (uint32_t)s;
        carry = s >> 32;
    }
    // p < 2^255, so a+b < 2^256 and the sum needs at most one conditional subtract.
    if (carry || fe_ge_p(r)) fe_sub_p(r);
}
__device__ __forceinline__ void fe_sub(fe &r, const fe &a, const fe &b) {
    uint64_t borrow = 0;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) {
        uint64_t d = (uint64_t)a.v[i] - b.v[i] - borrow;
        r.v[i] = (uint32_t)d;
        borrow = (d >> 63) & 1;
    }
    if (borrow) { // add p back
        uint64_t carry = 0;
#pragma unroll
        for (int i = 0; i < LIMBS; i++) {
            uint64_t s = (uint64_t)r.v[i] + P_MOD[i] + carry;
            r.v[i] = (uint32_t)s;
            carry = s >> 32;
        }
    }
}
__device__ __forceinline__ void fe_dbl(fe &r, const fe &a) { fe_add(r, a, a); }

/// Montgomery multiplication: r = a·b·R⁻¹ mod p. Schoolbook product then REDC.
/// Written for clarity over cycles — it is the piece most likely to be wrong, and a
/// wrong field multiply is a wrong note. Optimise only after the differential test
/// is green.
__device__ void fe_mul(fe &r, const fe &a, const fe &b) {
    uint32_t t[2 * LIMBS + 1];
#pragma unroll
    for (int i = 0; i < 2 * LIMBS + 1; i++) t[i] = 0;

    // t = a * b
#pragma unroll
    for (int i = 0; i < LIMBS; i++) {
        uint32_t carry = 0;
#pragma unroll
        for (int j = 0; j < LIMBS; j++) {
            uint64_t cur = (uint64_t)a.v[i] * b.v[j] + t[i + j] + carry;
            t[i + j] = (uint32_t)cur;
            carry = (uint32_t)(cur >> 32);
        }
        // propagate the tail carry
        int idx = i + LIMBS;
        while (carry) {
            uint64_t s = (uint64_t)t[idx] + carry;
            t[idx] = (uint32_t)s;
            carry = (uint32_t)(s >> 32);
            idx++;
        }
    }

    // REDC: fold p in, one limb at a time, so the low half becomes zero.
#pragma unroll
    for (int i = 0; i < LIMBS; i++) {
        uint32_t m = t[i] * P_INV; // mod 2^32 implicitly
        uint32_t carry = 0;
#pragma unroll
        for (int j = 0; j < LIMBS; j++) {
            uint64_t cur = (uint64_t)m * P_MOD[j] + t[i + j] + carry;
            t[i + j] = (uint32_t)cur;
            carry = (uint32_t)(cur >> 32);
        }
        int idx = i + LIMBS;
        while (carry) {
            uint64_t s = (uint64_t)t[idx] + carry;
            t[idx] = (uint32_t)s;
            carry = (uint32_t)(s >> 32);
            idx++;
        }
    }

#pragma unroll
    for (int i = 0; i < LIMBS; i++) r.v[i] = t[i + LIMBS];
    // The extra word t[16] can only be 0 or 1; either it is set, or the value may
    // still be >= p. Both mean "subtract p once".
    if (t[2 * LIMBS] || fe_ge_p(r)) fe_sub_p(r);
}
__device__ __forceinline__ void fe_sqr(fe &r, const fe &a) { fe_mul(r, a, a); }

/// Canonical -> Montgomery (multiply by R²), and back (multiply by 1).
__device__ __forceinline__ void fe_to_mont(fe &r, const fe &a) {
    fe r2;
#pragma unroll
    for (int i = 0; i < LIMBS; i++) r2.v[i] = P_R2[i];
    fe_mul(r, a, r2);
}
__device__ __forceinline__ void fe_from_mont(fe &r, const fe &a) {
    fe one;
    fe_zero(one);
    one.v[0] = 1;
    fe_mul(r, a, one);
}

/// Jacobian doubling for a = 0 (EFD "dbl-2009-l"): 2M + 5S.
/// Z1 = 0 (identity) propagates correctly: Z3 = 2·Y1·Z1 = 0.
__device__ void pt_dbl(pt &r, const pt &q) {
    fe A, B, C, D, E, F, t0, t1;
    fe_sqr(A, q.X);            // A = X²
    fe_sqr(B, q.Y);            // B = Y²
    fe_sqr(C, B);              // C = B²
    fe_add(t0, q.X, B);        // X + B
    fe_sqr(t0, t0);            // (X+B)²
    fe_sub(t0, t0, A);
    fe_sub(t0, t0, C);
    fe_dbl(D, t0);             // D = 2((X+B)² - A - C)
    fe_dbl(t1, A);
    fe_add(E, t1, A);          // E = 3A
    fe_sqr(F, E);              // F = E²
    fe_dbl(t0, D);
    fe_sub(r.X, F, t0);        // X3 = F - 2D
    fe_sub(t0, D, r.X);
    fe_mul(t0, E, t0);         // E(D - X3)
    fe_dbl(t1, C);
    fe_dbl(t1, t1);
    fe_dbl(t1, t1);            // 8C
    fe_sub(r.Y, t0, t1);       // Y3 = E(D-X3) - 8C
    fe_mul(t0, q.Y, q.Z);
    fe_dbl(r.Z, t0);           // Z3 = 2·Y·Z
}

/// Mixed addition: Jacobian += affine (EFD "madd-2007-bl"), 7M + 4S.
/// Handles the identity on either side and the doubling case, which a naive
/// implementation gets silently wrong exactly when acc == P.
__device__ void pt_madd(pt &r, const pt &q, const pt_aff &a) {
    if (fe_is_zero(q.Z)) { // acc is the identity -> result is just `a`, as Jacobian Z=1
        r.X = a.x;
        r.Y = a.y;
#pragma unroll
        for (int i = 0; i < LIMBS; i++) r.Z.v[i] = P_ONE[i];
        return;
    }
    fe Z1Z1, U2, S2, H, HH, I, J, rr, V, t0, t1;
    fe_sqr(Z1Z1, q.Z);
    fe_mul(U2, a.x, Z1Z1);
    fe_mul(S2, a.y, q.Z);
    fe_mul(S2, S2, Z1Z1);
    fe_sub(H, U2, q.X);
    fe_sub(rr, S2, q.Y);
    if (fe_is_zero(H)) {
        if (fe_is_zero(rr)) { // acc == a -> this is a doubling, not an addition
            pt_dbl(r, q);
            return;
        }
        // acc == -a -> the identity
        fe_zero(r.X);
        fe_zero(r.Y);
        fe_zero(r.Z);
        r.X.v[0] = 1;
        r.Y.v[0] = 1;
        return;
    }
    fe_sqr(HH, H);
    fe_dbl(I, HH);
    fe_dbl(I, I);              // I = 4·HH
    fe_mul(J, H, I);
    fe_dbl(rr, rr);            // r = 2(S2 - Y1)
    fe_mul(V, q.X, I);
    fe_sqr(t0, rr);
    fe_sub(t0, t0, J);
    fe_dbl(t1, V);
    fe_sub(r.X, t0, t1);       // X3 = r² - J - 2V
    fe_sub(t0, V, r.X);
    fe_mul(t0, rr, t0);
    fe_mul(t1, q.Y, J);
    fe_dbl(t1, t1);
    fe_sub(r.Y, t0, t1);       // Y3 = r(V - X3) - 2·Y1·J
    fe_add(t0, q.Z, H);
    fe_sqr(t0, t0);
    fe_sub(t0, t0, Z1Z1);
    fe_sub(r.Z, t0, HH);       // Z3 = (Z1+H)² - Z1Z1 - HH
}

// The scalar (the ivk), canonical little-endian, in constant memory. It is the SAME
// for every thread — that is the property that removes all warp divergence from the
// ladder below, and it is also exactly how a wallet scan works: one viewing key
// against a whole page of ephemeral keys.
__device__ __constant__ uint32_t SCALAR[LIMBS];
__device__ __constant__ int SCALAR_BITS;

/// r = scalar · P, MSB-first double-and-add.
///
/// Deliberately NOT windowed. A window would need a per-thread precomputed table of
/// multiples of P (P is different in every thread), which costs registers or local
/// memory — and local memory here would cost far more than the additions it saves.
/// With a shared scalar the branch is uniform across the warp, so the naive ladder
/// runs at full occupancy.
__device__ void pt_mul(pt &r, const pt_aff &P) {
    fe_zero(r.X);
    fe_zero(r.Y);
    fe_zero(r.Z); // Z = 0 is the identity
    r.X.v[0] = 1;
    r.Y.v[0] = 1;
    for (int i = SCALAR_BITS - 1; i >= 0; i--) {
        pt t;
        pt_dbl(t, r);
        r = t;
        if ((SCALAR[i >> 5] >> (i & 31)) & 1u) {
            pt u;
            pt_madd(u, r, P);
            r = u;
        }
    }
}

/// One thread per ephemeral key. Input and output are CANONICAL (non-Montgomery)
/// little-endian limbs, so the host never has to know about Montgomery form.
__global__ void ka_kernel(const uint32_t *__restrict__ in_xy, uint32_t *__restrict__ out_xyz, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;

    pt_aff P;
#pragma unroll
    for (int k = 0; k < LIMBS; k++) P.x.v[k] = in_xy[(size_t)i * 2 * LIMBS + k];
#pragma unroll
    for (int k = 0; k < LIMBS; k++) P.y.v[k] = in_xy[(size_t)i * 2 * LIMBS + LIMBS + k];
    fe_to_mont(P.x, P.x);
    fe_to_mont(P.y, P.y);

    pt R;
    pt_mul(R, P);

    // Back to canonical. The Jacobian -> affine inverse is left to the caller: it is
    // one field inversion for a WHOLE batch (Montgomery's trick) instead of one per
    // point, which is why doing it here would be strictly worse.
    fe ox, oy, oz;
    fe_from_mont(ox, R.X);
    fe_from_mont(oy, R.Y);
    fe_from_mont(oz, R.Z);
#pragma unroll
    for (int k = 0; k < LIMBS; k++) out_xyz[(size_t)i * 3 * LIMBS + k] = ox.v[k];
#pragma unroll
    for (int k = 0; k < LIMBS; k++) out_xyz[(size_t)i * 3 * LIMBS + LIMBS + k] = oy.v[k];
#pragma unroll
    for (int k = 0; k < LIMBS; k++) out_xyz[(size_t)i * 3 * LIMBS + 2 * LIMBS + k] = oz.v[k];
}

#define CK(call)                                                                                   \
    do {                                                                                           \
        cudaError_t e = (call);                                                                    \
        if (e != cudaSuccess) {                                                                    \
            fprintf(stderr, "CUDA error %s at %s:%d\n", cudaGetErrorString(e), __FILE__, __LINE__); \
            return 2;                                                                              \
        }                                                                                          \
    } while (0)

// Input file (written by the Rust generator):
//   [32B scalar canonical LE][n × 64B affine point: x LE, y LE]
// Output file:
//   [n × 96B Jacobian: X LE, Y LE, Z LE]
int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <in.bin> <out.bin> <n> [iters]\n", argv[0]);
        return 2;
    }
    const char *in_path = argv[1];
    const char *out_path = argv[2];
    int n = atoi(argv[3]);
    int iters = argc > 4 ? atoi(argv[4]) : 1;

    FILE *f = fopen(in_path, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", in_path); return 2; }
    uint32_t scalar[LIMBS];
    if (fread(scalar, 4, LIMBS, f) != LIMBS) { fprintf(stderr, "short read (scalar)\n"); return 2; }
    size_t in_words = (size_t)n * 2 * LIMBS;
    uint32_t *h_in = (uint32_t *)malloc(in_words * 4);
    if (fread(h_in, 4, in_words, f) != in_words) { fprintf(stderr, "short read (points)\n"); return 2; }
    fclose(f);

    // Highest set bit of the scalar — the ladder must not waste 255 doublings on a
    // small scalar, and must not skip real bits on a full-width one.
    int bits = 0;
    for (int i = LIMBS * 32 - 1; i >= 0; i--) {
        if ((scalar[i >> 5] >> (i & 31)) & 1u) { bits = i + 1; break; }
    }
    CK(cudaMemcpyToSymbol(SCALAR, scalar, sizeof(scalar)));
    CK(cudaMemcpyToSymbol(SCALAR_BITS, &bits, sizeof(bits)));

    size_t out_words = (size_t)n * 3 * LIMBS;
    uint32_t *h_out = (uint32_t *)malloc(out_words * 4);
    uint32_t *d_in = nullptr, *d_out = nullptr;
    CK(cudaMalloc(&d_in, in_words * 4));
    CK(cudaMalloc(&d_out, out_words * 4));
    CK(cudaMemcpy(d_in, h_in, in_words * 4, cudaMemcpyHostToDevice));

    int block = 128;
    int grid = (n + block - 1) / block;

    // Warm-up: the first launch pays JIT/context costs that have nothing to do with
    // the arithmetic, and quoting those as the kernel's speed would be a lie.
    ka_kernel<<<grid, block>>>(d_in, d_out, n);
    CK(cudaDeviceSynchronize());

    cudaEvent_t t0, t1;
    CK(cudaEventCreate(&t0));
    CK(cudaEventCreate(&t1));
    CK(cudaEventRecord(t0));
    for (int it = 0; it < iters; it++) ka_kernel<<<grid, block>>>(d_in, d_out, n);
    CK(cudaEventRecord(t1));
    CK(cudaEventSynchronize(t1));
    float ms = 0;
    CK(cudaEventElapsedTime(&ms, t0, t1));
    CK(cudaGetLastError());

    CK(cudaMemcpy(h_out, d_out, out_words * 4, cudaMemcpyDeviceToHost));
    FILE *g = fopen(out_path, "wb");
    if (!g) { fprintf(stderr, "cannot write %s\n", out_path); return 2; }
    fwrite(h_out, 4, out_words, g);
    fclose(g);

    double total_ms = ms / iters;
    double per_us = total_ms * 1000.0 / n;
    printf("scalar bits      : %d\n", bits);
    printf("points           : %d  (grid %d x block %d, %d iter(s))\n", n, grid, block, iters);
    printf("kernel time      : %.2f ms\n", total_ms);
    printf("per scalar mult  : %.3f us\n", per_us);
    printf("throughput       : %.0f mults/sec\n", n / (total_ms / 1000.0));
    return 0;
}
