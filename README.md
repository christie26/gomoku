### Dev instructions

Shell 1
- Enter developing shell with `nix develop`
- Go to faster_functions `cd faster_functions`
- Run `maturin develop --release` to build the rust Gomoku class as a python package. 
it will get installed in the venv that you entered thanks the command in the last step

Shell 2
- In another shell, run `nix develop` again to be in the same venv
- Run gomoku with `python -m cProfile -o profile.prof gomoku_gui.py --black 1 --white 1`.
- The profile.prof file generated can be opened with snakeviz, by running `snakeviz profile.prof`

if you want to enter zsh, do `nix develop -c zsh`