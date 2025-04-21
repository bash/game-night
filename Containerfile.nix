FROM docker.io/nixos/nix
WORKDIR /usr/local/src/game-night
RUN nix --extra-experimental-features "nix-command flakes" flake check --all-systems
