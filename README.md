### Dev instructions

#### Shell 1 - make your library
- Enter developing shell with
```
nix develop
```
- Go to lib_gomoku
```
cd lib_gomoku
```
- Run
```
maturin develop --release
``` 
to build the rust Gomoku class as a python package.
it will get installed in the venv that you entered thanks the command in the last step

#### Shell 2 - run python file using rust library
- In another shell, run
```
nix develop
```
again to be in the same ven
- Run gomoku with 
```
python -m cProfile -o profile.prof src/gomoku_gui.py --black Etienne --white Yoonseo
```
- The profile.prof file generated can be opened with snakeviz, by running
```
snakeviz profile.prof
```

if you want to enter zsh, do
```
nix develop -c zsh
```

#### Running the cli AI vs AI 

```
nix develop
cd lib_gomoku
cargo run --release
```

```
cargo run --release
```
(more compile time, but faster exec) to run it
