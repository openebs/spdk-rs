#!/usr/bin/env bash

set -o pipefail

OWNER="openebs"
REPO="spdk"
REV=""

silent_pushd "$SPDK_ROOT_DIR"

if [[ -z "$REV" ]]
then
    msg_debug "Detecting git HEAD revision in $(pwd) ..."
    if ! REV=$(git rev-parse HEAD 2> /dev/null)
    then
        msg_error "Cannot detect HEAD git revision hash"
        exit 1
    fi
fi

msg_info "Using HEAD git revision: $REV"

CMD="nix-prefetch-github --fetch-submodules --rev $REV $OWNER $REPO"

msg_debug "Command to prefetch: $CMD"
msg_info "Fetching from github ..."

# For low verbosity levels, suppress fetch info printed by nex-prefect.
OUT=$($CMD)
if [[ "$VERBOSE" -ge "2" ]]
then
    echo "$OUT"
fi
SHA=$(echo "$OUT" | grep '"hash"' | cut -d '"' -f4)

if [[ -z "$SHA" ]]
then
    msg_error "Failed to fetch revision $REV from github"
    exit 1
fi

echo
echo "SPDK version         : $SPDK_VERSION-${REV:0:7}"
echo "SPDK revision        : $REV"
echo "SPDK revision SHA256 : $SHA"
echo '''
Tip: copy these to default.nix of SPDK Nix package: to "drvAttrs.version",
and to "rev" and "sha256" arguments of "fetchFromGitHub" call respectively.
'''

silent_popd
