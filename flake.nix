{
  description = "Camper - A simple Bandcamp music player client";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          wrapGAppsHook4
        ];

        buildInputs = with pkgs; [
          gtk4
          libadwaita
          glib
          cairo
          pango
          gdk-pixbuf
          graphene
          webkitgtk_6_0
          libsoup_3
          glib-networking
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
          gst_all_1.gst-plugins-ugly
        ];
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "camper";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          postInstall = ''
            install -Dm644 camper.png $out/share/icons/hicolor/512x512/apps/camper.png
            install -Dm644 ${builtins.toFile "camper.desktop" ''
              [Desktop Entry]
              Name=Camper
              Comment=A simple Bandcamp music player client
              Exec=camper
              Icon=camper
              Terminal=false
              Type=Application
              Categories=Audio;Music;Player;
            ''} $out/share/applications/camper.desktop
          '';

          meta = with pkgs.lib; {
            description = "A simple Bandcamp music player client";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
          '';
        };
      }
    );
}
