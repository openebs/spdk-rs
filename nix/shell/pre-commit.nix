{ pkgs, src-root }:
let
  usePreCommit = builtins.getEnv "IN_NIX_SHELL" == "impure" && builtins.getEnv "CI" != "1";
  pre-commit = pkgs.runCommand "pre-commit" { } ''
    mkdir -p $out/bin
    cp ${pkgs.pre-commit}/bin/pre-commit $out/bin/pre-commit
  '';
in
{
  buildInputs = pkgs.lib.optional (usePreCommit) pre-commit;

  shellHook = ''
    if [ "${toString usePreCommit}" = "1" ]; then
      echo "Installing CI pre-commit hooks..."
      # Into the tree this shell belongs to, which is not always the one entered from.
      (
        cd "${toString src-root}"
        pre-commit install
        pre-commit install --hook commit-msg
      )
      echo
    fi
  '';
}
