{
  description = "Lua and Luau source minifier in Rust";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      runtimeEnv = pkgs: {
        LUA51 = "${pkgs.lua5_1}/bin/lua";
        LUA52 = "${pkgs.lua5_2}/bin/lua";
        LUA53 = "${pkgs.lua5_3}/bin/lua";
        LUA54 = "${pkgs.lua5_4}/bin/lua";
        LUAU = "${pkgs.luau}/bin/luau";
      };
    in
    {
      packages = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.rustPlatform.buildRustPackage ({
            pname = "miniluar";
            version = "0.1.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            nativeCheckInputs = [ pkgs.lua5_4 ];
            meta.mainProgram = "minilua";
          } // runtimeEnv pkgs);
        });
      checks = forAllSystems (system: {
        default = self.packages.${system}.default;
      });
      devShells = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.mkShell ({
            packages = with pkgs; [ cargo rustc rustfmt clippy lua5_4 ];
          } // runtimeEnv pkgs);
        });
    };
}
