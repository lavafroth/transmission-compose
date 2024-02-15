{ pkgs ? import <nixpkgs> {}} :
pkgs.rustPlatform.buildRustPackage {
  pname = "transmission-compose";
  version = "1.1.0";
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
}
