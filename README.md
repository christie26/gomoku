### Dev instructions

- Enter developing shell with `nix develop`
- Go to faster_functions and run `maturin develop --release` to build the rust Gomoku class as a python package. it will get installed in the venv that you entered thanks the command in the last step
- In another shell, run `nix develop` again to be in the same venv, then run gomoku with `python -m cProfile -o profile.prof gomoku_gui.py --black 1 --white 1`.
- The profile.prof file generated can be opened with snakeviz, by running `snakeviz profile.prof`
