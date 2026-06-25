{
  description = "Devcontainer tool profile";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };
      devcontainerTools = pkgs.buildEnv {
        name = "devcontainer-tools";
        paths = with pkgs; [
          # Core CLI tools
          ripgrep
          file
          ast-grep

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
      };
    in
    {
      packages.${system} = {
        devcontainer-tools = devcontainerTools;
        default = devcontainerTools;
      };
    };
}
