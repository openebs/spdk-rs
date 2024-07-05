{ pkgs
, spdk
, spdk-path ? null
}:
let
  fioDetectHook = ''
    export FIO="$(which fio 2>/dev/null)"
  '';

  spdkDrvInfo = ''
    echo "SPDK version    : $(echo $SPDK_ROOT_DIR | sed 's/.*libspdk-//g')"
  '';

  spdkPathsInfo = ''
    echo "SPDK path       : ''${SPDK_ROOT_DIR:-(SPDK_ROOT_DIR is undefined, set it manually)}"
    echo "SPDK FIO plugin : ''${FIO_SPDK:-(FIO_SPDK is undefined, set it manually)}"
    echo "FIO version     : $(fio --version 2> /dev/null)"
    echo "FIO path        : $FIO"
  '';

  # spdk-path argument overrides spdk argument.
  spdkCfg = if spdk-path != null then "none" else spdk;

  configurations = {
    # Use development build libspdk package.
    develop = rec {
      drv = pkgs.libspdk-dev;
      buildInputs = [ drv ] ++ drv.buildInputs;

      shellEnv = {
        SPDK_ROOT_DIR = drv;
        FIO_SPDK = "${drv}/fio/spdk_nvme";
      };

      shellHook = fioDetectHook;

      shellInfoHook = ''
        echo
        echo "SPDK derivation : develop (debug build)"
      ''
      + spdkDrvInfo
      + spdkPathsInfo;
    };

    # Use release build of libspdk package.
    release = rec {
      drv = pkgs.libspdk;
      buildInputs = [ drv ] ++ drv.buildInputs;

      shellEnv = {
        SPDK_ROOT_DIR = drv;
        FIO_SPDK = "${drv}/fio/spdk_nvme";
      };

      shellHook = fioDetectHook;

      shellInfoHook = ''
        echo
        echo "SPDK derivation : release"
      ''
      + spdkDrvInfo
      + spdkPathsInfo;
    };

    # Do not use Nix libspdk. User must provide SPDK.
    # Build environment for development libspdk packahe is provided.
    none = {
      drv = null;

      buildInputs = with pkgs; libspdk-dev.nativeBuildInputs ++ libspdk-dev.buildInputs;

      shellEnv = {
        CFLAGS = "-msse4";
        SPDK_RS_BUILD_USE_LOGS = "yes";  # Tells spdk-rs build.rs script to rerun when build_logs dir is updated.
      };

      shellHook = fioDetectHook
       + (if spdk-path == null then "" else ''
        export SPDK_ROOT_DIR=$(realpath ${spdk-path} 2>/dev/null);
        if [[ -n "$SPDK_ROOT_DIR" ]];
        then
          export FIO_SPDK="$SPDK_ROOT_DIR/build/fio/spdk_nvme";
        fi
      '')
      + ''
        echo
        echo "You have requested Nix shell without SPDK."
        echo "Use 'build_scripts/build_spdk.sh' to configure and compile SPDK."
      '';

      shellInfoHook = ''
        echo
        echo "SPDK derivation : none"
      ''
      + spdkPathsInfo;
    };
  };
in
  configurations.${spdkCfg}
