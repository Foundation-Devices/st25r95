# SPDX-FileCopyrightText: 2025 Foundation Devices, Inc. <hello@foundation.xyz>
# SPDX-License-Identifier: GPL-3.0-or-later
{
  self,
  system,
  pkgs,
  fenix,
}:
let
  toolchainSha256 = "sha256-18J/HvJzns0BwRNsUNkCSoIw7MtAmppPu5jNzuzMvCc=";

  baseToolchain = fenix.packages.${system}.fromToolchainFile {
    file = self + "/rust-toolchain.toml";
    sha256 = toolchainSha256;
  };

  armv7aStd = fenix.packages.${system}.targets.armv7a-none-eabi.fromToolchainFile {
    file = self + "/rust-toolchain.toml";
    sha256 = toolchainSha256;
  };

in
{
  rust-toolchain = fenix.packages.${system}.combine [
    baseToolchain
    armv7aStd
  ];
  rust-analyzer = fenix.packages.${system}.rust-analyzer;
}
