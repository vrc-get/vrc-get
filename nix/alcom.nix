{
  lib,
  rustPlatform,
  importNpmLock,
  nodejs,
  wrapGAppsHook4,
  pkg-config,
  gobject-introspection,
  cargo-tauri,
  webkitgtk_4_1,
  cairo,
  gdk-pixbuf,
  glib,
  gtk3,
  librsvg,
  libsoup_3,
  openssl,
  stdenv,
  darwin,
  src,
  ...
}:

rustPlatform.buildRustPackage {
  pname = "alcom";
  version = (builtins.fromTOML (builtins.readFile ../vrc-get-gui/Cargo.toml)).package.version;

  inherit src;
  
  env = {
    TAURI_CLI_NO_WATCH=true;
  };

  dontStrip = true;

  cargoLock.lockFile = "${src}/Cargo.lock";

  npmRoot = "vrc-get-gui";
  npmDeps = importNpmLock {
      npmRoot = ../vrc-get-gui/.;
  };

  nativeBuildInputs = [
    cargo-tauri.hook
    nodejs
    importNpmLock.npmConfigHook
    gobject-introspection
    pkg-config
  ]
  ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook4 ];

  buildInputs = [
    cairo
    gdk-pixbuf
    glib
    openssl
    gtk3
    librsvg
    libsoup_3
  ]
  ++ lib.optionals stdenv.isLinux [
    webkitgtk_4_1
  ]
  ++ lib.optionals stdenv.isDarwin (
    with darwin.apple_sdk.frameworks;
    [
      AppKit
      WebKit
      CoreGraphics
      SystemConfiguration
      CoreServices
      Security
      Foundation
      ApplicationServices
    ]
  );

  preBuild = ''
    cd vrc-get-gui

    npm run build

    cd ..
  '';

  postPatch = ''
    substituteInPlace vrc-get-gui/Tauri.toml \
        --replace 'createUpdaterArtifacts = "v1Compatible"' 'createUpdaterArtifacts = false'

    substituteInPlace vrc-get-gui/Tauri.toml \
        --replace 'beforeBuildCommand = "npm run build"' 'beforeBuildCommand = ""'
  '';

  cargoBuildFlags = [
    "--package"
    "vrc-get-gui"
  ];

  meta = {
    description = "A fast open-source alternative of VRChat Creator Companion";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
