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
            cadaver

            nodejs_25
            nodePackages.pnpm
            typeshare
            jq
            uv
          ];

          shellHook = ''
            export DATABASE_URL="postgres://postgres:postgres@localhost:54322/postgres"
            export SUPABASE_URL="http://127.0.0.1:54321"
            export SUPABASE_ANON_KEY="eyJhbGciOiJFUzI1NiIsImtpZCI6ImI4MTI2OWYxLTIxZDgtNGYyZS1iNzE5LWMyMjQwYTg0MGQ5MCIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjIwODQ2Njc3ODR9.14RcFvWVyolyA2YqEMyaWMmIornT3UIDhqNIskEBglzobeaZWwo5MAHKEXxNDgY2Vzgy4BqDPnuQqziUXf_jPg"
            export API_PORT=3001
            echo "❄️ Televent Nix Shell" >&2
            echo "Database URL: $DATABASE_URL" >&2
          '';
        };
      });
}
