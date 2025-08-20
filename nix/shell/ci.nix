{ pkgs }:
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
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
