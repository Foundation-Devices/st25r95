{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

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
  };

  enterShell = ''
    git --version # Use packages
  '';
}
