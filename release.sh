#!/usr/bin/env bash
set -e
set -u
cp driver/jlcpcb/production_files/BOM-driver.csv release/driver-jlcpcb-BOM.csv
cp driver/jlcpcb/production_files/CPL-driver.csv release/driver-jlcpcb-CPL.csv
cp driver/jlcpcb/production_files/GERBER-driver.zip release/driver-jlcpcb-GERBER.zip
cp mobile/flappy.apk release/mobile.apk
cp models/flaps.3mf release/model-flaps.3mf
cp models/simplified/housing.3mf release/model-housing.3mf
cp models/simplified/inner.3mf release/model-inner.3mf
cp models/simplified/outer.3mf release/model-outer.3mf
cp firmware/target/thumbv8m.main-none-eabihf/release/controller release/firmware.elf
cp setup/target/aarch64-apple-darwin/release/setup release/setup-aarch64-apple-darwin
cp setup/target/x86_64-apple-darwin/release/setup release/setup-x86_64-apple-darwin
mkdir -p release/web-client/www
mkdir -p release/web-client/pkg
cp web-client/www/index.css release/web-client/www
cp web-client/www/index.html release/web-client/www
cp web-client/www/index.js release/web-client/www
cp web-client/pkg/web_client_bg.js release/web-client/pkg
cp web-client/pkg/web_client_bg.wasm release/web-client/pkg
cp web-client/pkg/web_client.js release/web-client/pkg
cp web-client/firebase.json release/web-client

