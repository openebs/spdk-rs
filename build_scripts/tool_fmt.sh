#!/usr/bin/env bash

set -o pipefail

silent_pushd "$SPDK_ROOT_DIR"
./scripts/check_format.sh
R=$?
silent_popd
exit $R
