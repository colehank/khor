"""Read-only local checks; reuse CI commands without building native products."""
import argparse
import ast
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def executable(name):
    path = shutil.which(name)
    if path:
        return path
    if name in {"actionlint", "shellcheck"}:
        setting = subprocess.run(["git", "config", "--get", f"khor.{name}"], capture_output=True, text=True)
        if setting.returncode == 0 and Path(setting.stdout.strip()).is_file():
            return setting.stdout.strip()
    raise RuntimeError(f"Missing {name}; install the documented tool before retrying (docs/LOCAL-CHECKS.md)")


def run(command, **kwargs):
    print("+ " + " ".join(command), flush=True)
    subprocess.run(command, check=True, **kwargs)


def syntax(paths):
    for filename in paths:
        path = Path(filename)
        if path.suffix == ".py":
            ast.parse(path.read_bytes(), filename=filename)
        elif path.suffix == ".json":
            json.loads(path.read_bytes())
        elif path.suffix == ".toml":
            tomllib.loads(path.read_text(encoding="utf-8"))
        elif path.suffix in {".js", ".mjs", ".cjs"}:
            run([executable("node"), "--check", filename])
        elif path.suffix == ".sh":
            shell = "bash" if "bash" in path.read_text(encoding="utf-8").split("\n", 1)[0] else "sh"
            run([executable(shell), "-n", filename])


def npm(app, tasks):
    if not Path(f"apps/{app}/node_modules/typescript/package.json").is_file():
        raise RuntimeError(f"Run npm ci --prefix apps/{app} before checking this component")
    for task in tasks:
        run([executable("npm"), "run", task], cwd=f"apps/{app}")


def push_tree():
    status = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=normal"], text=True)
    if status:
        raise RuntimeError("Commit or separately preserve unfinished changes before pushing; checks must describe the pushed tree")
    target = os.environ.get("PRE_COMMIT_TO_REF")
    if target:
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        pushed = subprocess.check_output(["git", "rev-parse", f"{target}^{{commit}}"], text=True).strip()
        if head != pushed:
            raise RuntimeError("Push from a worktree checked out at the commit being pushed")


def script_tests():
    if sys.platform == "win32":
        print("Windows: run portable release/API tests; POSIX shell/process fixtures run on macOS/Linux and CI")
        run([sys.executable, "-m", "unittest", "scripts.tests.test_apple_api", "scripts.tests.test_tag_release", "scripts.tests.test_mac_notarization", "scripts.tests.test_local_checks"])
    else:
        run([sys.executable, "scripts/sync-install-components.py"])
        run([sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests", "-p", "test_*.py"])


def workflows():
    shellcheck = executable("shellcheck")
    version = subprocess.check_output([shellcheck, "--version"], text=True)
    if "version: 0.11.0" not in version.splitlines():
        raise RuntimeError("Use ShellCheck 0.11.0, matching the CI workflow validator")
    run([executable("actionlint"), "-shellcheck", shellcheck])


def main():
    actions = {"workflows": workflows,
               "gui-types": lambda: npm("gui", ["test:types"]),
               "agent-types": lambda: npm("agent", ["check"]),
               "gui": lambda: npm("gui", ["test:unit", "test:types", "gates", "mobile:static"]),
               "agent": lambda: npm("agent", ["test"]),
               "scripts": script_tests, "push-tree": push_tree, "syntax": None}
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("check", choices=actions)
    parser.add_argument("files", nargs="*")
    args = parser.parse_args()
    try:
        if args.check == "syntax":
            syntax(args.files)
        else:
            actions[args.check]()
    except (ValueError, SyntaxError, RuntimeError, OSError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
