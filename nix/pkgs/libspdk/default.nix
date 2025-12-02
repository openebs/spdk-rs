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
, keyutils
, astyle
}:
let
  nix-prefetch-github = pkgs.runCommand "nix-prefetch-github" { } ''
    mkdir -p $out/bin
    cp ${pkgs.nix-prefetch-github}/bin/nix-prefetch-github $out/bin/nix-prefetch-github
  '';

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

      # Only set crossPrefix if we're actually cross-compiling
      # (which we aren't, but let's keep the logic).
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
    rev = "fd233103d1a24e18a275159e68922e3d1289c28c";
    sha256 = "sha256-ebr1C2r/CeNRU9RAIhBN6rdom7LFXkzIvs7l/GyGG70=";
    pname = "libspdk${nameSuffix}";
    version = "25.05-${lib.substring 0 7 rev}";
    name = "${pname}-${version}";
  };

  drvAttrs = rec {
    pname = spdk.pname;
    version = spdk.version;
    rev = spdk.rev;

    src = [
      (fetchFromGitHub {
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

    devBuildInputs = [ astyle pkgs.python3Packages.tabulate pkgs.python3Packages.jinja2 nix-prefetch-github ];

    nativeBuildInputs = [
      cmake
      gcc
      help2man
      meson
      ninja
      pkg-config
      procps
      udev
      utillinux
      pkgs.python3Packages.pyelftools
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
      keyutils
      zlib
    ] ++ extraBuildInputs;

    propagatedBuildInputs = [
      python3.pkgs.configshell
    ];

    outputs = [ "out" ] ++ lib.optional (fioOutput != "out") fioOutput;

    dontStrip = build-type == "debug";
    enableParallelBuilding = true;
    hardeningDisable = [ "all" ];
    dontCheckForBrokenSymlinks = true;

    # Our phases
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
pkgs.stdenv.mkDerivation drvAttrs
