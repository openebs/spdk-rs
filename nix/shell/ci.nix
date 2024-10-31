{ pkgs }:
let
  nixShellPurity = builtins.getEnv "IN_NIX_SHELL";
in
{
  buildInputs = with pkgs; [
    pre-commit
  ];

  shellHook = ''
    if [ -z "$CI" ] && [ "${nixShellPurity}" == "impure" ]; then
      echo "Installing CI pre-commit hooks..."
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
