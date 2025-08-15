{ sources
, pkgs
, rust ? "stable"
, spdk ? "develop"
, spdk-path ? null
, cfg
}:
let
  rustCfg = import ./rust.nix { inherit pkgs; inherit sources; inherit rust; };
  spdkCfg = import ./spdk.nix { inherit pkgs; inherit spdk; inherit spdk-path; };
  ciCfg = import ./ci.nix { inherit pkgs; };
in
rec {
  # fortify does not work with -O0 which is used by spdk when --enable-debug
  hardeningDisable = [ "fortify" ];

  buildInputs = with pkgs; [
    commitlint
    libunwind
  ]
  ++ rustCfg.buildInputs
  ++ spdkCfg.buildInputs
  ++ ciCfg.buildInputs
  ++ cfg.buildInputs;

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  shellHook = ''
    echo "Running shell hooks..."
  ''
  + ciCfg.shellHook
  + rustCfg.shellHook
  + rustCfg.shellInfoHook
  + spdkCfg.shellHook
  + spdkCfg.shellInfoHook
  + cfg.shellHook
  + cfg.shellInfoHook;
}
// rustCfg.shellEnv // spdkCfg.shellEnv // cfg.shellEnv
