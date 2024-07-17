{ pkgs }:
let
  inPureNixShell = builtins.getEnv "IN_NIX_SHELL" == "pure";
in
{
  buildInputs = with pkgs; [
    pre-commit
  ];

  shellHook = ''
    if [ -z "$CI" ] && [ "${toString inPureNixShell}" == "0" ]; then
      echo "Installing CI pre-commit hooks..."
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
