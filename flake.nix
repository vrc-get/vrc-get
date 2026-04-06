{
    description = "Fast Open Source command line (vrc-get) and graphical (ALCOM) client of VRChat Package Manager (VRChat Creator Companion) ";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, flake-utils }:
        flake-utils.lib.eachDefaultSystem (
            system:
            let 
                pkgs = import nixpkgs { system = system; };
            in
            {
                packages = {
                    vrc-get = pkgs.callPackage ./nix/vrc-get.nix { src = self; };
                    alcom = pkgs.callPackage ./nix/alcom.nix { src = self; };
                    default = self.packages.${system}.vrc-get;
                };
            }
        );
}
