# nrtm-installer

You can download from [releases](https://github.com/futsuuu/nrtm/releases/latest). 

## Usage

```bash
nrtm-installer <directory>
```

Example:

```bash
nrtm-installer $HOME/.nrtm

# Add the `bin` directory to $PATH to use `nvim` and `nrtm` commands
export PATH=$PATH:$HOME/.nrtm/bin
```

## Building from source

Requirements:

- [Rust](https://www.rust-lang.org/tools/install/) 

```bash
git clone https://github.com/fusuuu/nrtm.git
cd nrtm

cargo xtask build --dist -- --release

# ./out/nrtm-installer
```
