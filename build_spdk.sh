#!/bin/env bash
# This script replicates the build of SPDK as its done by nix without setting
# any specifics in terms of CPU. The purpose is to easily make changes to libspdk
# locally and then recompile it and test it with mayastor.
#

pushd spdk || { echo "Can not find spdk directory"; exit; }
[ ! -d dpdk/.git ] || { echo "Submodules not checked out?"; exit; }

./configure --enable-debug \
	--target-arch=nehalem \
	--without-isal \
	--with-crypto \
	--with-uring \
	--disable-unit-tests \
	--disable-tests \
	--with-fio=$(which fio | sed s';bin/fio;include;')

make -j $(nproc)

popd || exit
