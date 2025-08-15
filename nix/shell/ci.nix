{ pkgs }:
let
  usePreCommit = builtins.getEnv "IN_NIX_SHELL" == "impure" && builtins.getEnv "CI" != "1";
in
{
  buildInputs = with pkgs; [
    pre-commit
  ];

  shellHook = ''
    if [ "${toString usePreCommit}" = "1" ]; then
      echo "Installing CI pre-commit hooks..."
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
