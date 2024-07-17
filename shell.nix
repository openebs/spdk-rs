{ rust ? "stable"
, spdk ? "develop"
, spdk-path ? null
} @ args:
let
  sources = import ./nix/sources.nix;

  pkgs = import sources.nixpkgs {
    overlays = [
      (_: _: { inherit sources; })
      (import ./nix/overlay.nix { })
    ];
  };

  shellAttrs = import ./nix/shell (args // {
    inherit sources;
    inherit pkgs;

    cfg = {
      buildInputs = [ ];
      shellEnv = { };
      shellHook = "";
      shellInfoHook = "";
    };
  });
in
pkgs.mkShell shellAttrs // {
  name = "spdk-rs-dev-shell";
}
