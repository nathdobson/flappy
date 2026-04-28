#!/usr/bin/env bash
set -e
set -u
(cd firmware; cargo hack check --feature-powerset --no-dev-deps --depth 2 -p firmware)
(cd firmware; cargo test -p firmware)
(cd common; cargo test)
(cd native-client; cargo hack check --feature-powerset --no-dev-deps)
(cd native-client; cargo test)
