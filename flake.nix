{
  description = "dhtmsg: simple DHT hello PoC with cross builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type: builtins.baseNameOf path != "target";
        };
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "dhtmsg";
          version = "0.1.0";
          inherit src;
          cargoLock.lockFile = ./Cargo.lock;
          doCheck = false;
          meta.mainProgram = "dhtmsg";
        };

        # Cross-compiled Windows build (x86_64-pc-windows-gnu) with static CRT.
        packages.windows = let
          crossPkgs = import nixpkgs {
            localSystem = system;
            crossSystem = "x86_64-w64-mingw32";
          };
        in crossPkgs.rustPlatform.buildRustPackage {
          pname = "dhtmsg";
          version = "0.1.0";
          inherit src;
          cargoLock.lockFile = ./Cargo.lock;
          doCheck = false;
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
          cargoBuildFlags = [ "--release" "--target" "x86_64-pc-windows-gnu" ];
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-C target-feature=+crt-static";
          dontPatchELF = true;
          meta = {
            mainProgram = "dhtmsg";
            platforms = [ "x86_64-w64-mingw32" ];
          };
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            pkg-config
          ];
        };
      });
}
