{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs.buildPackages; [
    cairo
    glib
    libxkbcommon
    pango

    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
    rust-analyzer
  ];
}
