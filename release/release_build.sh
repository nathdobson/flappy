#!/usr/bin/env bash
set -e
set -u

(cd firmware; ./build.sh)
#(cd models; ./build.sh)
(cd native-client; ./build.sh)
(cd web-client; ./build.sh)
