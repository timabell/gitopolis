{
  description = "gitopolis - manage multiple git repositories (prebuilt binary from GitHub releases)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "1.19.1";

      assets = {
        x86_64-linux = {
          name = "gitopolis-linux-x86_64.tar.gz";
          sha256 = "bcd3fcee0fa874c6ee83beb141e8ed6375e84835c4b77b8715760a87f4f13ca3";
        };
        x86_64-darwin = {
          name = "gitopolis-macos-x86_64.tar.gz";
          sha256 = "73aabfceb1c30137f3ec933357cd4daa26068e4775c382e485fc736917f04d2c";
        };
        aarch64-darwin = {
          name = "gitopolis-macos-aarch64.tar.gz";
          sha256 = "6f0d0d285c3b0bcb183b4d9b0926f0180f85eee17bc7d1f251c6cfd9192c9966";
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
