# SPDX-FileCopyrightText: 2025 Foundation Devices, Inc. <hello@foundation.xyz>
# SPDX-License-Identifier: GPL-3.0-or-later

{
  description = "Rust development environment with local cargo dir";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      fenix,
    }:
    let
      inherit (nixpkgs) lib;

      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];

      forAllSystems = f: lib.genAttrs systems f;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          ci-pkgs = with pkgs; {
            inherit just cargo-sort;
          };
        in
        ci-pkgs
        // import ./nix/rust-toolchain.nix {
          inherit
            self
            system
            pkgs
            fenix
            ;
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          customPackages = self.packages.${system};

          buildPackages =
            with pkgs;
            [
              cargo-sort
              cargo-msrv
              git
            ]
            ++ (with customPackages; [
              rust-toolchain
            ]);

          devPackages =
            buildPackages
            ++ (with customPackages; [
              rust-analyzer
            ]);

          darwinPackages =
            let
              xcodeenv = import (nixpkgs + "/pkgs/development/mobile/xcodeenv") { inherit (pkgs) callPackage; };
            in
            lib.optionals pkgs.stdenv.isDarwin [
              (xcodeenv.composeXcodeWrapper { versions = [ "16.0" ]; })
            ];

          linuxPackages =
            with pkgs;
            lib.optionals stdenv.isLinux [
              clang
              # llvmPackages.libclang
            ];

          linuxAttrs = lib.optionalAttrs pkgs.stdenv.isLinux {
            # for bindgen in c++ libs
            # macos already has xcode clang
            # LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          };

          mkShell =
            packages:
            pkgs.mkShellNoCC (
              {
                strictDeps = true;
                packages = packages ++ linuxPackages ++ darwinPackages;
                hardeningDisable = [ "all" ];
                buildInputs = with pkgs; [
                ];

                LD_LIBRARY_PATH =
                  with pkgs;
                  lib.makeLibraryPath (
                    [
                    ]
                    ++ lib.optionals stdenv.isLinux [
                      # llvmPackages.libclang.lib
                    ]
                  );

                shellHook = ''
                  # darwin xcode
                  unset DEVELOPER_DIR
                  unset SDKROOT

                  # unset clang env variables
                  unset CC
                  unset CXX
                  unset AR
                  unset RANLIB
                  	  
                  export CARGO_HOME=$PWD/.cargo
                  export CARGO_NET_GIT_FETCH_WITH_CLI=true
                  export CARGO_INCREMENTAL=0
                  export PATH=$PATH:''${CARGO_HOME}/bin
                '';
              }
              // linuxAttrs
            );
        in
        {
          # full development shell
          default = mkShell devPackages;
          # minimal build shell
          build = mkShell buildPackages;
        }
      );
    };
}
