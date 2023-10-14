# nrtm

A runtime manager for [Neovim](https://neovim.io/)

## Installation

### Build from source

Requirements:

- [Rust](https://www.rust-lang.org/tools/install/)
- [Python](https://www.python.org/downloads/)

```shell
INSTALL_DIR=$HOME/.nrtm

git clone https://github.com/fusuuu/nrtm && cd nrtm
python build.py --release --dist
./out/nrtm-installer $INSTALL_DIR

export PATH=PATH:$INSTALL_DIR/bin
```

## License

This repository is licensed under the [MIT License](./LICENSE).
