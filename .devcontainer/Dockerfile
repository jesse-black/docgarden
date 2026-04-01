FROM mcr.microsoft.com/devcontainers/base:trixie

USER vscode

RUN curl -fsSL -o /tmp/nix-install "https://releases.nixos.org/nix/nix-2.34.3/install" \
    && echo "15af988e88ffc16d5d4abd6e1d100eba8b0dff6a41c6e03a7718c2b4a87e0cd6  /tmp/nix-install" | sha256sum -c - \
    && sh /tmp/nix-install --no-daemon \
    && rm -f /tmp/nix-install