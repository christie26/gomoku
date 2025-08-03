{
  description = "Gomoku dev env";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # pkgs = nixpkgs.legacyPackages.${system};
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ (import rust-overlay) ];
        };
        pythonEnv = pkgs.python3.withPackages (ps: with ps; [
          tkinter
          # Add your project dependencies here
        ]);
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src" # for rust-analyzer
            "rust-analyzer"
            "rustfmt"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            pythonEnv
            python3Packages.python-lsp-server
            python3Packages.black
            rust
          ];

        };
      });
}
