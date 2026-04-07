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
    targets = [ "thumbv7em-none-eabi" ];
  };

  enterShell = ''
    git --version # Use packages
  '';
}
