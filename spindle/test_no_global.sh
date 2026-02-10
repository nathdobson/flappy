#!/usr/bin/env bash
RUSTFLAGS="--cfg no_global_oom_handling" cargo build -Z build-std=core,compiler_builtins,alloc