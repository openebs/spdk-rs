## Overview

These are a collection of packages we need, or packages where we
want to control the exact version(s) of.

The packages are imported through the `nix-shell` automatically. If you
run NixOS, read the following section.

## Configuration

## nix-shell

Build environment for spdk-rs including all test and debug dependencies.
It can be run with two arguments:

* `--arg nospdk true`: to use your own SPDK.
* `--arg spdk_rel true`: to use your release mode SPDK.
* `--arg norust true`: to use your own rust toolchain.
