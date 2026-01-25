{
  description = "Televent development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustVersion = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" "llvm-tools" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            pkg-config
            openssl
            just
            cargo-llvm-cov
            sqlx-cli

            nodejs_20
            jq
            uv
          ];

          shellHook = ''
            export DATABASE_URL="postgres://postgres:postgres@localhost:54322/postgres"
            export API_PORT=3001
            export JWT_SECRET=dev_secret_change_me_in_prod
            echo "❄️ Televent Nix Shell" >&2
            echo "Database URL: $DATABASE_URL" >&2
          '';
        };
      });
}
