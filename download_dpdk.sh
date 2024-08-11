#!/bin/sh

if [ ! -d "./run-dpdk-sys/deps/dpdk" ]; then
    git clone --depth=1 -b v23.07 https://github.com/DPDK/dpdk.git ./run-dpdk-sys/deps/dpdk
fi
