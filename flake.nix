{
  description = "gitopolis - manage multiple git repositories (prebuilt binary from GitHub releases)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "1.17.4";

      assets = {
        x86_64-linux = {
          name = "gitopolis-linux-x86_64.tar.gz";
          sha256 = "73403b09594a21af21144f8956f3d0097516fb41c441c932697769cc5d239a5e";
        };
        x86_64-darwin = {
          name = "gitopolis-macos-x86_64.tar.gz";
          sha256 = "feee03312dc9592f13f8b448c39b7a772f528d1d6b2db949c4b04ed6d1328e1f";
        };
        aarch64-darwin = {
          name = "gitopolis-macos-aarch64.tar.gz";
          sha256 = "580dfe464687175a6617c751a7f96519534c362e05a584cda1709a700b7c1340";
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
