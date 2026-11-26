{
  description = "gitopolis - manage multiple git repositories (prebuilt binary from GitHub releases)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "1.18.0";

      assets = {
        x86_64-linux = {
          name = "gitopolis-linux-x86_64.tar.gz";
          sha256 = "a6388c31991fd9115d33d51173dd498a9ec563e8f02084d5e3ec7283c78c52df";
        };
        x86_64-darwin = {
          name = "gitopolis-macos-x86_64.tar.gz";
          sha256 = "d513aaf99a1545f39ac44430d1d3d4319c259e843e3e95f62bbf6fceb70b1282";
        };
        aarch64-darwin = {
          name = "gitopolis-macos-aarch64.tar.gz";
          sha256 = "4ce789a1e5697d402044f2ffc0bb58a43d08409518434578b42ad3b344e71301";
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
