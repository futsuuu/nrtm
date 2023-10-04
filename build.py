"""
This is a wrapper of `cargo build` command.

1> build.py

    out/
     '-- bin/
          '-- *.exe

2> build.py --dist

    nrtm.zip
     '-- nrtm/
          '-- bin/
               '-- *.exe

3> build.py --target $target

    out-{target}/
     '-- bin/
          '-- *.exe

4> build.py --dist --target $target

    nrtm-{target}.zip
     '-- nrtm/
           '-- bin/
                '-- *.exe

"""

import json
import os
import platform
import re
import shutil
import subprocess
import sys


OUT_DIR = "out"
DIST_NAME = "nrtm"
DIST_FLAG = "--dist"
TARGET_FLAG = "--target"


def create_dist(base_name: str, base_dir: str):
    dist_format = "zip" if platform.system() == "Windows" else "gztar"
    shutil.make_archive(
        base_name,
        dist_format,
        root_dir=".",
        base_dir=base_dir,
    )

    dist_name = DIST_NAME + (".zip" if dist_format == "zip" else ".tar.gz")
    print(f"./{base_dir}/ --> ./{dist_name}  # {get_size(dist_name)} KB")


def get_size(path: str) -> int:
    return round(os.path.getsize(path) / 1024)


def main():
    args = sys.argv[1:]

    if dist := DIST_FLAG in args:
        args.remove(DIST_FLAG)
        if "--release" not in args and "-r" not in args:
            args.append("--release")

    build_target = ""
    if TARGET_FLAG in args:
        build_target = args[args.index(TARGET_FLAG) + 1]
        subprocess.run(["rustup", "target", "add", build_target])

    # Install cargo-zigbuild if needed
    build_command = "build"
    if dist and "-musl" in build_target:
        subprocess.run(["python", "-m", "pip", "install", "cargo-zigbuild"])
        build_command = "zigbuild"

    result = subprocess.run(
        ["cargo", build_command, "--message-format", "json"] + args,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )

    out_dir = DIST_NAME if dist else OUT_DIR
    if build_target:
        out_dir += f"-{build_target}"
    bin_dir = os.path.join(out_dir, "bin")

    os.makedirs(bin_dir, exist_ok=True)

    for line in result.stdout.split("\n"):
        if not line:
            continue

        if not (exe := json.loads(line).get("executable")):
            continue

        basename = os.path.basename(exe)
        # Rename `shim` to `nvim`, e.g. `shim.exe` -> `nvim.exe`
        filename = re.sub(r"^shim((\.\w+)+)$", r"nvim\1", basename)

        target = os.path.join(bin_dir, filename)
        shutil.copy2(exe, target)
        print(f"{exe} --> ./{target}  # {get_size(target)} KB")

    if not dist:
        return

    if build_target:
        dist_file = f"{DIST_NAME}-{build_target}"
    else:
        dist_file = DIST_NAME

    create_dist(dist_file, out_dir)
    shutil.rmtree(out_dir)


if __name__ == "__main__":
    main()
