{
  description = "Devcontainer Home Manager configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, home-manager, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };
      localModule = ./. + "/local.nix";
      localModules = if builtins.pathExists localModule then [ localModule ] else [ ];
    in
    {
      homeConfigurations.vscode = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;
        modules = [
          (
            { pkgs, ... }:
            {
              home.username = "vscode";
              home.homeDirectory = "/home/vscode";
              home.stateVersion = "23.11";

              programs.home-manager.enable = true;

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

                # Nix editor tooling
                nil
                nixfmt

                # Rust tooling
                rustup
                cargo-llvm-cov
                cargo-nextest
                cargo-deny
                cargo-machete
                cargo-binstall
              ];
            }
          )
        ]
        ++ localModules;
      };
    };
}
