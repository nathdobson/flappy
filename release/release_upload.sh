#!/usr/bin/env bash
set -e
set -u

VERSION=$(git describe --exact-match --tags)

cp driver/jlcpcb/production_files/BOM-driver.csv release/artifacts/driver-jlcpcb-BOM.csv
cp driver/jlcpcb/production_files/CPL-driver.csv release/artifacts/driver-jlcpcb-CPL.csv
cp driver/jlcpcb/production_files/GERBER-driver.zip release/artifacts/driver-jlcpcb-GERBER.zip
cp models/flaps.3mf release/artifacts/model-flaps.3mf
cp models/simplified/housing.3mf release/artifacts/model-housing.3mf
cp models/simplified/inner.3mf release/artifacts/model-inner.3mf
cp models/simplified/outer.3mf release/artifacts/model-outer.3mf
cp models/simplified/left-cap.3mf release/artifacts/model-left-cap.3mf
cp models/simplified/right-cap.3mf release/artifacts/model-right-cap.3mf
cp firmware/target/thumbv8m.main-none-eabihf/release/controller release/artifacts/firmware.elf

cp setup-client/target/aarch64-apple-darwin/release/setup-client release/artifacts/setup-client-aarch64-apple-darwin
cp setup-client/target/x86_64-apple-darwin/release/setup-client release/artifacts/setup-client-x86_64-apple-darwin

#cp setup-client/target/i686-pc-windows-gnu/release/setup-client.exe release/artifacts/setup-client-i686-pc-windows-gnu.exe
cp setup-client/target/x86_64-pc-windows-gnu/release/setup-client.exe release/artifacts/setup-client-x86_64-pc-windows-gnu.exe

cp setup-client/target/aarch64-unknown-linux-gnu/release/setup-client release/artifacts/setup-client-aarch64-unknown-linux-gnu
cp setup-client/target/i686-unknown-linux-gnu/release/setup-client release/artifacts/setup-client-i686-unknown-linux-gnu
#cp setup-client/target/x86_64-unknown-linux-gnu/release/setup-client release/artifacts/setup-client-x86_64-unknown-linux-gnu

#cp setup-client/target/aarch64-unknown-linux-musl/release/setup-client release/setup-client-aarch64-unknown-linux-musl
#cp setup-client/target/x86_64-unknown-linux-musl/release/setup-client release/setup-client-x86_64-unknown-linux-musl

cp web-client/web-client.zip release/artifacts/
# The order of assets listed of the website depends on upload order, so upload one file at a time
ls release/artifacts | xargs -n 1 -I{} gh release upload --clobber $VERSION release/artifacts/{}

