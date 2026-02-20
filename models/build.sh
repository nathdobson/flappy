#!/usr/bin/env bash
set -e
set -u
cargo run --release --bin inner
cargo run --release --bin outer
cargo run --release --bin housing
cargo run --release --bin left_cap
cargo run --release --bin right_cap
cargo run --release --bin flaps -- --config flaps.json
