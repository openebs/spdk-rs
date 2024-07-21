{ pkgs

, build-type
, with-fio
, multi-outputs ? false

, targetPlatform
, buildPlatform

, buildPackages
, fetchFromGitHub
, lib
, stdenv

, autoconf
, automake
, binutils
, cmake
, cunit
, fio
, gcc
, help2man
, jansson
, lcov
, libaio
, libbpf
, libbsd
, libelf
, libexecinfo
, libpcap
, libtool
, liburing
, libuuid
, llvmPackages
, meson
, nasm
, ncurses
, ninja
, numactl
, openssl
, pkg-config
, procps
, python3
, udev
, utillinux
, zlib
, rdma-core
}:
let
  # Suffix for debug build name.
  nameSuffix = if build-type == "debug" then "-dev" else "";

  # Select FIO plugin output: either to the libspdk pkg's out, or
  # to a separate libspdk-fio pkg's out.
  fioOutput = if multi-outputs then "fio" else "out";

  # Additional build inputs for debug build.
  extraBuildInputs = if build-type == "debug" then [ cunit lcov ] else [ ];

  # Build script path.
  buildScript = "../build_scripts/build_spdk.sh";

  # Common arguments for the build script.
  commonArgs =
    let
      fioArg =
        if with-fio then
          "--with-fio ${fio.dev}/include"
        else
          "--without-fio";

      crossPrefix =
        if targetPlatform.config != buildPlatform.config then
          "--crossPrefix=${targetPlatform.config}"
        else
          "";
    in
    "-v --no-log --with-spdk . -b ${build-type} -t ${targetPlatform.config} ${fioArg} ${crossPrefix}";

  # Arguments for the install phase.
  installArgs = if multi-outputs then "--with-fio-dst $fio" else "";

  #
  # Derivation attributes
  #
  drvAttrs = rec {
    pname = "libspdk${nameSuffix}";
    version = "24.01-c8e02b0";

    src = [
      (fetchFromGitHub {
        name = pname;
        owner = "openebs";
        repo = "spdk";
        rev = "c8e02b0259cc32494c305ef75c63f5a9614976b2";
        sha256 = "sha256-/YI1NBZUqC2r2X7fdJtbeuEEnt+uaHUAsEBkHZIBppA=";
        fetchSubmodules = true;
      })
      ../../../build_scripts
    ];

    sourceRoot = pname;

    nativeBuildInputs = [
      cmake
      gcc
      help2man
      llvmPackages.bintools
      llvmPackages.clang
      llvmPackages.libclang
      meson
      ninja
      pkg-config
      procps
      python3
      udev
      utillinux
    ] ++ extraBuildInputs;

    buildInputs = [
      autoconf
      automake
      binutils
      jansson
      fio
      libaio
      libbpf
      libbsd
      libelf
      libexecinfo
      libpcap
      libtool
      liburing
      libuuid
      nasm
      ncurses
      numactl
      openssl
      rdma-core
      (python3.withPackages (ps: with ps; [ pyelftools ]))
      zlib
    ] ++ extraBuildInputs;

    outputs = [ "out" ] ++ lib.optional (fioOutput != "out") fioOutput;

    dontStrip = build-type == "debug";
    enableParallelBuilding = true;
    hardeningDisable = [ "all" ];

    #
    # Phases.
    #
    prePatch = ''
      pushd ..
      chmod -R u+w build_scripts
      patchShebangs . > /dev/null
      popd
    '';

    configurePhase = ''
      ${buildScript} configure ${commonArgs}
    '';

    buildPhase = ''
      ${buildScript} make ${commonArgs}
    '';

    installPhase = ''
      ${buildScript} install $out ${commonArgs} ${installArgs}
    '';
  };
in
llvmPackages.stdenv.mkDerivation drvAttrs
