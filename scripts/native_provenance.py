"""First-party Cargo provenance; Cargo/process execution stays with the caller.

prepare_build snapshots inputs and selects a bounded, explicit source refresh.
verify_build admits actual Cargo output only after that snapshot still matches.
A fresh Cargo row requires a previous content receipt, never just its paths.
This is a trusted build receipt, not authentication of untrusted Cargo JSON.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import time


VERSION = 1


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def key(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def regular(path):
    path = Path(path)
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or path.resolve() != path.absolute():
        raise ValueError(f"Expected a regular file without symlink ancestors: {path}")
    return {"sha256": digest(path), "size": info.st_size,
            "mtime_ns": info.st_mtime_ns, "dev": info.st_dev,
            "ino": info.st_ino, "uid": info.st_uid, "nlink": info.st_nlink}


def source_frame(root):
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root).decode().split("\0")
    frame = {}
    for name in sorted(set(names) - {""}):
        path = root / name
        if path.is_symlink():
            frame[name] = {"link": os.readlink(path)}
        elif path.is_file():
            frame[name] = {"sha256": digest(path)}
        elif not path.exists():
            frame[name] = {"missing": True}
    return frame


def tree(path):
    path = Path(path)
    if path.is_dir() and not path.is_symlink():
        return {str(p.relative_to(path)): regular(p)["sha256"]
                for p in sorted(path.rglob("*")) if not p.is_dir() or p.is_symlink()}
    return {".": regular(path)["sha256"]}


def environment():
    # Hash values so credentials accidentally carried in CARGO_* are not printed.
    names = {"PATH", "CC", "CXX", "AR", "CFLAGS", "CPPFLAGS", "CXXFLAGS", "LDFLAGS",
             "SDKROOT", "MACOSX_DEPLOYMENT_TARGET", "IPHONEOS_DEPLOYMENT_TARGET",
             "KHOR_AGENT_BUNDLE"}
    names.update(n for n in os.environ if n.startswith(("CARGO", "RUST", "KHOR_", "CC_", "CXX_", "AR_", "PKG_CONFIG")))
    return {n: key(os.environ.get(n)) for n in sorted(names)}


def context(root, tools):
    configs = {root / "Cargo.toml", root / "Cargo.lock",
               root / "rust-toolchain", root / "rust-toolchain.toml"}
    for directory in (root, *root.parents):
        configs.update(directory / ".cargo" / name for name in ("config", "config.toml"))
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    configs.update(cargo_home / name for name in ("config", "config.toml"))
    return {"environment": environment(),
            "tools": {str(Path(p).resolve()): regular(Path(p).resolve())["sha256"]
                      for p in tools},
            "config": {str(p): regular(p)["sha256"] if p.exists() else None
                       for p in sorted(configs)}}


def packages(root, metadata):
    if metadata.get("version") != 1 or Path(metadata["workspace_root"]).resolve() != root:
        raise ValueError("Cargo metadata belongs to another workspace")
    members = set(metadata["workspace_members"])
    result = {}
    for p in metadata["packages"]:
        if p["id"] not in members:
            continue
        manifest = Path(p["manifest_path"])
        manifest.relative_to(root)
        regular(manifest)
        result[p["id"]] = p
    if set(result) != members or not metadata.get("resolve"):
        raise ValueError("Expected complete workspace packages and Cargo resolve graph")
    graph = {n["id"]: set(n["dependencies"]) & members
             for n in metadata["resolve"]["nodes"]}
    if not members <= graph.keys():
        raise ValueError("Incomplete workspace dependency graph")
    return result, graph


def closure(graph, start):
    selected = {start}
    while True:
        expanded = selected | {d for p in selected for d in graph[p]}
        if selected == expanded:
            return selected
        selected = expanded


def build_state(root, metadata, tools):
    source = source_frame(root)
    pkgs, graph = packages(root, metadata)
    common = context(root, tools)
    common["vendored_sources"] = {n: v for n, v in source.items() if n.startswith("vendor/")}
    states = {}
    for ident, package in pkgs.items():
        directory = Path(package["manifest_path"]).parent
        prefix = str(directory.relative_to(root)) + "/"
        inputs = {n: v for n, v in source.items() if n.startswith(prefix)}
        if not inputs or any("sha256" not in v for v in inputs.values()):
            raise ValueError(f"Missing or nonregular package input: {prefix}")
        extras = {}
        if package["name"] == "khor-web":
            extras["apps/gui/dist"] = tree(root / "apps/gui/dist")
        if package["name"] == "khor-node" and os.environ.get("KHOR_AGENT_BUNDLE"):
            extras[os.environ["KHOR_AGENT_BUNDLE"]] = tree(os.environ["KHOR_AGENT_BUNDLE"])
        # Default to root-sensitive: env!, file!, generated absolute include paths,
        # and proc-macro expansion are not proven relocatable by a text search.
        states[ident] = {"manifest": str(Path(package["manifest_path"]).relative_to(root)),
                         "inputs": inputs, "extra_inputs": extras, "embedded_root": str(root)}
    keys = {p: key({"context": common,
                    "packages": {states[d]["manifest"]: states[d] for d in closure(graph, p)}})
            for p in pkgs}
    return source, common, pkgs, graph, states, keys


def matching_prior(priors, package_key, variant=None):
    for prior in priors:
        if prior.get("version") != VERSION:
            continue
        for unit in prior.get("units", []):
            if unit["package_key"] == package_key and (variant is None or unit["variant"] == variant):
                yield unit


def same_files(files):
    try:
        return all(content(regular(p)) == content(stamp) for p, stamp in files.items())
    except (OSError, ValueError):
        return False


def content(stamp):
    return stamp["sha256"], stamp["size"]


def same_generation(generation):
    try:
        out = Path(generation["out_dir"])
        expected = {str(Path(name).relative_to(out)): stamp["sha256"]
                    for name, stamp in generation["files"].items()}
        return (tree(out) == expected
                and content(regular(out.parent / "output")) == content(generation["invocation_marker"]))
    except (OSError, ValueError, KeyError):
        return False


def prepare_build(root, target_dir, metadata, request, tools, prior_receipts=(), apply_refresh=False):
    """Snapshot current inputs; optionally touch only unproven first-party entries.

    The caller must own the Cargo slot before apply_refresh=True. No Cargo,
    cache deletion, fingerprint mutation or process launch happens here.
    """
    root, target_dir = Path(root).resolve(), Path(target_dir).resolve()
    if len(tools) < 2:
        raise ValueError("Record the actual Cargo and rustc binaries")
    source, common, pkgs, graph, states, keys = build_state(root, metadata, tools)
    selected = [p for p, v in pkgs.items() if v["name"] == request["package"]]
    if len(selected) != 1:
        raise ValueError("Expected one requested workspace package")
    selected = closure(graph, selected[0])
    refresh = {}
    for ident in sorted(selected):
        for target in pkgs[ident]["targets"]:
            requested = (pkgs[ident]["name"] == request["package"]
                         and target["name"] == request["name"] and target["kind"] == request["kind"])
            if requested or set(target["kind"]) & {"lib", "custom-build", "proc-macro", "rlib", "cdylib", "staticlib"}:
                expected = {"name": target["name"], "kind": target["kind"],
                            "test": request["test"] if requested else False}
                reusable = any(all(unit["variant"][name] == value for name, value in expected.items())
                               and same_files(unit["files"])
                               for unit in matching_prior(prior_receipts, keys[ident]))
                if target["kind"] == ["custom-build"]:
                    # A shared build-script executable can be unchanged across
                    # A/B while its OUT_DIR still contains the other state's data.
                    reusable = reusable and any(
                        generation["package_key"] == keys[ident] and same_generation(generation)
                        for prior in prior_receipts for generation in prior.get("generated", []))
                if reusable:
                    continue
                path = Path(target["src_path"])
                path.relative_to(Path(pkgs[ident]["manifest_path"]).parent)
                name = str(path.relative_to(root))
                before = regular(path)
                if source.get(name) != {"sha256": before["sha256"]}:
                    raise ValueError(f"Refresh input absent from complete source frame: {name}")
                refresh[name] = before
    changed = []
    if apply_refresh:
        for name, before in refresh.items():
            path = root / name
            if regular(path) != before:
                raise ValueError(f"Refresh input changed: {name}")
            stamp = max(time.time_ns(), before["mtime_ns"] + 1)
            os.utime(path, ns=(path.stat().st_atime_ns, stamp), follow_symlinks=False)
            changed.append({"path": name, "before": before, "after": regular(path)})
    return {"version": VERSION, "root": str(root), "target_dir": str(target_dir),
            "prepared_ns": time.time_ns(), "source": source, "context": common,
            "metadata": metadata, "tools": [str(Path(p).resolve()) for p in tools],
            "request": request, "package_keys": keys, "selected": sorted(selected),
            "states": states, "refresh_plan": sorted(refresh), "refreshed": changed,
            "prior_receipts": list(prior_receipts)}


def make_words(value):
    """Rustc Make escaping: continuation, escaped whitespace/#/:/backslash, $$.

    Refuse unsupported Make expressions instead of guessing a source path.
    """
    words, word, i = [], "", 0
    while i < len(value):
        char = value[i]
        if char == "\\":
            i += 1
            if i == len(value):
                raise ValueError("Trailing dep-info escape")
            word += value[i]
        elif char == "$":
            if value[i:i + 2] != "$$":
                raise ValueError("Unsupported Make expression")
            word += "$"
            i += 1
        elif char.isspace():
            if word:
                words.append(word)
                word = ""
        else:
            word += char
        i += 1
    if word:
        words.append(word)
    return words


def depinfo(path, cwd):
    raw = Path(path).read_text().replace("\\\n", "")
    targets, sources = set(), set()
    for line in raw.splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        index = 0
        while index < len(line):
            if line[index] == "\\":
                index += 2
                continue
            if line[index] == ":" and (index + 1 == len(line) or line[index + 1].isspace()):
                break
            index += 1
        if index == len(line):
            raise ValueError("Unsupported dep-info rule")
        lhs, rhs = line[:index], line[index + 1:]
        targets.update(os.path.abspath(cwd / p) for p in make_words(lhs))
        sources.update(os.path.abspath(cwd / p) for p in make_words(rhs))
    return targets, sources


def variant(row, root):
    target = row["target"]
    return {"name": target["name"], "kind": target["kind"],
            "test": row["profile"]["test"], "profile": row["profile"],
            "features": sorted(row["features"]), "edition": target["edition"],
            "source": str(Path(target["src_path"]).relative_to(root))}


def artifact_files(row, root, target_dir):
    for path in row["filenames"]:
        Path(path).relative_to(target_dir)
    files = {str(Path(p)): regular(p) for p in row["filenames"]}
    executable = row.get("executable")
    first = Path(executable or row["filenames"][0])
    first.relative_to(target_dir)
    paired = first
    if executable and first.parent.name != "deps":
        # The top-level .d may be left over by incidental Cargo test builds.
        crate_name = row["target"]["name"].replace("-", "_")
        candidates = [p for p in (first.parent / "deps").glob(crate_name + "-*")
                      if re.fullmatch(re.escape(crate_name) + r"-[0-9a-f]{16}", p.name)
                      and regular(p)["sha256"] == files[str(first)]["sha256"]
                      and regular(p)["size"] == files[str(first)]["size"]
                      and regular(p)["mtime_ns"] == files[str(first)]["mtime_ns"]]
        if len(candidates) != 1:
            raise ValueError("Expected one content-paired normal executable")
        paired = candidates[0]
        files[str(paired)] = regular(paired)
    stem = paired.stem.removeprefix("lib") if paired.suffix in (".rlib", ".rmeta") else paired.name
    candidates = [paired.parent / (stem + ".d")]
    if row["target"]["kind"] == ["custom-build"]:
        candidates = list(paired.parent.glob("*.d"))
    matches = []
    for path in candidates:
        if not path.is_file():
            continue
        targets, sources = depinfo(path, root)
        if str(Path(row["target"]["src_path"])) in sources:
            declared = str(paired) in targets
            if row["target"]["kind"] == ["custom-build"] and not declared:
                pairs = [Path(t) for t in targets
                         if Path(t).parent == paired.parent
                         and re.fullmatch(r"build_script_build-[0-9a-f]{16}", Path(t).name)]
                if len(pairs) == 1 and pairs[0].is_file():
                    stamp = regular(pairs[0])
                    declared = (content(stamp) == content(files[str(first)])
                                and stamp["mtime_ns"] == files[str(first)]["mtime_ns"])
                    if declared:
                        files[str(pairs[0])] = stamp
            if declared and str(path) in targets:
                matches.append((path, targets, sources))
    if len(matches) != 1:
        raise ValueError(f"Missing unique paired rustc dep-info: {paired}")
    path, targets, sources = matches[0]
    files[str(path)] = regular(path)
    return files, {"path": str(path), "targets": sorted(targets), "sources": sorted(sources)}


def covered_inputs(prepared, package):
    """Only inputs already represented in this producer's key may be admitted."""
    root = Path(prepared["root"])
    members = set(prepared["states"])
    graph = {n["id"]: set(n["dependencies"]) & members
             for n in prepared["metadata"]["resolve"]["nodes"] if n["id"] in members}
    covered = closure(graph, package)
    frozen = {str(root / name): value["sha256"] for ident in covered
              for name, value in prepared["states"][ident]["inputs"].items()}
    frozen.update({name: sha for name, sha in prepared["context"]["config"].items() if sha})
    frozen.update(prepared["context"]["tools"])
    frozen.update({str(root / name): value["sha256"]
                   for name, value in prepared["context"]["vendored_sources"].items()
                   if "sha256" in value})
    extras = {str((root / name).resolve()): entries for ident in covered
              for name, entries in prepared["states"][ident]["extra_inputs"].items()}
    return frozen, extras, covered


