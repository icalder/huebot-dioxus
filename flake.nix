{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Crane library for the default system to get filters
        craneLibDefault = crane.mkLib pkgs;

        # Source filtering
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            (craneLibDefault.filterCargoSources path type)
            || (builtins.match ".*hue-openapi\.yaml$" path != null)
            || (builtins.match ".*/assets/.*$" path != null)
            || (builtins.match ".*/tailwind\.css$" path != null);
        };

        # Helper function to build huebot and its docker image for a given hostPkgs
        makeBuild = hostPkgs: let
          # Use a function for overrideToolchain as recommended for cross-compilation
          toolchainFunc = p: p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib hostPkgs).overrideToolchain toolchainFunc;
          
          # Explicitly define vendor dependencies
          vendor = craneLib.vendorCargoDeps { inherit src; };

          # Rust triple for the host platform
          rustTarget = hostPkgs.stdenv.hostPlatform.rust.rustcTarget;

          commonArgs = {
            inherit src;
            strictDeps = true;
            cargoVendorDir = vendor;
            nativeBuildInputs =
              with pkgs;
              [
                pkg-config
                dioxus-cli
                tailwindcss_4
                binaryen
                removeReferencesTo
              ]
              ++ lib.optionals pkgs.stdenv.isDarwin [
                libiconv
              ];
            buildInputs = with hostPkgs; [
              openssl
            ];
          };

          # Build dependencies
          cargoArtifacts = craneLib.buildDepsOnly (
            commonArgs
            // {
              doCheck = false;
            }
          );

          huebot = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              # We use dx bundle instead of the default cargo build
              buildPhase = ''
                export HOME=$(mktemp -d)
                
                # Get the actual toolchain path from rustc
                TOOLCHAIN_PATH=$(rustc --print sysroot)
                echo "Detected toolchain path: $TOOLCHAIN_PATH"

                # Remap path prefixes to remove references to the Rust toolchain, vendor deps, and artifacts
                export RUSTFLAGS="$RUSTFLAGS \
                  --remap-path-prefix $TOOLCHAIN_PATH=/rust-toolchain \
                  --remap-path-prefix ${vendor}=/vendor \
                  --remap-path-prefix ${cargoArtifacts}=/cargo-deps \
                  --remap-path-prefix ${src}=/source \
                  --remap-path-prefix ${hostPkgs.openssl.dev or "/dev/null"}=/openssl-dev \
                  --remap-path-prefix ${hostPkgs.openssl.out or "/dev/null"}=/openssl-out \
                  --remap-path-prefix ${pkgs.stdenv.cc.cc}=/build-cc \
                  --remap-path-prefix ${hostPkgs.stdenv.cc.cc}=/host-cc"
                
                echo "Using RUSTFLAGS: $RUSTFLAGS"

                # Build for the target architecture
                dx bundle --platform web --release @server --target ${rustTarget} --features server
              '';

              installPhase = ''
                mkdir -p $out/bin

                # Find the binary and public folder
                BINARY=$(find target -name huebot -type f | grep release | grep web | head -n 1)
                if [ -z "$BINARY" ]; then
                   echo "Error: Could not find huebot binary in target/"
                   exit 1
                fi
                cp "$BINARY" $out/bin/huebot

                # Strip the binary to remove debug symbols
                echo "Stripping $BINARY using $STRIP"
                $STRIP $out/bin/huebot

                # Force removal of references to build-time dependencies that might have leaked
                echo "Removing references to build-time tools..."
                TOOLCHAIN_PATH=$(rustc --print sysroot)
                
                # We use a loop to ensure we catch all artifacts
                for f in $(find $out -type f); do
                  # Check if it's a binary or wasm (we don't want to process text files unnecessarily, but it doesn't hurt)
                  if file "$f" | grep -qE "ELF|WebAssembly"; then
                    echo "Removing references from $f"
                    remove-references-to -t $TOOLCHAIN_PATH "$f"
                    remove-references-to -t ${vendor} "$f"
                    remove-references-to -t ${cargoArtifacts} "$f"
                    remove-references-to -t ${pkgs.stdenv.cc.cc} "$f"
                    remove-references-to -t ${hostPkgs.stdenv.cc.cc} "$f"
                  fi
                done

                # Dioxus server expects 'public' folder to be next to the executable
                mkdir -p $out/bin/public
                PUBLIC_DIR=$(find target -name public -type d | grep release | grep web | head -n 1)
                if [ -z "$PUBLIC_DIR" ]; then
                   echo "Error: Could not find public directory in target/"
                   exit 1
                fi
                cp -r "$PUBLIC_DIR"/* $out/bin/public/
              '';

              doCheck = false;
            }
          );

          # Function to build image with a specific tag
          buildWithTag = tag: hostPkgs.dockerTools.buildImage {
            name = "huebot";
            inherit tag;
            copyToRoot = hostPkgs.buildEnv {
              name = "image-root";
              paths = [
                huebot
                hostPkgs.cacert
              ];
              pathsToLink = [ "/bin" ];
            };
            config = {
              Cmd = [ "/bin/huebot" ];
              WorkingDir = "/bin";
              ExposedPorts = {
                "8080/tcp" = { };
              };
              Env = [
                "PORT=8080"
                "IP=0.0.0.0"
              ];
            };
          };

          arch = hostPkgs.stdenv.hostPlatform.linuxArch or "latest";
          dockerImage = buildWithTag arch;
        in {
          inherit huebot dockerImage arch;
        };

        # Native build
        native = makeBuild pkgs;

        # Cross-builds (only if on x86_64-linux)
        aarch64 = if system == "x86_64-linux" then makeBuild pkgs.pkgsCross.aarch64-multiplatform else null;
        x86_64 = if system == "aarch64-linux" then makeBuild pkgs.pkgsCross.x86_64-multiplatform else null;

      in
      {
        packages.default = native.huebot;
        packages.docker = native.dockerImage;
        
        # Explicit arch-specific docker images
        packages.docker-aarch64 = if aarch64 != null then aarch64.dockerImage else if system == "aarch64-linux" then native.dockerImage else null;
        packages.docker-x86_64 = if x86_64 != null then x86_64.dockerImage else if system == "x86_64-linux" then native.dockerImage else null;

        packages.huebot-aarch64 = if aarch64 != null then aarch64.huebot else if system == "aarch64-linux" then native.huebot else null;
        packages.huebot-x86_64 = if x86_64 != null then x86_64.huebot else if system == "x86_64-linux" then native.huebot else null;

        # Convenience package to build both at once
        packages.docker-all = pkgs.linkFarm "huebot-docker-all" (
          pkgs.lib.filter (x: x.path != null) [
            { name = "huebot-x86_64.tar.gz"; path = if system == "x86_64-linux" then native.dockerImage else if x86_64 != null then x86_64.dockerImage else null; }
            { name = "huebot-aarch64.tar.gz"; path = if system == "aarch64-linux" then native.dockerImage else if aarch64 != null then aarch64.dockerImage else null; }
          ]
        );

        devShells.default = pkgs.mkShell {
          inputsFrom = [ native.huebot ];
          packages = [
            (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
            pkgs.rust-analyzer-unwrapped
          ];
        };
      }
    );
}
