"""
This is a wrapper of `cargo build` command.
You can pass the `--release` flag, the `--target` flag, etc.
If you pass the `--dist` flag, `out.zip` or `out.tar.gz` will be created.
"""

import json
import os
import platform
import re
import shutil
import subprocess
import sys


ARCHIVE_NAME = "out"
DIST_FLAG = "--dist"


def make_archive(directory: str):
    shutil.make_archive(
        ARCHIVE_NAME,
        "zip" if platform.system() == "Windows" else "gztar",
        root_dir=".",
        base_dir=directory,
    )


def main():
    args = sys.argv[1:]

    if dist := DIST_FLAG in args:
        args.remove(DIST_FLAG)

    result = subprocess.run(
        ["cargo", "build", "--message-format", "json"] + args,
        stdout=subprocess.PIPE,
        encoding="utf-8",
    )

    out_dir = "nrtm" if dist else ARCHIVE_NAME
    bin_dir = os.path.join(out_dir, "bin")

    os.makedirs(bin_dir, exist_ok=True)

    for line in result.stdout.split("\n"):
        if not line:
            continue

        data = json.loads(line)
        if exe := data.get("executable"):
            basename = os.path.basename(exe)
            # Rename `shim` to `nvim`, e.g. `shim.exe` -> `nvim.exe`
            filename = re.sub(r"^shim((\.\w+)+)$", r"nvim\1", basename)
            shutil.copy(exe, os.path.join(bin_dir, filename))

    if dist:
        make_archive(out_dir)
        shutil.rmtree(out_dir)


if __name__ == "__main__":
    main()
