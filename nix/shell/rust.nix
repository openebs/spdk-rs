{ pkgs
, sources
, rust
}:
let
  rustChannels = import ../lib/rust.nix { inherit sources; };

  buildInputs = with pkgs; [
    cmake
    gcc
    llvmPackages.bintools
    llvmPackages.clang
    llvmPackages.libclang
    pkg-config
    procps
    utillinux
  ];

  shellEnv = {
    # Path to nightly Rust, needed for linter and style.
    RUST_NIGHTLY_PATH = rustChannels.nightly;

    # Path to debug output dir.
    RUST_TARGET_DEBUG = "target/debug";
  };

  rustShellInfo = ''
    echo "Rust version     : $(rustc --version 2> /dev/null)"
    echo "Rust path        : $(which rustc 2> /dev/null)"
    echo "Target debug dir : $RUST_TARGET_DEBUG"
  '';

  configurations = {
    # Stable Rust channel configuration.
    stable = {
      buildInputs = buildInputs ++ [
        rustChannels.stable
        rustChannels.nightly
      ];

      inherit shellEnv;

      shellHook = ''
      '';

      shellInfoHook = ''
        echo "Rust channel     : stable"
      ''
      + rustShellInfo;
    };

    # Nightly Rust channel configuration.
    nightly = {
      buildInputs = buildInputs ++ [
        rustChannels.nightly
      ];

      inherit shellEnv;

      shellHook = ''
      '';

      shellInfoHook = ''
        echo "Rust channel     : nightly (explicity selected)"
      ''
      + rustShellInfo;
    };

    # Rust configuration for ASAN enabled.
    # Rust's ASAN requires nightly Rust.
    asan = {
      buildInputs = buildInputs ++ [
        rustChannels.nightly
      ];

      shellEnv = shellEnv // {
        # ASAN-related Cargo settings.
        ASAN_ENABLE             = "1";
        ASAN_OPTIONS            = "detect_leaks=0";
        ASAN_BUILD_ENV          = "shell";
        RUSTFLAGS               = "-Zsanitizer=address";
        CARGO_BUILD_RUSTFLAGS   = "-Zbuild-std";
        CARGO_BUILD_TARGET      = "x86_64-unknown-linux-gnu";
        CARGO_PROFILE_DEV_PANIC = "unwind";
        RUST_BACKTRACE          = "full";
        RUST_TARGET_DEBUG       = "target/x86_64-unknown-linux-gnu/debug";
      };

      shellHook = ''
        export LLVM_SYMBOLIZER_DIR=$(dirname $(realpath $(which llvm-symbolizer)))
      '';

      shellInfoHook = ''
        echo "Rust channel     : nightly (forced by ASAN)"
      ''
      + rustShellInfo
      + ''
        echo
        echo "AddressSanitizer for Rust is enabled (nightly rustc forced)."
        echo "ASAN_ENABLE             : $ASAN_ENABLE"
        echo "ASAN_OPTIONS            : $ASAN_OPTIONS"
        echo "RUSTFLAGS               : $RUSTFLAGS"
        echo "CARGO_BUILD_RUSTFLAGS   : $CARGO_BUILD_RUSTFLAGS"
        echo "CARGO_BUILD_TARGET      : $CARGO_BUILD_TARGET"
        echo "CARGO_PROFILE_DEV_PANIC : $CARGO_PROFILE_DEV_PANIC"
        echo "RUST_BACKTRACE          : $RUST_BACKTRACE"
        echo "LLVM_SYMBOLIZER_DIR     : $LLVM_SYMBOLIZER_DIR"
      '';
    };

    # No Nix-provided Rust configuration.
    none = {
      inherit buildInputs;

      inherit shellEnv;

      shellHook = ''
        echo "You have requested Nix shell without Rust."
        echo "Use system rustup tool to configure Rust."
      '';

      shellInfoHook = ''
        echo
        echo "Rust channel     : none (system-wide Rust will be used)"
      ''
      + rustShellInfo;
    };
  };
in
  configurations.${rust}