def generated_output(row, prepared, package, priors):
    out = Path(row["out_dir"])
    out.relative_to(Path(prepared["target_dir"]))
    if not out.is_dir() or out.resolve() != out.absolute():
        raise ValueError("Generated output directory is absent or has symlink ancestors")
    files = {str(p): regular(p) for p in sorted(out.rglob("*"))
             if not p.is_dir() or p.is_symlink()}
    marker = regular(out.parent / "output")
    package_root = Path(next(p["manifest_path"] for p in prepared["metadata"]["packages"]
                             if p["id"] == package)).parent
    frozen, extras, _ = covered_inputs(prepared, package)
    for line in (out.parent / "output").read_text().splitlines():
        instruction = re.fullmatch(r"cargo::?(rerun-if-changed|rerun-if-env-changed)=(.*)", line)
        if not instruction:
            continue
        kind, value = instruction.groups()
        if kind == "rerun-if-env-changed":
            if value not in prepared["context"]["environment"]:
                raise ValueError(f"Build script names an unrecorded environment input: {value}")
        else:
            path = os.path.abspath(package_root / value)
            if path in extras:
                if tree(path) != extras[path]:
                    raise ValueError("Generated input tree changed")
            elif path not in frozen or digest(path) != frozen[path]:
                raise ValueError(f"Build script names an unfrozen input outside its producer key; configure it explicitly: {path}")
    previous = [g for p in priors for g in p.get("generated", [])
                if g["package_key"] == prepared["package_keys"][package]
                and {p: content(s) for p, s in g["files"].items()}
                == {p: content(s) for p, s in files.items()}]
    if not previous:
        if marker["mtime_ns"] < prepared["prepared_ns"] or any(
                s["mtime_ns"] < prepared["prepared_ns"] for s in files.values()):
            raise ValueError("Generated output has no current invocation or matching prior receipt")
    return {"package_key": prepared["package_keys"][package], "out_dir": str(out),
            "files": files, "invocation_marker": marker}


