#!/usr/bin/env bash
set -e
set -u
touch firmware/build.rs
cargo build --release
cargo objcopy --bin controller --release --verbose -- -O binary target/firmware.bin
