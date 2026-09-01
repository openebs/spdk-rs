{ sources ? import ../sources.nix }:
let
  pkgs =
    import sources.nixpkgs { overlays = [ (import sources.rust-overlay) ]; };
  nightly_version = "2026-07-16";
  stable_version = "1.97.1";
in
with pkgs; rec  {
  nightly = rust-bin.nightly.${nightly_version}.default;
  stable = rust-bin.stable.${stable_version}.default;
  asan = rust-bin.nightly.${nightly_version}.default;
}
