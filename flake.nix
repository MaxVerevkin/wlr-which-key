{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    devenv.url = "github:cachix/devenv";
    devenv.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };
  outputs =
    inputs@{
      nixpkgs,
      systems,
      devenv,
      treefmt-nix,
      ...
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      packages = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "wlr-which-key";
            version = "1.3.0";

            src = ./.;

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            buildInputs = with pkgs; [
              cairo
              pango.dev
              libxkbcommon
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            meta = {
              description = "Keymap manager for wlroots-based compositors";
              homepage = "https://github.com/MaxVerevkin/wlr-which-key";
              license = pkgs.lib.licenses.gpl3Only;
            };
          };
        }
      );
      devShells = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              {
                languages.rust = {
                  enable = true;
                };
                packages = with pkgs; [
                  libudev-zero
                  pkg-config
                  cairo
                  pango.dev
                  libxkbcommon
                ];
                git-hooks.hooks = {
                  nixfmt.enable = true;
                  deadnix.enable = true;
                  statix.enable = true;
                  rustfmt.enable = true;
                };
              }
            ];
          };
        }
      );
      formatter = eachSystem (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        (treefmt-nix.lib.evalModule pkgs {
          projectRootFile = "flake.nix";
          programs = {
            nixfmt.enable = true;
            deadnix.enable = true;
            statix.enable = true;
            rustfmt.enable = true;
          };
        }).config.build.wrapper
      );
    };
}
