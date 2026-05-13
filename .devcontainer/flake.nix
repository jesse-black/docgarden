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

      rustToolchain = pkgs.rust-bin.stable."1.95.0".default.override {
        extensions = [
          "clippy"
          "llvm-tools-preview"
          "rust-src"
          "rustfmt"
        ];
      };
    in {
      homeConfigurations.vscode = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;
        modules = [
          ./home.nix
          ({ pkgs, ... }: {
            home.packages = with pkgs; [
              # Core CLI tools
              yq-go
              ripgrep
              fd
              eza
              gh
              file
              python3
              ast-grep
              bubblewrap

              # Shell/script tooling
              shellcheck
              shfmt

              # Rust tooling
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