def verify_build(prepared, messages):
    """Verify a successful guarded Cargo build. Does not launch Cargo or tests."""
    root = Path(prepared["root"])
    source, common, pkgs, _, states, keys = build_state(root, prepared["metadata"], prepared["tools"])
    if (source != prepared["source"] or common != prepared["context"]
            or states != prepared["states"] or keys != prepared["package_keys"]):
        raise ValueError("Build inputs changed after preparation")
    if [m.get("success") for m in messages if m.get("reason") == "build-finished"] != [True]:
        raise ValueError("Expected one successful actual Cargo build")
    rows = [m for m in messages if m.get("reason") == "compiler-artifact" and m["package_id"] in pkgs]
    if set(prepared["selected"]) != {m["package_id"] for m in rows}:
        raise ValueError("Actual first-party rows do not cover the prepared build graph")
    priors = prepared["prior_receipts"]
    generated = [generated_output(m, prepared, m["package_id"], priors) for m in messages
                 if m.get("reason") == "build-script-executed" and m["package_id"] in pkgs]
    script_packages = [m["package_id"] for m in messages
                       if m.get("reason") == "build-script-executed" and m["package_id"] in pkgs]
    expected_scripts = {m["package_id"] for m in rows if m["target"]["kind"] == ["custom-build"]}
    if len(script_packages) != len(set(script_packages)) or set(script_packages) != expected_scripts:
        raise ValueError("Missing or ambiguous actual build-script output")
    units = []
    for row in rows:
        package = pkgs[row["package_id"]]
        if row["manifest_path"] != package["manifest_path"] or not any(
                t["src_path"] == row["target"]["src_path"] for t in package["targets"]):
            raise ValueError("Actual artifact target/manifest differs from current metadata")
        v = variant(row, root)
        files, dep = artifact_files(row, root, Path(prepared["target_dir"]))
        allowed, extras, covered = covered_inputs(prepared, row["package_id"])
        for path, entries in extras.items():
            allowed.update({str((Path(path) / name).resolve()): sha for name, sha in entries.items()})
        covered_keys = {keys[ident] for ident in covered}
        for g in generated:
            if g["package_key"] in covered_keys:
                allowed.update({p: s["sha256"] for p, s in g["files"].items()})
        for path in dep["sources"]:
            if path not in allowed or digest(path) != allowed[path]:
                raise ValueError(f"Dep-info source is not a frozen build input covered by its producer key; configure it explicitly: {path}")
        previous = [u for u in matching_prior(priors, keys[row["package_id"]], v)
                    if {p: s["sha256"] for p, s in u["files"].items()}
                    == {p: s["sha256"] for p, s in files.items()}]
        if row["fresh"] is not False and not (row["fresh"] is True and previous):
            raise ValueError(f"Fresh first-party artifact lacks a matching producer: {v['name']}")
        if row["fresh"] is False and any(s["mtime_ns"] < prepared["prepared_ns"]
                                         for path, s in files.items() if not path.endswith(".d")):
            raise ValueError("New first-party artifact predates the prepared build window")
        units.append({"package_key": keys[row["package_id"]], "variant": v,
                      "files": files, "depinfo": dep, "row": row,
                      "reuse": bool(row["fresh"])})
    request = prepared["request"]
    selected = [u for u in units if pkgs[u["row"]["package_id"]]["name"] == request["package"]
                and all(u["variant"][k] == request[k] for k in ("name", "kind", "test"))]
    if len(selected) != 1:
        raise ValueError("Expected exactly one requested actual build target")
    return {"version": VERSION, "root": str(root), "source": source, "context": common,
            "units": units, "generated": generated, "request": request,
            "extra_inputs": {p: entries for state in states.values()
                             for p, entries in state["extra_inputs"].items()},
            "selected": selected[0], "messages_sha256": key(messages)}


