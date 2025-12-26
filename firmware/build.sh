#!/usr/bin/env bash
set -e
set -u
touch firmware/build.rs
cargo build --release
