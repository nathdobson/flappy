#!/bin/sh
export NODE_OPTIONS=--openssl-legacy-provider
cd www || exit
npm run start
