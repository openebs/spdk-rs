{ sources
, pkgs
, src-root
, rust ? "stable"
, spdk ? "develop"
, spdk-path ? null
, cfg
}:
let
  rustCfg = import ./rust.nix { inherit pkgs; inherit sources; inherit rust; };
  spdkCfg = import ./spdk.nix { inherit pkgs; inherit spdk; inherit spdk-path; };
  preCommitCfg = import ./pre-commit.nix { inherit pkgs src-root; };
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
  ++ preCommitCfg.buildInputs
  ++ cfg.buildInputs;

  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";

  shellHook = ''
    echo "Running shell hooks..."
  ''
  + preCommitCfg.shellHook
  + rustCfg.shellHook
  + rustCfg.shellInfoHook
  + spdkCfg.shellHook
  + spdkCfg.shellInfoHook
  + cfg.shellHook
  + cfg.shellInfoHook;
}
// rustCfg.shellEnv // spdkCfg.shellEnv // cfg.shellEnv
