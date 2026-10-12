#!/bin/sh
# Build the CUDA kernel: a standalone bench binary, and a shared library the daemon
# dlopens at runtime. The toolchain lives under /root/zkas so nothing is installed
# system-wide; the driver was already present on the box.
#
# The daemon does NOT link this at build time — a wallet must build and run on hosts
# with no GPU and no CUDA toolkit, so the library is optional and loaded if found.
set -e
NV=${NVCC_DIR:-/root/zkas/cuda/cuda_nvcc-linux-x86_64-12.6.85-archive}
RT=${CUDART_DIR:-$(ls -d /root/zkas/cudaenv/lib/python3*/site-packages/nvidia/cuda_runtime)}
ARCH=${GPU_ARCH:-sm_75}

"$NV/bin/nvcc" -O3 -arch=$ARCH -I"$RT/include" -L"$RT/lib" \
    -o pallas_gpu pallas.cu -lcudart_static -lpthread -ldl -lrt
echo "built pallas_gpu (standalone bench + differential test)"

"$NV/bin/nvcc" -O3 -arch=$ARCH -DZKAS_GPU_LIB -Xcompiler -fPIC -shared \
    -I"$RT/include" -L"$RT/lib" \
    -o libzkas_gpu.so pallas.cu -lcudart_static -lpthread -ldl -lrt
echo "built libzkas_gpu.so (loaded by walletd at runtime)"
