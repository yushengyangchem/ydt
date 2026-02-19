{ pkgs }:
let
  cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
in
pkgs.rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  outputs = [
    "out"
    "doc"
  ];

  postBuild = ''
    cargo doc --release --no-deps
  '';

  postInstall = ''
    docdir="$CARGO_TARGET_DIR/doc"
    if [ -z "$CARGO_TARGET_DIR" ]; then
      docdir="target/doc"
    fi
    if [ ! -d "$docdir" ]; then
      echo "doc directory not found: $docdir"
      exit 1
    fi
    mkdir -p "$doc/share/doc/$pname"
    cp -r "$docdir/." "$doc/share/doc/$pname/"
  '';
}
