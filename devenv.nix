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

  languages.rust.enable = true;

  enterShell = ''
    git --version # Use packages
  '';
}
