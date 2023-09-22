{
  pkgs,
  self,
}: let
  inherit (pkgs) lib;
  test = {
    name = "bipper-nixos-test";
    nodes.machine = {
      self,
      pkgs,
      ...
    }: {
      imports = [self.nixosModules.default];
      services.postgresql = {
        enable = true;
        ensureDatabases = ["bipper"];
        identMap = ''
          # ArbitraryMapName systemUser DBUser
          superuser_map      root      postgres
          superuser_map      postgres  postgres
          # Let other names login as themselves
          superuser_map      /^(.*)$   \1
        '';
        authentication = pkgs.lib.mkOverride 10 ''
          #type database  DBuser  auth-method optional_ident_map
          local sameuser  all     peer        map=superuser_map
        '';
        ensureUsers = [
          {
            name = "bipper";
            ensurePermissions = {
              "DATABASE bipper" = "ALL PRIVILEGES";
            };
          }
        ];
      };
      services.bipper = {
        enable = true;
        address = "127.0.0.1";
        port = 3939;
      };
    };
    testScript = let
      testData = size: ''
        machine.succeed("head -c ${size} /dev/urandom > /tmp/data-${size}")
        machine.succeed("curl --fail --location 'http://localhost:3939/store/data-${size}' --header 'Content-Type: application/octet-stream' --data '@/tmp/data-${size}' > /tmp/passphrase-${size}")
        machine.succeed("head -c ${size} /dev/urandom > /tmp/data-${size}")
        machine.succeed("curl --fail --location 'http://localhost/retrieve' --header 'Content-Type: application/json' --data '{\"mnemonic\": \"$(< /tmp/passphrase-${size})\"}' > /tmp/retrieved-${size}")
        machine.succeed("diff -q /tmp/data-${size} /tmp/retrieved-${size}")
      '';
    in ''
      machine.wait_for_unit("bipper.service")
      machine.wait_for_open_port(3939)
      ${testData "0"}
      ${testData "9"}
      ${testData "15"}
      ${testData "16"}
      ${testData "17"}
      ${testData "1k"}
      ${testData "1000003"}
      ${testData "1m"}
      ${testData "10m"}
    '';
  };

  testRunner = (import (pkgs.path + "/nixos/lib") {}).runTest {
    defaults.documentation.enable = lib.mkDefault false;
    hostPkgs = pkgs;
    node.specialArgs = {inherit self;};
    imports = [
      test
    ];
  };
in
  testRunner.config.result
