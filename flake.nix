{
  description = "A custom Minecraft launcher for LiquidBounce, a popular utility mod, that features auto install & update and mod management.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        rec {
          liquidlauncher-unwrapped = pkgs.callPackage ./nix/liquidlauncher-unwrapped.nix { };
          liquidlauncher = pkgs.callPackage ./nix/liquidlauncher.nix { inherit liquidlauncher-unwrapped; };
          default = liquidlauncher;
        }
      );
    };
}
