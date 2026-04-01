{
  description = "Devcontainer Home Manager configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, home-manager, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };
      localModule = ./. + "/local.nix";
      localModules =
        if builtins.pathExists localModule
        then [ localModule ]
        else [ ];
        
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "llvm-tools-preview" ];
      };
    in {
      homeConfigurations.vscode = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;
        modules = [
          ./home.nix
          ({ pkgs, ... }: {
            home.packages = with pkgs; [
              rustToolchain
              cargo-llvm-cov
              cargo-deny
              cargo-machete
              cargo-binstall
            ];
          })
        ] ++ localModules;
      };
    };
}