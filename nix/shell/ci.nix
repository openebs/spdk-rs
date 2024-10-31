{ pkgs }:
{
  buildInputs = with pkgs; [
    pre-commit
  ];

  shellHook = ''
    if [ -z "$CI" ] && [ "$IN_NIX_SHELL" == "impure" ]; then
      echo "Installing CI pre-commit hooks..."
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
