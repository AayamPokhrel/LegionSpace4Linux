{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.hardware.lenovo-legion;
  defaultPkg = pkgs.callPackage ./package.nix { };
in
{
  options.hardware.lenovo-legion = {
    enable = mkEnableOption "Lenovo Legion hardware control driver and CLI utility";

    package = mkOption {
      type = types.package;
      default = defaultPkg;
      defaultText = literalExpression "pkgs.legion-cli";
      description = "The legion-cli package to install.";
    };
  };

  config = mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    services.udev.packages = [ cfg.package ];

    # Ensure required kernel modules for Legion WMI & EC interfaces are loaded
    boot.kernelModules = [
      "lenovo_wmi_gamezone"
      "lenovo_wmi_other"
      "ideapad_laptop"
      "wmi"
    ];
  };
}

