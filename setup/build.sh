#!/usr/bin/env bash
set -e
set -u
cargo build --release --target aarch64-apple-darwin
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target aarch64-unknown-linux-musl

cross build --release --target i686-pc-windows-gnu
cross build --release --target i686-unknown-linux-gnu

cross build --release --target x86_64-pc-windows-gnu
cross build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin
cross build --release --target x86_64-unknown-linux-musl
