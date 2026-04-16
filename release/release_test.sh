#!/usr/bin/env bash
set -e
set -u
(cd firmware; cargo hack check --feature-powerset --no-dev-deps --depth 2)
(cd firmware; cargo test)
(cd common; cargo test)
(cd native-client; cargo hack check --feature-powerset --no-dev-deps)
(cd native-client; cargo test)
