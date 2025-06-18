{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs.buildPackages; [
    openssl
    pkg-config

    # rust vmm uses latest stable and oxalica tend to lag behind.break
    # so we temporary force use of beta.

    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
    rust-analyzer
  ];
}
