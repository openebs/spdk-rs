{ sources ? import ../sources.nix }:
let
  pkgs =
    import sources.nixpkgs { overlays = [ (import sources.rust-overlay) ]; };
  nightly_version = "2025-06-26";
  stable_version = "1.88.0";
in
with pkgs; rec  {
  nightly = rust-bin.nightly.${nightly_version}.default;
  stable = rust-bin.stable.${stable_version}.default;
  asan = rust-bin.nightly.${nightly_version}.default;
}
