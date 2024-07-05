# `spdk-rs` crate

`spdk-rs` crate provides a higher-level bindings and wrappers around
SPDK library to enable building safer SPDK-based Rust applications.

Read about SPDK here: [https://spdk.io/](https://spdk.io/).

About linking SPDK applications:
[https://spdk.io/doc/pkgconfig.html](https://spdk.io/doc/pkgconfig.html).

OpenEBS provides a version of SPDK with a number of patches needed by OpenEBS
storage products: https://github.com/openebs/spdk.
When a new version of SPDK is released, OpenEBS maintainers port these 
patches on the new branch.
For Mayastor family of products, the branches with OpenEBS patches are named
like `v24.05.x-mayastor` where `v24.05` is the tag name of a major SPDK release.

Please note that not every major SPDK release has a branch with OpenEBS patches.

## Development enviroment

For developing with `spdk-rs`, a Rust compiler and an SPDK library must be
present on the system. This `spdk-rs` repo contains a set of scripts 
that allows to set up this development environment easily.

`spdk-rs` leverages Nix package manager for that purpose: instead of 
relying on system's package manager, all the necessary tools and libraries 
are supplied by Nix.

In order to provide a stable and reproducible build environment,
`spdk-rs` pins the concrete versions of all the required tools and libraries.
This is done via a set of Nix package definitions.

Currently, `spdk-rs` can only be built on Linux.

### Nix package manager

In order to make use of this repo's developement scripts, a developer 
must have a Nix environment installed on the system. One can either install
a Nix packager manager on the existing operating system,
or install the Nix OS Linux distro which is built on top of 
the Nix packager manager.

Installing Nix as a packager manager: https://nixos.org/download/

Installing Nix as a Linux distro: https://nixos.org/download/#nixos-iso 

Once a Nix environment is ready, the developer can proceed and start
`spdk-rs` Ni x shell.

### Nix shell

To start Nix shell, run this command from `spdk-rs` directory:
```
nix-shell
```
Nix package manager will download and install all the necessary 
tool and libraries, including the Rust compiler and SPDK.
Then, it will start a bash session.

When started for the first time, it would take some time to download
and build the dependencies. It may take a considerable amount of time,
depending on Internet connection speed and CPU power.
Next time, Nix will use dependencies previously built and cached.

Nix installs different versions of the same package independently,
allowing to have multiple versions of a library on the same system 
in different shells.

**Note**: Nix package manager does not install anything on any the system-wide 
`bin` or `lib` directories, instead it creates a virtual environment for 
each shell instance.
All the required libraries and binaries are installed into 
Nix's own directory. Within the Nix shell, `PATH` and other variables 
like `LD_LIBRARY_PATH` are set up accordingly.

### Nix shell arguments

Nix shell for `spdk-rs` accepts some arguments that allow to alter
the development environment.
Nix shell arguments are passed by `--argstr <value>` shell argument, e.g.:

```
nix-shell --argstr spdk release --argstr rust nightly
```

The supported parameters are the following:
* `spdk <config>`:
  * `develop`: use debug Nix package for SPDK (this is the default);
  * `release`: use release Nix package for SPDK;
  * `none`: do not use Nix package for SPDK; see below.
* `spdk-path <path>`: sets path to SPDK source directory; see below.
* `rust <config>`
  * `stable`: use stable Rust Nix package (this is the default);
  * `nightly`: use nightly Rust Nix package;
  * `asan`: enable ASAN for Rust; nightly Nix package is forced;
  * `none`: do not use Nix package for SPDK; see below.

## Building `spdk-rs`

Once the Nix shell is started, `spdk-rs` can be built the usual Rust way:
```
cargo build
```

Example SPDK-based programs from this repo can be built and run with:
```
cargo build --example hello_world
sudo ./target/debug/examples/hello_world
```

**Note**: Generally, SPDK-based program are likely to require root priveleges.

## SPDK packages

This repo provides a Nix package definition for SPDK library, configured 
for `spdk-rs` needs.

`spdk-rs` is not designed to work with an arbitrary SPDK version, or arbitrary
SPDK configuration. Rather, a specific `spdk-rs` version is compatible with
a specific SPDK commit and configuration. This relationship is established 
by specifying SPDK commit hash in SPDK Nix package definition:
`nix/pkgs/libspdk/default.nix`. When Nix shell is started, it fetches
the version specified in that file (github commit hash).

Nix package definition for SPDK configures and compiles SPDK with options 
compatible with `spdk-rs`.

One can select either a debug SPDK package (no optimizations, 
assertions enabled), or a release package by starting Nix shell with
the `spdk` argument:
```
nix-shell --argstr spdk debug
or
nix-shell --argstr spdk release
```
By default, Nix shell starts with debug SPDK.

**Note**: Installing SPDK via system's package managers like `apt` is 
not supported. It may, or may not work.

## Custom SPDK

To add new features to, or fix bugs in SPDK itself, SPDK has to be built 
from source code. To do that, one has to start the Nix shell without
SPDK packages, pull and checkout the correct SPDK version, and build it
manually. `spdk-rs` provides helper shell scripts to simplify SPDK compilation.

SPDK must be built with the environment compatible with `spdk-rs`, and
with a compatible configuration. To set up the compatible environment, 
configure and compile SPDK from within the `spdk-rs` Nix shell.

### Getting SPDK source code

To get the SPDK source code, determine the branch and/or commit you need, and
run the following commands:
```
git clone git@github.com:openebs/spdk.git
cd spdk
git checkout -t origin/v24.01.x-mayastor
git submodule update --recursive --init
```

### Starting the compatible Nix shell

`spdk-rs` relies on `SPDK_ROOT_DIR` environment variable to point to
SPDK installation or source code directory.

To begin with custom SPDK compilation, start the shell with SPDK packages
disabled, and SPDK source path specified:
```
nix-shell --argstr spdk none --argstr spdk-path ~/myspdk
```

`--argstr spdk-path <path>` will set `SPDK_ROOT_DIR` to point to the given 
path. (Please note tha the path is resolved to the real path before assigning
the shell var).

**Note**: If `--argstr spdk-path <path>` is given, `--argstr spdk none` is
implied:
```
nix-shell --argstr spdk-path ~/myspdk
```

**Note**:  However, if `--argstr spdk none` is given, and
`--argstr spdk-path <path>` is not given, the developer must 
set `SPDK_ROOT_DIR` manually.

### SPDK helper scripts

`spdk-rs` provides a helper script to simplify configuring and compiling SPDK,
`./build_scripts/build_spdk.sh`. The script supports verbose (`-v`) and 
dry run (`-n`) modes. See script's help (`--help`) for more options.


To configure SPDK, run: 
```
./build_scripts/build_spdk.sh configure
```

To compile SPDK, run:
```
./build_scripts/build_spdk.sh make
```

To check and fix SPDK source code formatting according to SPDK rules, run:
```
./build_scripts/build_spdk.sh fmt
```

To clean the SPDK directory with git clean, run:
```
./build_scripts/build_spdk.sh clean
```

#### Rebuild Rust code when SPDK is rebuilt

When Nix shell is started with Nix SPDK packages disabled, Rust build script
for `spdk-rs` will tell Cargo to recompile Rust code every time
`build_spdk.sh make` is invoked.

(`./build_spdk.sh make` writes to `build_logs` directory.
Rust build script detects modifications of `build_logs` directory, and 
rebuilds Rust sources.)

### Building SPDK manually

Instead of relying on `build_spdk.sh` script, one can configure and make
SPDK manually:
```
cd <spdk_path>
AS=nasm ./configure --enable-debug --target-arch=nehalem --without-shared \
    --without-crypto \
    --without-nvme-cuse --without-fuse \
    --with-uring --without-uring-zns \
    --disable-unit-tests --disable-tests \
    --with-fio=$(realpath $(dirname $(which fio))/..)

make -j
```

**Note**: Currently, `spdk-rs` can only link to SPDK built as static libraries.

`spdk-rs` must be cleaned manually in order to rebuild Rust code after 
making SPDK.

#### Checking style

```
cd <spdk_dir>
./scripts/check_format.sh
```

#### Cleaning SPDK build

To clean up the SPDK directory and reset to the initial state:
```
mkdir ${your_spdk_dir}
make clean
git clean -fdx ./
git submodule foreach --recursive git clean -xfd
git submodule update
```

### Installing SPDK

Currently, SPDK's standard `make install` script does not make an installation
sutiable to be used with `spdk-rs`. That is, trying to configure (via `DESTDIR`)
and _install_ SPDK in other directory using `Makefile` is not supported.

To install, run:
```
./build_scripts/build_spdk.sh install <path>
```

## Rust compiler

To provide a reproducible build environment, Rust compiler is suppiled by Nix.
By default, a compatible, verified stable Rust is used when Nix shell starts.
One can choose a nightly Rust (although, it is a compatible, verified nightly):
```
nix-shell --argstr rust nightly
```

To use Rust provided by the system, e.g. by brough  up `rustup`, run:
```
nix-shell --argstr rust none
```

### ASAN

One can compile and run Rust code with LLVM Address Sanitizer (ASAN) enabled:

```
nix-shell --argstr rust asan
cargo build
```

**Note**: ASAN forces nightly Rust, and changes output directory.

### System Rust

TODO

## Appendix A: SPDK versions and configurations

Different versions of SPDK require different configuration option.

### Versions < 22.09

```
./configure --enable-debug --target-arch=nehalem --without-shared \
    --without-isal --with-crypto --with-uring --disable-unit-tests \
    --disable-tests --with-fio=$(which fio | sed s';bin/fio;include;')
make
```

### Versions >= 22.09 and < 24.01
* ISAL is now mandatory
* When upgrading from previous versions, clean the SPDK directory
  before building (e.g. git clean).
* An `AS` shell var must be set, either `yasm` or `nasm`.
* Zone support for uring must be disabled.
* Crypto must be disabled.

```
AS=yasm ./configure --enable-debug --target-arch=nehalem --without-shared \
    --without-crypto \
    --with-uring --without-uring-zns \
    --disable-unit-tests --disable-tests \
    --with-fio=$(realpath $(dirname $(which fio))/..)

make
```

### Version >= 24.01
Same as 22.09+ but with the following additionally:
* `AS` must be `nasm` now.
* FUSE must be disabled.

 ```
 AS=nasm ./configure --enable-debug --target-arch=nehalem --without-shared \
     --without-crypto \
     --without-nvme-cuse --without-fuse \
     --with-uring --without-uring-zns \
     --disable-unit-tests --disable-tests \
     --with-fio=$(realpath $(dirname $(which fio))/..)
 make
 ```
