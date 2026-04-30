{
  description = "gitopolis - manage multiple git repositories (prebuilt binary from GitHub releases)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "1.17.1";

      assets = {
        x86_64-linux = {
          name = "gitopolis-linux-x86_64.tar.gz";
          sha256 = "5f3a5229cc3556843cae7ab0b9a2d5bb2f1a7d0523040e389694722ddcb7e159";
        };
        x86_64-darwin = {
          name = "gitopolis-macos-x86_64.tar.gz";
          sha256 = "5e75092af69c590ba6c3b5fc88402a2194ef208ebebd6f735f972859a87e4d42";
        };
        aarch64-darwin = {
          name = "gitopolis-macos-aarch64.tar.gz";
          sha256 = "6d19de283cb58f7f815ab3847062b8cdbd2205930bb762e02a9991288b7c5e73";
        };
      };

      systems = builtins.attrNames assets;
      forAllSystems = nixpkgs.lib.genAttrs systems;

      mkPackage = system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          asset = assets.${system};
        in
        pkgs.stdenvNoCC.mkDerivation {
          pname = "gitopolis";
          inherit version;

          src = pkgs.fetchurl {
            url = "https://github.com/timabell/gitopolis/releases/download/v${version}/${asset.name}";
            sha256 = asset.sha256;
          };

          dontUnpack = true;

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            tar -xzf $src -C $out/bin gitopolis
            chmod +x $out/bin/gitopolis
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Manage multiple git repositories - CLI tool - run commands, clone, and organize repos with tags";
            homepage = "https://github.com/timabell/gitopolis";
            license = licenses.agpl3Only;
            platforms = systems;
            mainProgram = "gitopolis";
          };
        };
    in
    {
      packages = forAllSystems (system: {
        default = mkPackage system;
        gitopolis = mkPackage system;
      });

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${mkPackage system}/bin/gitopolis";
        };
      });
    };
}
