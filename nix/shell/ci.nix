{ pkgs }:
{
  buildInputs = with pkgs; [
    pre-commit
  ];

  shellHook = ''
    if [ -z "$CI" ]; then
      echo "Installing CI pre-commit hooks..."
      pre-commit install
      pre-commit install --hook commit-msg
      echo
    fi
  '';
}
