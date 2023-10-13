"""
This is a wrapper of `cargo build` command.

1> build.py

    out/
     '-- bin/
          '-- *.exe

2> build.py --dist

    out.zip
     '-- *.exe
"""

import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from sys import exit, argv
from typing import Iterator


OUT = "out"
DIST_FLAG = "--dist"
TARGET_FLAG = "--target"


def get_size(path: Path) -> int:
    return round(os.path.getsize(path) / 1024)


def with_build_target(path: Path, target: str) -> Path:
    if not target:
        return path
    stem = f"{path.stem}-{target}"
    return path.with_stem(stem)


def build_and_get_executables(args: list[str]) -> Iterator[Path]:
    result = subprocess.run(
        args + ["--message-format", "json"], stdout=subprocess.PIPE, encoding="utf-8"
    )
    if result.returncode != 0:
        exit("Compiling failed.")

    for line in result.stdout.split("\n"):
        try:
            yield Path(json.loads(line)["executable"])
        except:
            pass


def main():
    print("Check nrtm package")
    if subprocess.run(["cargo", "check", "--package", "nrtm"]).returncode != 0:
        exit("Checking failed.")

    args = ["cargo", "build"] + argv[1:]

    if dist := DIST_FLAG in args:
        args.remove(DIST_FLAG)

    if TARGET_FLAG in args:
        build_target = args[args.index(TARGET_FLAG) + 1]
        subprocess.run(["rustup", "target", "add", build_target])
    else:
        build_target = ""

    # Static linking for musl target
    if "-musl" in build_target:
        subprocess.run(["python", "-m", "pip", "install", "cargo-zigbuild"])
        args[1] = "zigbuild"

    out_dir = with_build_target(Path(".") / OUT, build_target)
    bin_dir = out_dir / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)

    print("Compile nrtm package")

    for exe in build_and_get_executables(args + ["--package", "nrtm"]):
        # Rename `shim` to `nvim`, e.g. `shim.exe` -> `nvim.exe`
        filename = re.sub(r"^shim((\.\w+)*)$", r"nvim\1", exe.name)

        target = bin_dir / filename
        shutil.copy2(exe, target)
        print(f"Copy {exe} --> {target}  # {get_size(target)} KB")

    if not dist:
        return

    zip_root = bin_dir

    print("Create zip file")
    shutil.make_archive(OUT, "zip", zip_root)

    dist_name = Path(".") / f"{OUT}.zip"
    print(f"Archive {zip_root} --> {dist_name}  # {get_size(dist_name)} KB")

    print("Compile nrtm-installer")
    installer = next(
        build_and_get_executables(["cargo", "build", "--package", "nrtm-installer"])
    )

    print(f"Remove {dist_name}")
    dist_name.unlink()

    target = with_build_target(out_dir / installer.name, build_target)
    shutil.copy2(installer, target)
    print(f"Copy {installer} --> {target}  # {get_size(installer)} KB")


if __name__ == "__main__":
    main()