def require_provenance(receipt, root, row):
    """Admission for an existing native runner; never substitutes for its guards."""
    if (receipt.get("version") != VERSION or receipt["root"] != str(root.resolve())
            or receipt["source"] != source_frame(root)):
        raise ValueError("Provenance source frame does not match the current tree")
    unit = receipt["selected"]
    # Linked executables remain proven when Cargo legitimately overwrites an
    # intermediate rlib later. Reuse of that rlib needs its own current check.
    if unit["row"] != row or not same_files(unit["files"]):
        raise ValueError("Actual native artifact does not match the proven build")
    if not row["executable"] or not (row["profile"]["test"] or row["target"]["kind"] == ["bin"]):
        raise ValueError("Native admission requires a linked executable")
    if any(tree(root / name) != entries for name, entries in receipt["extra_inputs"].items()):
        raise ValueError("Generated build inputs changed before native acceptance")
    current = context(root, receipt["context"]["tools"])
    current["vendored_sources"] = {n: v for n, v in receipt["source"].items() if n.startswith("vendor/")}
    if current != receipt["context"]:
        raise ValueError("Build environment/config/toolchain changed before native acceptance")


def read_messages(path):
    messages = []
    for line in Path(path).read_text().splitlines():
        try:
            message = json.loads(line)
        except ValueError:
            continue  # Cargo may interleave rendered diagnostics.
        if not isinstance(message, dict):
            raise ValueError("Expected Cargo JSON objects")
        messages.append(message)
    return messages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare")
    prepare.add_argument("--root", type=Path, default=Path.cwd())
    prepare.add_argument("--target-dir", type=Path, required=True)
    prepare.add_argument("--metadata", type=Path, required=True)
    prepare.add_argument("--package", required=True)
    prepare.add_argument("--name", required=True)
    prepare.add_argument("--kind", choices=("lib", "bin"), required=True)
    prepare.add_argument("--test", action="store_true")
    prepare.add_argument("--tool", type=Path, action="append", required=True)
    prepare.add_argument("--prior", type=Path, action="append", default=[])
    prepare.add_argument("--apply-refresh", action="store_true")
    verify = commands.add_parser("verify")
    verify.add_argument("--prepared", type=Path, required=True)
    verify.add_argument("--artifacts", type=Path, required=True)
    for command in (prepare, verify):
        command.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("Keep previous evidence; output already exists")
    root = args.root.resolve() if args.command == "prepare" else Path(json.loads(args.prepared.read_text())["root"])
    target = args.target_dir.resolve() if args.command == "prepare" else Path(json.loads(args.prepared.read_text())["target_dir"])
    if args.output.resolve().is_relative_to(root) or args.output.resolve().is_relative_to(target):
        parser.error("Keep provenance evidence outside the source tree and shared target")
    if args.command == "prepare":
        result = prepare_build(args.root, args.target_dir, json.loads(args.metadata.read_text()),
                               {"package": args.package, "name": args.name,
                                "kind": [args.kind], "test": args.test}, args.tool,
                               [json.loads(p.read_text()) for p in args.prior], args.apply_refresh)
    else:
        result = verify_build(json.loads(args.prepared.read_text()), read_messages(args.artifacts))
    with args.output.open("x") as stream:
        json.dump(result, stream, indent=2, sort_keys=True)
        stream.write("\n")


if __name__ == "__main__":
    main()
