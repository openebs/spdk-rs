#!/usr/bin/env bash

set -o pipefail

GIT_CLEAN_ARGS=(
    "--force"
    "-d"            # remove whole directories
    "-x"            # remove ignored files, too
)

if [[ "$DRY_RUN" == "yes" ]]
then
    GIT_CLEAN_ARGS+=("--dry-run")
fi

ARGS="${GIT_CLEAN_ARGS[*]}"

silent_pushd "$SPDK_ROOT_DIR"

msg_info "Running 'git clean $ARGS' for repo root ..."
git clean $ARGS
R=$?
if [[ "$R" -ne 0 ]]
then
    exit $R
fi

msg_info "Running 'git clean $ARGS' for each submodule ..."
git submodule foreach git clean $ARGS
R=$?
if [[ "$R" -ne 0 ]]
then
    exit $R
fi

silent_popd
