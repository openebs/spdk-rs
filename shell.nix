{ nospdk ? false, spdk_rel ? false }:
let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {
    overlays =
      [ (_: _: { inherit sources; }) (import ./nix/overlay.nix { }) ];
  };
in
with pkgs;
let
  nospdk_moth =
    "You have requested environment without SPDK, you should provide it!";
  norustc_msg = "no rustc, use rustup tool to install it";
  channel = import ./nix/lib/rust.nix { inherit sources; };
  spdk = if (!spdk_rel) then libspdk-dev else libspdk;
in
mkShell {
  name = "spdk-rs-dev-shell";
  # fortify does not work with -O0 which is used by spdk when --enable-debug
  hardeningDisable = [ "fortify" ];
  buildInputs = [
    autoconf
    automake
    clang
    cowsay
    libaio
    libbsd
    libnvme
    libpcap
    libunwind
    liburing
    llvmPackages.bintools
    llvmPackages.libclang
    meson
    ninja
    openssl
    pkg-config
    pre-commit
    procps
    udev
    utillinux
    yasm
    commitlint
  ] ++ (if (nospdk) then [ spdk.buildInputs ] else [ spdk ]);

  NODE_PATH = "${nodePackages."@commitlint/config-conventional"}/lib/node_modules";

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
  SPDK_PATH = if nospdk then null else "${spdk}";
  FIO_SPDK = if nospdk then null else "${spdk}/fio/spdk_nvme";

  shellHook = ''
    ${pkgs.lib.optionalString (!nospdk) "echo 'SPDK version    :' $(echo $SPDK_PATH | sed 's/.*libspdk-//g')"}
    ${pkgs.lib.optionalString (!nospdk) "echo 'SPDK path       :' $SPDK_PATH"}
    ${pkgs.lib.optionalString (!nospdk) "echo 'SPDK FIO plugin :' $FIO_SPDK"}
    echo 'Rust version    :' $(rustc --version 2> /dev/null || echo '${norustc_msg}')
    echo 'Rust path       :' $(which rustc 2> /dev/null || echo '${norustc_msg}')
    ${pkgs.lib.optionalString (nospdk) "cowsay ${nospdk_moth}"}
    ${pkgs.lib.optionalString (nospdk) "export CFLAGS=-msse4"}
    ${pkgs.lib.optionalString (nospdk) "echo"}

    if [ -z "$CI" ]; then
      echo
      pre-commit install
      pre-commit install --hook commit-msg
    fi
  '';
}
