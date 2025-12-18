#!/usr/bin/env bash
wasm-pack build --release --target web
cp 404.html public
mkdir -p public/www
mkdir -p public/pkg
cp www/index.css public/www
cp www/index.html public/www
cp www/index.js public/www
cp www/bootstrap.js public/www
cp pkg/web_client_bg.js public/pkg
cp pkg/web_client_bg.wasm public/pkg
cp pkg/web_client.js public/pkg
rm web-client.zip
zip -r web-client.zip public
