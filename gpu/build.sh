#!/bin/sh
# Build the CUDA kernel. The toolchain lives under /root/zkas so nothing is installed
# system-wide; the driver (580) was already present on the box.
set -e
NV=/root/zkas/cuda/cuda_nvcc-linux-x86_64-12.6.85-archive
RT=$(ls -d /root/zkas/cudaenv/lib/python3*/site-packages/nvidia/cuda_runtime)
"$NV/bin/nvcc" -O3 -arch=sm_75 -I"$RT/include" -L"$RT/lib" \
    -o pallas_gpu pallas.cu -lcudart_static -lpthread -ldl -lrt
echo "built pallas_gpu"
