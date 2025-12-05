#!/bin/sh
wasm-pack build || exit
export NODE_OPTIONS=--openssl-legacy-provider
cd www || exit
npm run start
