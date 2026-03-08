#!/bin/bash
set -e

export CARGO_BUILD_TARGET=thumbv6m-none-eabi
mkdir -p /out

echo "rehan's epic rust based hsm buildier now running"
cargo build --release

echo "build complete"
