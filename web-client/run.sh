#!/bin/sh
wasm-pack build --target web || exit
export NODE_OPTIONS=--openssl-legacy-provider
cd www || exit
npm run start
