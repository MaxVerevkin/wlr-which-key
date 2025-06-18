{
  pkgs ? import <nixpkgs> {},
  lib,
  ...
}:
pkgs.rustPlatform.buildRustPackage rec {
  pname = "wlr-which-key";
  version = (builtins.fromTOML (lib.readFile ./${pname}/Cargo.toml)).package.version;

  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  # disable tests
  checkType = "debug";
  doCheck = false;

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  buildInputs = with pkgs; [
    cairo
    glib
    libxkbcommon
    pango

    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
  ];
}
