inputs: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit
    (lib)
    mkOption
    mkEnableOption
    mkPackageOption
    hasPrefix
    optionalAttrs
    optionalString
    types
    mkIf
    ;
  cfg = config.services.bipper;
in {
  options.services.bipper = {
    enable = mkEnableOption "bipper";
    package = mkPackageOption pkgs "bipper" {};
    address = mkOption {
      description = ''
        Address to listen on
      '';
      type = types.str;
      default = "127.0.0.1";
      example = "[::1]";
    };
    port = mkOption {
      description = ''
        Port to listen on
      '';
      type = types.port;
      default = 8000;
    };
    postgres = {
      host = mkOption {
        type = types.str;
        default = "/run/postgresql";
        description = "Hostname/address of the postgres server to use. If an absolute path is given here, it will be interpreted as a unix socket path.";
      };

      port = mkOption {
        type = types.port;
        default = 5432;
        description = "The port of the postgres server to use.";
      };

      username = mkOption {
        type = types.str;
        default = "bipper";
        description = "The postgres username to use.";
      };

      passwordFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = ''
          Sets the password for authentication with postgres.
          May be unset when using socket authentication.
        '';
      };

      database = mkOption {
        type = types.str;
        default = "bipper";
        description = "The postgres database to use.";
      };
    };
  };
  config = mkIf cfg.enable {
    nixpkgs.overlays = [
      inputs.self.overlays.default
    ];
    systemd.services.bipper = {
      description = "Bipper";
      after = ["network.target" "postgresql.service"];
      wantedBy = ["multi-user.target"];
      serviceConfig = {
        ExecStart = pkgs.writeShellScript "bipper-start" ''
          set -euo pipefail
          ${optionalString (!hasPrefix "/" cfg.postgres.host) "export BIPPER_POSTGRES_PASSWORD=$(< ${cfg.postgres.passwordFile})"}
          exec ${cfg.package}/bin/bipper
        '';
        User = "bipper";
        Group = "bipper";
        DynamicUser = true;
        WorkingDirectory = "/var/lib/bipper";
        StateDirectory = "bipper";
        StateDirectoryMode = "0750";
        Restart = "on-failure";
        # TODO: set sandbox-related options
      };
      environment =
        {
          BIPPER_POSTGRES_HOST = cfg.postgres.host;
          BIPPER_POSTGRES_USER = cfg.postgres.username;
          BIPPER_POSTGRES_DATABASE = cfg.postgres.database;
          BIPPER_ADDRESS = cfg.address;
          BIPPER_PORT = toString cfg.port;
        }
        // optionalAttrs (!hasPrefix "/" cfg.postgres.host) {
          BIPPER_POSTGRES_PORT = toString cfg.postgres.port;
        };
    };
  };
}
