#!/usr/bin/env bash
set -e
set -u
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# CMake isn't happy
#cross build --release --target i686-pc-windows-gnu
cross build --release --target x86_64-pc-windows-gnu

cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target i686-unknown-linux-gnu
cross build --release --target armv7-unknown-linux-gnueabihf
# Hits this bug: https://gcc.gnu.org/bugzilla/show_bug.cgi?id=95189.
#cross build --release --target x86_64-unknown-linux-gnu

# BLE requires Dbus on linux, which doesn't seem to directly support musl.
cross build --release --target aarch64-unknown-linux-musl --no-default-features --features usb
cross build --release --target x86_64-unknown-linux-musl  --no-default-features --features usb
cross build --release --target i686-unknown-linux-musl --no-default-features --features usb
cross build --release --target armv7-unknown-linux-musleabihf --no-default-features --features usb
cross build --release --target arm-unknown-linux-musleabihf --no-default-features --features usb
