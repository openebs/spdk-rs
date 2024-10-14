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
  spdk = rec {
    rev = "50b064f553970b0f352691530e80f19b8432f034";
    sha256 = "sha256-fim71qqNjGtITeXfR7kWIRpBbI2iF47D0suny3mjcCQ=";
    pname = "libspdk${nameSuffix}";
    version = "24.05-${lib.substring 0 7 rev}";
    name = "${pname}-${version}";
  };
  drvAttrs = rec {
    pname = spdk.pname;
    version = spdk.version;

    src = [
      (fetchFromGitHub {
        # Note that this would only rebuild if the first 7 chars differ, but in practice should be fine
        name = spdk.name;
        owner = "openebs";
        repo = "spdk";
        rev = spdk.rev;
        sha256 = spdk.sha256;
        fetchSubmodules = true;
      })
      ../../../build_scripts
    ];

    sourceRoot = spdk.name;

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
