{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        # Source filtering
        src = pkgs.lib.cleanSourceWith {
          src = ./.; 
          filter = path: type:
            (craneLib.filterCargoSources path type) ||
            (builtins.match ".*hue-openapi\.yaml$" path != null) ||
            (builtins.match ".*/assets/.*$" path != null) ||
            (builtins.match ".*/tailwind\.css$" path != null);
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            pkg-config
            dioxus-cli
            binaryen
          ] ++ lib.optionals stdenv.isDarwin [
            libiconv
          ];
          buildInputs = with pkgs; [
            openssl
          ];
        };

        # Build dependencies
        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
           doCheck = false;
        });

        huebot = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          
          # We use dx bundle instead of the default cargo build
          buildPhase = ''
            export HOME=$(mktemp -d)
            dx bundle --platform web --release
          '';

          installPhase = ''
            mkdir -p $out/bin
            
            # The binary is named after the package name 'huebot'
            cp target/dx/huebot/release/web/huebot $out/bin/huebot
            
            # Strip the binary to remove debug symbols
            $STRIP $out/bin/huebot
            
            # Dioxus server expects 'public' folder to be next to the executable
            mkdir -p $out/bin/public
            cp -r target/dx/huebot/release/web/public/* $out/bin/public/
          '';
          
          doCheck = false;
        });
        
        dockerImage = pkgs.dockerTools.buildImage {
          name = "huebot";
          tag = "latest";
          copyToRoot = pkgs.buildEnv {
             name = "image-root";
             paths = [ huebot pkgs.cacert ];
             pathsToLink = [ "/bin" ];
          };
          config = {
            Cmd = [ "${huebot}/bin/huebot" ];
            WorkingDir = "${huebot}/bin";
            ExposedPorts = {
              "8080/tcp" = {};
            };
            Env = [
              "PORT=8080"
              "IP=0.0.0.0"
            ];
          };
        };
      in
      {
        packages.default = huebot;
        packages.docker = dockerImage;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ huebot ];
          packages = [
            toolchain
            pkgs.rust-analyzer-unwrapped
            pkgs.dioxus-cli
          ];
        };
      }
    );
}