{ pkgs, naersk }:
let
  naersk' = pkgs.callPackage naersk { };
in
naersk'.buildPackage {
  src = ./.;
  doCheck = true;
}
