#!/usr/bin/env bash

source ${BASH_SOURCE%/*}/rust-linter-env.sh
$CARGO clippy -p spdk-rs --all-targets -- -D warnings