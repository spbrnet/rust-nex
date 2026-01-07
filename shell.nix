{
  pkgs ? import <nixpkgs> { },
  pkg ? pkgs.callPackage ./. { },
}:
pkgs.mkShell {
  # Get dependencies from the main package
  # inputsFrom = [ pkg ];
  # Additional tooling
  buildInputs = with pkgs; [
    cargo
    rust-analyzer # LSP Server
    rustfmt # Formatter
    clippy # Linter
    podman
    yq
    jq
  ];
}
