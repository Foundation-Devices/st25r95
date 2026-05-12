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
    channel = "nightly";
    version = "2025-06-24";
    targets = [ "thumbv7em-none-eabi" ];
  };

  enterShell = ''
    git --version # Use packages
  '';
}
