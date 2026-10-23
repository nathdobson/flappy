#!/usr/bin/env bash
set -e
set -u
cd cad
cargo run --release --bin inner
cargo run --release --bin outer
cargo run --release --bin housing
cargo run --release --bin left_cap
cargo run --release --bin right_cap
cd ..
cd flaps
cargo run --release --bin flaps -- --config flaps.json
