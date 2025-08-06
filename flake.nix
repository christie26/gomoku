{
  description = "Python + Rust development environment with Maturin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      # Python with packages you might need
      python = pkgs.python311.withPackages (ps:
        with ps; [
          tkinter
          pip
          setuptools
          wheel
          virtualenv
          snakeviz
        ]);
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          # Python
          python
          pkgs.python311Packages.tkinter

          # Rust toolchain
          rustc
          cargo
          rustfmt
          clippy

          # Maturin
          maturin

          # Build tools that might be needed
          pkg-config
          openssl
          git
        ];

        shellHook = ''
          echo "🐍🦀 Python + Rust + Maturin development environment"
          echo "Python version: $(python --version)"
          echo "Rust version: $(rustc --version)"
          echo "Maturin version: $(maturin --version)"

          # Create and activate virtual environment
          if [ ! -d ".venv" ]; then
            echo "Creating virtual environment..."
            python -m venv .venv
          fi

          source .venv/bin/activate
          echo "✅ Virtual environment activated"
          echo ""
          echo "Ready to run: maturin develop"
        '';
      };
    });
}
