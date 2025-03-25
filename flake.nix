{
  description = "Keymap manager for wlroots-based compositors";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      systems,
      ...
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs (import systems);
    in
    {
      packages = eachSystem (system: rec {
        default = wlr-which-key;
        wlr-which-key =
          let
            pkgs = nixpkgs.legacyPackages.${system};
            craneLib = crane.mkLib pkgs;
            inherit (pkgs) lib;
          in
          craneLib.buildPackage {
            src = craneLib.cleanCargoSource ./.;
            nativeBuildInputs = with pkgs; [
              pkg-config
              glib
              pango
              libxkbcommon
            ];
            meta = {
              description = "Keymap manager for wlroots-based compositors";
              mainProgram = "wlr-which-key";
              homepage = "https://github.com/MaxVerevkin/wlr-which-key";
              license = lib.licenses.gpl3Only;
              platforms = lib.platforms.linux;
            };
          };
      });

      homeManagerModules = rec {
        default = wlr-which-key;
        wlr-which-key =
          {
            config,
            pkgs,
            lib,
            options,
            ...
          }:
          let
            cfg = config.programs.wlr-which-key;
            opt = options.programs.wlr-which-key;
            inherit (lib)
              types
              mkOption
              mkEnableOption
              literalExample
              ;
          in
          {
            options.programs.wlr-which-key = {
              enable = mkEnableOption "Enable wlr-which-key";
              package = mkOption {
                description = "The wlr-which-key package to use";
                type = types.package;
                default = self.packages.${pkgs.system}.wlr-which-key;
                example = literalExample "pkgs.wlr-which-key";
              };
              settings = mkOption {
                description = ''
                  The configuration for wlr-which-key, to be placed at $XDG_CONFIG_HOME/wlr-which-key/config.yaml
                  If it is a set, config.yaml is generated with pkgs.formats.yaml.
                  If it is a string, config.yaml is the string verbatim.
                '';
                default = { };
                type = types.either (types.attrsOf types.anything) types.str;
              };
            };
            config = lib.mkIf cfg.enable {
              home.packages = [ cfg.package ];

              xdg.configFile."wlr-which-key/config.yaml" =
                let
                  types = opt.settings.type.nestedTypes;
                  inherit (cfg) settings;
                in
                {
                  source = lib.mkIf (types.left.check settings) (
                    (pkgs.formats.yaml { }).generate "wlr-which-key-config.yaml" settings
                  );
                  text = lib.mkIf (types.right.check settings) settings;
                };
            };
          };
      };

      devShells = eachSystem (system: rec {
        default = wlr-which-key;
        wlr-which-key =
          let
            pkgs = nixpkgs.legacyPackages.${system};
            craneLib = crane.mkLib pkgs;
          in
          craneLib.devShell {
            inherit (self.packages.wlr-which-key) nativeBuildInputs;
          };
      });
    };
}
