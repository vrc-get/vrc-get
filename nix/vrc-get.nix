{ lib, rustPlatform, src, ... }:

rustPlatform.buildRustPackage {
    pname = "vrc-get";
    version = "1.9.2-beta0";

    inherit src;

    cargoLock.lockFile = "${src}/Cargo.lock";

    cargoBuildFlags = [ "--package" "vrc-get" ];
    cargoTestFlags = [ "--package" "vrc-get" ];

    meta = {
        description = "Open Source command line client of VRChat Package Manager, the main feature of VRChat Creator Companion (VCC), which supports Windows, Linux, and macOS.";
        license = lib.licenses.mit;
        platforms = lib.platforms.all;
    };
}
