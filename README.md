# nrtm

A runtime manager for [Neovim](https://neovim.io/)

## Installation

### Build from source

Requirements:

- [**Rust**](https://www.rust-lang.org/tools/install/)
- [Python](https://www.python.org/downloads/)

<details>
<summary>Without Python</summary>

```shell
git clone https://github.com/fusuuu/nrtm && cd nrtm
cargo build --release
mkdir -p out/bin
cp ./target/release/nrtm ./out/bin/
cp ./target/release/shim ./out/bin/nvim  # Rename `shim` to `nvim`
```

</details>

```shell
git clone https://github.com/fusuuu/nrtm && cd nrtm
python build.py --release
```

After compiling:

- you can move the `./out` directory to anywhere, e.g. `~/.nrtm`
- add `{out}/bin/` to your `$PATH`

## License

This repository is licensed under the [MIT License](./LICENSE).
