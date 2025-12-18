#!/usr/bin/env bash
set -e
set -u
cargo run --release --bin inner
cargo run --release --bin outer
cargo run --release --bin housing
cargo run --release --bin flaps
