"""Offline hooks for a source-only checkout. Full validation belongs to CI."""
import ast
import json
import re
import subprocess
import sys
import tomllib
from pathlib import PurePosixPath


def staged_sources():
    names = subprocess.check_output([
        "git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z",
    ]).split(b"\0")
    for name in filter(None, names):
        path = name.decode("utf-8", errors="surrogateescape")
        if re.search(r"(^|/)(vendor|node_modules|dist|gen|fixtures)/|^apps/gui/lab/", path):
            continue
        suffix = PurePosixPath(path).suffix
        if suffix in {".py", ".json", ".toml", ".sh"}:
            yield path, suffix, subprocess.check_output(["git", "show", ":" + path])


def check_staged():
    subprocess.run(["git", "diff", "--cached", "--check"], check=True)
    for name, suffix, source in staged_sources():
        try:
            if suffix == ".py":
                ast.parse(source, filename=name)
            elif suffix == ".json":
                json.loads(source)
            elif suffix == ".toml":
                tomllib.loads(source.decode("utf-8"))
            elif suffix == ".sh":
                shell = "bash" if b"bash" in source.split(b"\n", 1)[0] else "sh"
                subprocess.run([shell, "-n"], input=source, check=True)
        except (ValueError, SyntaxError, subprocess.CalledProcessError) as error:
            raise ValueError(f"{name}: {error}") from error


def main():
    try:
        if sys.argv[1] == "pre-commit":
            check_staged()
        print("Source-only checkout: offline checks complete. Types, format and tests are NOT validated locally; run scripts/dev.py check and require GitHub Quality before merging.")
        return 0
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
