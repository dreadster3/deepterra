{
  description = "Description for the project";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, fenix, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        # To import a flake module
        # 1. Add foo to inputs
        # 2. Add foo as a parameter to the outputs function
        # 3. Add here: foo.flakeModule

      ];
      systems =
        [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      perSystem = { self', pkgs, ... }:
        let
          version = self.rev or self.dirtyRev;
          toolchain = with fenix.packages.${pkgs.system};
            combine [
              stable.cargo
              stable.rustc
              targets.x86_64-unknown-linux-gnu.stable.rust-std
              targets.x86_64-apple-darwin.stable.rust-std
              targets.x86_64-pc-windows-gnu.stable.rust-std
              targets.aarch64-unknown-linux-gnu.stable.rust-std
              targets.aarch64-apple-darwin.stable.rust-std
            ];
        in {
          # Per-system attributes can be defined here. The self' and inputs'
          # module parameters provide easy access to attributes of the same
          # system.

          # Equivalent to  inputs'.nixpkgs.legacyPackages.hello;
          packages.default = (pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          }).buildRustPackage {
            inherit version;
            pname = "deepterra";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = with pkgs.lib; {
              description =
                "A tool to parse terraform and generate a resource dependency graph";
              homepage = "https://github.com/dreadster/deepterra";
              license = licenses.asl20;
            };
          };

          # Dev shells
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              toolchain
              rustfmt
              clippy
              rustup
              zig
              goreleaser
              pkgsCross.mingwW64.buildPackages.binutils
            ];
          };
        };
      flake = {
        # The usual flake attributes can be defined here, including system-
        # agnostic ones like nixosModule and system-enumerating ones, although
        # those are more easily expressed in perSystem.

      };
    };
}

