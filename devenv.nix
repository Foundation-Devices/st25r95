{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

let
  # Same input, and the same resolution, as `languages.rust` uses internally.
  rust-overlay = config.lib.getInput {
    name = "rust-overlay";
    url = "github:oxalica/rust-overlay";
    attribute = "languages.rust.channel";
    follows = [ "nixpkgs" ];
  };
  rustBin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;

  # rustfmt.toml enables unstable options, so formatting only behaves as CI
  # expects on nightly. Stable rustfmt warns that it cannot set them and then
  # formats with the defaults, which silently disagrees with the `fmt` job.
  # Keep this date in sync with .github/workflows/rust.yml.
  rustfmtNightly = rustBin.nightly."2025-06-24".rustfmt;
in

{
  packages = with pkgs; [
    cargo-sort
    cargo-msrv
    git
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
    version = "1.87.0";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
    ];
    targets = [ "thumbv7em-none-eabi" ];

    # Everything builds and tests on stable; only rustfmt comes from nightly,
    # so `cargo fmt`, a bare `rustfmt` and the editor all format the way CI
    # checks.
    toolchain.rustfmt = rustfmtNightly;
  };

  enterShell = ''
    git --version # Use packages
  '';
}
