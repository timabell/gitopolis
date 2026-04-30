{
  description = "gitopolis - manage multiple git repositories (prebuilt binary from GitHub releases)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      version = "1.17.0";

      assets = {
        x86_64-linux = {
          name = "gitopolis-linux-x86_64.tar.gz";
          sha256 = "eb2c6f616e327938be90edf0e87832afcd32878109bd027666a9a85952597f0b";
        };
        x86_64-darwin = {
          name = "gitopolis-macos-x86_64.tar.gz";
          sha256 = "7e38a82233a2582abc139425849cac4911fad9e0073250e8a6cca0bdf9f12386";
        };
        aarch64-darwin = {
          name = "gitopolis-macos-aarch64.tar.gz";
          sha256 = "76fdb1c10ab314ce6924b0ceef46e47f797e1923977997f7a88d8c391c9d33e7";
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
