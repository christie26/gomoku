### Dev instructions

Shell 1
- Enter developing shell with `nix develop`
- Go to faster_functions `cd faster_functions`
- Run `maturin develop --release` to build the rust Gomoku class as a python package. 
it will get installed in the venv that you entered thanks the command in the last step

Shell 2
- In another shell, run `nix develop` again to be in the same venv
- Run gomoku with `python -m cProfile -o profile.prof gomoku_gui.py --black Etienne --white Yoonseo`.
- The profile.prof file generated can be opened with snakeviz, by running `snakeviz profile.prof`

if you want to enter zsh, do `nix develop -c zsh`

#### Running the cli AI vs AI 

- `nix develop` to enter the dev shell
- Make sure to be in `cd faster_functions`
- `cargo run` or `cargo run --release` (more compile time, but faster exec) to run it.

