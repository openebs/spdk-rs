# `spdk-rs` crate

`spdk-rs` crate provides a higher-level bindings and wrappers around
SPDK library to enable building safer SPDK-based Rust applications.

## Getting SPDK library

Read about SPDK here: [https://spdk.io/](https://spdk.io/).

`spdk-rs` crate requires SPDK library to exist on the system. `spdk-rs` supports
both linking to an SPDK library installed system-wide, or to SPDK libraries
and headers located in the SPDK build directory.

`spdk-rs` uses `SPDK_PATH` environment variable to locate SPDK. If it is not
set, `spdk-rs` tries to find SPDK in `spdk-rs/spdk` directory.

In order to get SPDK on the system, one can either use the Nix scripts
provided by the shell, or build SPDK manually. Installing SPDK via system's
package managers like `apt` is not fully supported. It may, or may not work.

Currently, `spdk-rs` can only link to SPDK built as static libraries.

About linking SPDK applications:
[https://spdk.io/doc/pkgconfig.html](https://spdk.io/doc/pkgconfig.html).

## Using Nix scripts provided by

### Mayastor
```
cd $mayastor-repo
nix-shell
```

### SPDK-RS

You may instead use the Nix shell provided by this repo:
```
nix-shell
```

When you do it for the first time, it would require some time to Nix to
build SPDK and all required dependencies, and configure the environment to use
them.

Now, build `spdk-rs` with Cargo:
```
cd spdk-rs
cargo build
```


However please note this may get out of sync as it's not the way we usually do it.
We'll gladly accept PR's to sync up the nix build :)

## Building SPDK manually

When making changes (or debugging) SPDK, it is often more convenient to have
a local checkout of SPDK rather than dealing with packages.

In order to have all other dependencies (except SPDK) properly installed,
start the Nix shell with SPDK package disabled:
```
nix-shell --arg nospdk true
```
Alternatively, it is possible to install all the dependencies required
by SPDK manually. These steps are not covered in this manual.

Now, clone and checkout a supported version of SPDK:
```
mkdir ${your_spdk_dir}
cd ${your_spdk_dir}
git checkout vYY.mm.x-mayastor
git submodule update --init --recursive
```

### Configure and build it:

#### Version < 22.09
```
./configure --enable-debug --target-arch=nehalem --without-shared \
    --without-isal --with-crypto --with-uring --disable-unit-tests \
    --disable-tests --with-fio=$(which fio | sed s';bin/fio;include;')
make
```

#### Version >= 22.09
* ISAL is now mandatory
* When upgrading from previous versions, clean the SPDK directory
  before building (e.g. git clean).
* An `AS` is now required, either `yasm` or `nasm`.
* Crypto must be disabled.

```
AS=yasm ./configure --enable-debug --target-arch=nehalem --without-shared \
    --without-crypto \
    --with-uring \
    --disable-unit-tests --disable-tests \
    --with-fio=$(realpath $(dirname $(which fio))/..)

make
```

---
**NOTE**

Currently, SPDK's `make install` script does not make an installation
sutiable to be used with `spdk-rs`. That is, trying to configure (via `DESTDIR`)
and _install_ SPDK in other directory using `Makefile` is not supported.
___


After build, SPDK can be used to build `spdk-rs`. Either set `SPDK_PATH`,
or create a symlink in `spdk-rs` directory to point to your SPDK.

```
export SPDK_PATH=${your_spdk_dir}
```
or
```
cd ${workspace}/spdk-rs
ln -s ${your_spdk_dir} ./spdk
```

Alternatively, one can clone SPDK inside `spdk-rs` and use
the proivided `build_spdk.sh` script to build it.

### Cleaning SPDK build
To clean up the SPDK directory and reset to the initial state:
```
mkdir ${your_spdk_dir}
make clean
git clean -fdx ./
git submodule foreach --recursive git clean -xfd
git submodule update
```

### Revert to Nix SPDK
To go back to SPDK library installed by Nix, exit the `nospdk` Nix shell and
start it again with:
```
nix-shell
```

`SPDK_PATH` variable takes precedence over SPDK found in `spdk-rs/spdk`,
so no need to remove or rename it.
