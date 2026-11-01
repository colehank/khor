"""Control tests with synthetic Cargo reports, never a Rust compilation claim."""

import importlib.util
import contextlib
import io
import json
import os
import shutil
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "native_provenance", Path(__file__).resolve().parents[1] / "native_provenance.py")
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name).resolve()
        self.root = self.base / "source"
        self.target = self.base / "target"
        self.root.mkdir()
        self.target.mkdir()
        self.put("Cargo.toml", "[workspace]\nmembers=['dep','app']\n")
        self.put("Cargo.lock", "version=4\n")
        self.metadata = {"version": 1, "workspace_root": str(self.root),
                         "workspace_members": ["dep", "app"], "packages": [],
                         "resolve": {"nodes": [{"id": "dep", "dependencies": []},
                                               {"id": "app", "dependencies": ["dep"]}]}}
        for name in ("dep", "app"):
            self.put(f"{name}/Cargo.toml", f"[package]\nname='{name}'\nversion='0.1.0'\n")
            self.put(f"{name}/src/lib.rs", f"pub fn {name}() {{}}\n")
            target = {"name": name, "kind": ["lib"], "edition": "2024",
                      "src_path": str(self.root / name / "src/lib.rs")}
            self.metadata["packages"].append(
                {"id": name, "name": name, "manifest_path": str(self.root / name / "Cargo.toml"),
                 "targets": [target]})
        self.tools = [self.base / "cargo", self.base / "rustc"]
        for tool in self.tools:
            tool.write_text("synthetic tool identity: " + tool.name)
        self.request = {"package": "app", "name": "app", "kind": ["lib"], "test": True}
        # Only the git inventory transport is mocked. All source/artifact/dep-info,
        # additions, timestamps and refresh effects below are real temp files.
        def inventory(*args, **kwargs):
            return "\0".join(str(f.relative_to(self.root)) for f in self.root.rglob("*")
                              if f.is_file() or f.is_symlink()).encode() + b"\0"
        self.addCleanup(patch.stopall)
        patch.object(p.subprocess, "check_output", side_effect=inventory).start()
        patch.dict(os.environ, {"CARGO_HOME": str(self.base / "cargo-home")}, clear=True).start()

    def put(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(data)
        return path

    def prepare(self, prior=(), refresh=False):
        return p.prepare_build(self.root, self.target, self.metadata, self.request,
                               self.tools, prior, refresh)

    def rows(self, fresh=False):
        rows = []
        for package in self.metadata["packages"]:
            for target in package["targets"]:
                name = target["name"]
                test = (package["id"] == "app" and self.request["test"]
                        and target["kind"] != ["custom-build"])
                directory = self.target / "debug/deps"
                directory.mkdir(parents=True, exist_ok=True)
                if target["kind"] == ["custom-build"]:
                    directory = self.target / "debug/build" / (package["id"] + "-0000000000000000")
                    directory.mkdir(parents=True, exist_ok=True)
                    executable = directory / "build-script-build"
                    paired = directory / "build_script_build-1111111111111111"
                    paired.write_bytes(b"synthetic build script")
                    executable.write_bytes(paired.read_bytes())
                    stamp = paired.stat().st_mtime_ns
                    os.utime(executable, ns=(stamp, stamp))
                    file = paired
                    published = executable
                else:
                    file = directory / (name + "-1111111111111111" if test or target["kind"] == ["bin"]
                                        else "lib" + name + "-1111111111111111.rlib")
                    file.write_bytes(b"synthetic artifact " + name.encode())
                    published = file
                dep = file.with_suffix(".d")
                if not test and target["kind"] not in (["custom-build"], ["bin"]):
                    dep = file.with_name(file.stem.removeprefix("lib") + ".d")
                dep.write_text(f"{file}: {target['src_path']}\n{dep}: {target['src_path']}\n")
                rows.append({"reason": "compiler-artifact", "package_id": package["id"],
                             "manifest_path": package["manifest_path"], "target": target,
                             "profile": {"test": test, "opt_level": "0"}, "features": [],
                             "filenames": [str(published)],
                             "executable": str(published) if test or target["kind"] == ["bin"] else None,
                             "fresh": fresh})
        return rows + [{"reason": "build-finished", "success": True}]

    def initial(self):
        prepared = self.prepare()
        rows = self.rows()
        return p.verify_build(prepared, rows), rows

    def test_new_build_then_same_source_reuse_without_refresh(self):
        receipt, rows = self.initial()
        self.put("README.md", "unrelated documentation must not rebuild every package\n")
        prepared = self.prepare([receipt])
        self.assertEqual(prepared["refresh_plan"], [])
        for row in rows[:-1]:
            row["fresh"] = True
        reused = p.verify_build(prepared, rows)
        self.assertTrue(all(u["reuse"] for u in reused["units"]))
        p.require_provenance(reused, self.root, rows[1])

    def add_web(self):
        self.put("web/Cargo.toml", "[package]\nname='khor-web'\nversion='0.1.0'\n")
        source = self.put("web/src/lib.rs", "pub fn web() {}\n")
        self.metadata["workspace_members"].append("web")
        self.metadata["packages"].append(
            {"id": "web", "name": "khor-web", "manifest_path": str(self.root / "web/Cargo.toml"),
             "targets": [{"name": "web", "kind": ["lib"], "edition": "2024",
                          "src_path": str(source)}]})
        self.metadata["resolve"]["nodes"].append({"id": "web", "dependencies": ["dep"]})

    def test_unselected_web_missing_dist_allows_app_and_node_closure_and_reuse(self):
        self.add_web()
        self.metadata["packages"][0]["name"] = "khor-node"
        self.put("dep/Cargo.toml", "[package]\nname='khor-node'\nversion='0.1.0'\n[lib]\nname='dep'\n")
        self.put("apps/agent/providers.json", '{"provider":"fixture"}\n')
        self.put("scripts/install.sh", "#!/bin/sh\n")
        self.assertFalse((self.root / "apps/gui/dist").exists())
        for request in (self.request.copy(),
                        {"package": "khor-node", "name": "dep", "kind": ["lib"], "test": False}):
            with self.subTest(package=request["package"]):
                self.request = request
                prepared = self.prepare()
                self.assertNotIn("web", prepared["states"])
                self.assertEqual(set(prepared["states"]), set(prepared["selected"]))
                self.assertEqual(set(prepared["package_keys"]), set(prepared["selected"]))
                self.assertIn("web/src/lib.rs", prepared["source"])
                rows = [row for row in self.rows()
                        if row.get("package_id") in prepared["selected"]
                        or row["reason"] == "build-finished"]
                receipt = p.verify_build(prepared, rows)
                self.assertNotIn("apps/gui/dist", receipt["extra_inputs"])
                reused = self.prepare([receipt])
                self.assertEqual(reused["refresh_plan"], [])
                for row in rows[:-1]:
                    row["fresh"] = True
                verified = p.verify_build(reused, rows)
                self.assertTrue(all(unit["reuse"] for unit in verified["units"]))
                if request["test"]:
                    p.require_provenance(verified, self.root, verified["selected"]["row"])

    def test_selected_web_still_requires_dist_during_prepare_and_verify(self):
        self.add_web()
        self.request = {"package": "khor-web", "name": "web", "kind": ["lib"], "test": False}
        with self.assertRaises(FileNotFoundError) as missing:
            self.prepare()
        self.assertEqual(Path(missing.exception.filename), self.root / "apps/gui/dist")
        dist = self.put("apps/gui/dist/index.html", "current frontend\n")
        prepared = self.prepare()
        rows = [row for row in self.rows()
                if row.get("package_id") in prepared["selected"] or row["reason"] == "build-finished"]
        dist.unlink()
        dist.parent.rmdir()
        with self.assertRaises(FileNotFoundError):
            p.verify_build(prepared, rows)

    def test_prepare_verify_cli_round_trip_and_preserve_previous_receipts(self):
        metadata = self.base / "metadata.json"
        metadata.write_text(json.dumps(self.metadata))
        prepared = self.base / "prepared.json"
        args = ["native_provenance.py", "prepare", "--root", str(self.root),
                "--target-dir", str(self.target), "--metadata", str(metadata),
                "--package", "app", "--name", "app", "--kind", "lib", "--test",
                "--output", str(prepared)]
        for tool in self.tools:
            args.extend(["--tool", str(tool)])
        with patch("sys.argv", args):
            p.main()
        rows = self.rows()
        log = self.base / "artifacts.json"
        log.write_text("\n".join(json.dumps(row) for row in rows))
        receipt = self.base / "receipt.json"
        with patch("sys.argv", ["native_provenance.py", "verify", "--prepared", str(prepared),
                                 "--artifacts", str(log), "--output", str(receipt)]):
            p.main()
            before = receipt.read_bytes()
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                p.main()
            self.assertEqual(receipt.read_bytes(), before)
        p.require_provenance(json.loads(before), self.root, rows[1])

    def test_changed_dependency_with_preserved_mtime_rejects_old_api_artifacts(self):
        receipt, rows = self.initial()
        path = self.root / "dep/src/lib.rs"
        stamp = path.stat()
        path.write_text("pub fn new_api() {}\n")
        os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        prepared = self.prepare([receipt])
        self.assertEqual(prepared["refresh_plan"], ["app/src/lib.rs", "dep/src/lib.rs"])
        for row in rows[:-1]:
            row["fresh"] = True
        with self.assertRaisesRegex(ValueError, "lacks a matching producer"):
            p.verify_build(prepared, rows)

    def test_added_module_not_listed_in_old_depfile_invalidates_reuse(self):
        receipt, rows = self.initial()
        self.put("dep/src/new_module.rs", "pub struct NewApi;\n")
        prepared = self.prepare([receipt])
        rows[0]["fresh"] = True
        with self.assertRaisesRegex(ValueError, "lacks a matching producer"):
            p.verify_build(prepared, rows)

    def test_change_after_prepare_and_lock_or_environment_change_are_rejected(self):
        for change in (lambda: self.put("app/src/lib.rs", "changed"),
                       lambda: self.put("Cargo.lock", "changed"),
                       lambda: os.environ.update(RUSTFLAGS="--cfg changed")):
            prepared = self.prepare()
            rows = self.rows()
            change()
            with self.assertRaisesRegex(ValueError, "inputs changed"):
                p.verify_build(prepared, rows)

    def test_tauri_environment_records_unset_and_hashes_values(self):
        names = ("TAURI_CONFIG", "REMOVE_UNUSED_COMMANDS")
        absent = p.environment()
        for name in names:
            with self.subTest(name=name):
                self.assertEqual(absent[name], p.key(None))
                os.environ[name] = "private config value"
                present = p.environment()
                self.assertEqual(present[name], p.key("private config value"))
                self.assertNotIn("private config value", json.dumps(present))
                os.environ.pop(name)
                self.assertEqual(p.environment(), absent)

    def test_tauri_environment_set_change_and_remove_invalidate_producer_keys(self):
        for name in ("TAURI_CONFIG", "REMOVE_UNUSED_COMMANDS"):
            for previous, current in ((None, "A"), ("A", "B"), ("A", None)):
                with self.subTest(name=name, previous=previous, current=current):
                    if previous is None:
                        os.environ.pop(name, None)
                    else:
                        os.environ[name] = previous
                    prepared = self.prepare()
                    rows = self.rows()
                    if current is None:
                        os.environ.pop(name, None)
                    else:
                        os.environ[name] = current
                    changed = self.prepare()
                    self.assertNotEqual(prepared["package_keys"], changed["package_keys"])
                    with self.assertRaisesRegex(ValueError, "inputs changed"):
                        p.verify_build(prepared, rows)
                    self.assertEqual(changed["package_keys"], self.prepare()["package_keys"])
            os.environ.pop(name, None)

    def test_wrong_artifact_or_unknown_depfile_source_cannot_reach_native_runner(self):
        receipt, rows = self.initial()
        Path(rows[1]["executable"]).write_bytes(b"other tree binary")
        with self.assertRaisesRegex(ValueError, "artifact does not match"):
            p.require_provenance(receipt, self.root, rows[1])
        prepared = self.prepare()
        rows = self.rows()
        file = Path(rows[1]["executable"])
        file.with_suffix(".d").write_text(
            f"{file}: {self.root}/app/src/lib.rs /unknown/source.rs\n{file.with_suffix('.d')}:\n")
        with self.assertRaisesRegex(ValueError, "not a frozen build input"):
            p.verify_build(prepared, rows)

    def test_legitimate_intermediate_overwrite_does_not_invalidate_linked_executable(self):
        receipt, rows = self.initial()
        Path(rows[0]["filenames"][0]).write_bytes(b"another legitimate cache publication")
        p.require_provenance(receipt, self.root, rows[1])
        prepared = self.prepare([receipt])
        self.assertIn("dep/src/lib.rs", prepared["refresh_plan"])
        rows[0]["fresh"] = True
        with self.assertRaisesRegex(ValueError, "lacks a matching producer"):
            p.verify_build(prepared, rows)

    def test_refresh_only_first_party_entries_preserves_target_and_file_bytes(self):
        cached = self.target / ".fingerprint/third-party"
        cached.parent.mkdir()
        cached.write_bytes(b"untouched cache")
        before = p.regular(cached)
        prepared = self.prepare(refresh=True)
        self.assertEqual(len(prepared["refreshed"]), 2)
        self.assertTrue(all(x["after"]["mtime_ns"] > x["before"]["mtime_ns"]
                            and x["after"]["sha256"] == x["before"]["sha256"]
                            for x in prepared["refreshed"]))
        self.assertEqual(p.regular(cached), before)

    def test_normal_binary_uses_hashed_pair_and_not_stale_top_depfile(self):
        self.request.update(kind=["bin"], test=False)
        target = self.metadata["packages"][1]["targets"][0]
        target.update(kind=["bin"], src_path=str(self.put("app/src/main.rs", "fn main() {}\n")))
        prepared = self.prepare()
        rows = self.rows()
        row = rows[1]
        hashed = Path(row["executable"])
        normal = self.target / "debug/app"
        normal.write_bytes(hashed.read_bytes())
        stamp = hashed.stat().st_mtime_ns
        os.utime(normal, ns=(stamp, stamp))
        (normal.parent / "app.d").write_text("old: /another/tree/src/main.rs\n")
        row.update(filenames=[str(normal)], executable=str(normal))
        receipt = p.verify_build(prepared, rows)
        self.assertIn(str(hashed.with_suffix(".d")), receipt["selected"]["files"])
        self.assertNotIn(str(normal.parent / "app.d"), receipt["selected"]["files"])
        dep = hashed.with_suffix(".d")
        original = dep.read_text()
        dep.write_text(f"old-summary: {row['target']['src_path']}\n")
        with self.assertRaisesRegex(ValueError, "paired rustc dep-info"):
            p.verify_build(prepared, rows)
        dep.write_text(original)
        hashed.write_bytes(b"same path, different bytes")
        with self.assertRaisesRegex(ValueError, "content-paired"):
            p.verify_build(prepared, rows)

    def test_depfile_make_escaping_relative_sources_and_continuation(self):
        source = self.put("app/src/a #$\\b.rs", "test")
        dep = self.base / "escaped.d"
        escaped = "app/src/a\\ \\#$$\\\\b.rs"
        dep.write_text(f"out\\ name {dep}: \\\n {escaped}\n")
        lhs, rhs = p.depinfo(dep, self.root)
        self.assertEqual(rhs, {str(source)})
        self.assertIn(str(self.root / "out name"), lhs)
        dep.write_text("out: $(UNPROVEN)\n")
        with self.assertRaisesRegex(ValueError, "Unsupported Make expression"):
            p.depinfo(dep, self.root)

    def test_hyphenated_binary_uses_underscore_rustc_crate_pair(self):
        name = "khor-quality-provenance-probe"
        self.request.update(name=name, kind=["bin"], test=False)
        target = self.metadata["packages"][1]["targets"][0]
        target.update(name=name, kind=["bin"],
                      src_path=str(self.put("app/src/main.rs", "fn main() {}\n")))
        prepared = self.prepare()
        rows = self.rows()
        row = rows[1]
        original = Path(row["executable"])
        hashed = original.with_name("khor_quality_provenance_probe-1111111111111111")
        original.rename(hashed)
        original.with_suffix(".d").unlink()
        dep = hashed.with_suffix(".d")
        dep.write_text(f"{hashed}: {target['src_path']}\n{dep}: {target['src_path']}\n")
        published = self.target / "debug" / name
        published.write_bytes(hashed.read_bytes())
        stamp = hashed.stat().st_mtime_ns
        os.utime(published, ns=(stamp, stamp))
        row.update(filenames=[str(published)], executable=str(published))
        receipt = p.verify_build(prepared, rows)
        self.assertIn(str(hashed), receipt["selected"]["files"])
        self.assertEqual(receipt["selected"]["depinfo"]["path"], str(dep))
        hashed.write_bytes(b"wrong binary at the normalized crate path")
        with self.assertRaisesRegex(ValueError, "content-paired"):
            p.verify_build(prepared, rows)

    def add_generator(self):
        path = self.put("dep/build.rs", "fn main() {}\n")
        self.metadata["packages"][0]["targets"].append(
            {"name": "build-script-build", "kind": ["custom-build"],
             "edition": "2024", "src_path": str(path)})
        return self.put("dep/input.toml", 'key="old"\n')

    def generated_row(self, prepared, current):
        out = self.target / "debug/build/dep-output/out"
        out.mkdir(parents=True, exist_ok=True)
        (out / "generated.rs").write_text('const KEY: &str = "old";\n')
        marker = out.parent / "output"
        marker.write_text("cargo::rerun-if-changed=input.toml\n")
        if not current:
            for path in (marker, out / "generated.rs"):
                os.utime(path, ns=(1, 1))
        return {"reason": "build-script-executed", "package_id": "dep", "out_dir": str(out)}

    def test_cached_build_script_event_is_not_current_generation_proof(self):
        self.add_generator()
        prepared = self.prepare()
        rows = self.rows()
        generated = self.generated_row(prepared, current=False)
        with self.assertRaisesRegex(ValueError, "no current invocation"):
            p.verify_build(prepared, [generated, *rows])
        generated = self.generated_row(prepared, current=True)
        receipt = p.verify_build(prepared, [generated, *rows])
        reused = self.prepare([receipt])
        for row in rows[:-1]:
            row["fresh"] = True
        self.assertEqual(len(p.verify_build(reused, [generated, *rows])["generated"]), 1)

    def library_row(self):
        directory = self.target / "debug/deps"
        directory.mkdir(parents=True)
        files = [directory / ("libapp" + suffix) for suffix in (".a", ".dylib", ".rlib")]
        for path in files:
            path.write_text("synthetic library: " + path.suffix)
        dep = directory / "app.d"
        dep.write_text("".join(f"{path}: {self.root / 'app/src/lib.rs'}\n" for path in [dep, *files]))
        return {"filenames": [str(path) for path in files], "executable": None,
                "target": {"name": "app", "kind": ["staticlib", "cdylib", "rlib"],
                           "src_path": str(self.root / "app/src/lib.rs")}}, dep

    def test_static_and_dynamic_library_row_uses_the_rustc_crate_depinfo(self):
        row, dep = self.library_row()
        for primary in row["filenames"][:2]:
            with self.subTest(primary=primary):
                current = {**row, "filenames": [primary, *(name for name in row["filenames"] if name != primary)]}
                files, paired = p.artifact_files(current, self.root, self.target)
                self.assertEqual(paired["path"], str(dep))
                self.assertEqual(set(files), {*row["filenames"], str(dep)})
                self.assertTrue(set(row["filenames"]).issubset(paired["targets"]))

    def test_library_depinfo_must_name_the_reported_primary_artifact(self):
        row, dep = self.library_row()
        dep.write_text(f"{dep}: {self.root / 'app/src/lib.rs'}\n")
        with self.assertRaisesRegex(ValueError, "Missing unique paired"):
            p.artifact_files(row, self.root, self.target)

    def test_current_generation_copy_preserves_old_mtime(self):
        self.add_generator()
        source = self.base / "template.rs"
        source.write_text('const KEY: &str = "copied";\n')
        os.utime(source, ns=(1, 1))
        prepared = self.prepare()
        generated = self.generated_row(prepared, current=True)
        copied = Path(generated["out_dir"]) / "generated.rs"
        shutil.copy2(source, copied)
        self.assertEqual(copied.stat().st_mtime_ns, 1)
        self.assertGreaterEqual(copied.stat().st_ctime_ns, prepared["prepared_ns"])
        receipt = p.verify_build(prepared, [generated, *self.rows()])
        stamp = receipt["generated"][0]["files"][str(copied)]
        self.assertEqual(stamp["sha256"], p.digest(source))
        self.assertEqual(stamp["mtime_ns"], 1)
        self.assertEqual(stamp["ctime_ns"], copied.stat().st_ctime_ns)

    def test_current_marker_does_not_admit_unchanged_old_generated_file(self):
        self.add_generator()
        generated = self.generated_row(self.prepare(), current=True)
        file = Path(generated["out_dir"]) / "generated.rs"
        before = file.stat()
        prepared = self.prepare()
        marker = file.parent.parent / "output"
        marker.write_text("cargo::rerun-if-changed=input.toml\n")
        self.assertLess(before.st_mtime_ns, prepared["prepared_ns"])
        self.assertLess(before.st_ctime_ns, prepared["prepared_ns"])
        with self.assertRaisesRegex(ValueError, "no current invocation"):
            p.verify_build(prepared, [generated, *self.rows()])

    def test_current_generated_file_does_not_admit_an_old_marker(self):
        self.add_generator()
        prepared = self.prepare()
        generated = self.generated_row(prepared, current=True)
        marker = Path(generated["out_dir"]).parent / "output"
        os.utime(marker, ns=(1, 1))
        with self.assertRaisesRegex(ValueError, "no current invocation"):
            p.verify_build(prepared, [generated, *self.rows()])

    def test_compiler_artifact_still_requires_current_mtime(self):
        prepared = self.prepare()
        rows = self.rows()
        artifact = Path(rows[0]["filenames"][0])
        os.utime(artifact, ns=(1, 1))
        self.assertGreaterEqual(artifact.stat().st_ctime_ns, prepared["prepared_ns"])
        with self.assertRaisesRegex(ValueError, "artifact predates"):
            p.verify_build(prepared, rows)

    def test_returning_to_A_checks_each_target_and_generated_output(self):
        source = self.add_generator()
        original, stamp = source.read_bytes(), source.stat()
        prepared_a = self.prepare()
        rows_a = self.rows()
        generated = self.generated_row(prepared_a, current=True)
        receipt_a = p.verify_build(prepared_a, [generated, *rows_a])
        source.write_text('key="B"\n')
        os.utime(source, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        prepared_b = self.prepare([receipt_a])
        rows_b = self.rows()
        for row in rows_b[:-1]:
            if row["target"]["kind"] != ["custom-build"]:
                Path(row["filenames"][0]).write_bytes(b"synthetic B artifact")
        generated = self.generated_row(prepared_b, current=True)
        (Path(generated["out_dir"]) / "generated.rs").write_text('const KEY: &str = "B";\n')
        receipt_b = p.verify_build(prepared_b, [generated, *rows_b])
        self.assertEqual(self.prepare([receipt_a, receipt_b])["refresh_plan"], [])

        # Restore A's exact bytes and old timestamp while B remains in OUT_DIR.
        # The script executable is deliberately byte-identical in both states.
        source.write_bytes(original)
        os.utime(source, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        script_a = next(u for u in receipt_a["units"] if u["variant"]["kind"] == ["custom-build"])
        self.assertTrue(p.same_files(script_a["files"]))
        returned_a = self.prepare([receipt_a, receipt_b])
        self.assertEqual(returned_a["refresh_plan"],
                         ["app/src/lib.rs", "dep/build.rs", "dep/src/lib.rs"])
        rows_a = self.rows()
        for row in rows_a[:-1]:
            if row["target"]["kind"] == ["custom-build"]:
                row["fresh"] = True
        # This matches the real failure: script fresh, dep/app newly compiled,
        # and stale B generation. Verification must still reject it.
        with self.assertRaisesRegex(ValueError, "no current invocation"):
            p.verify_build(returned_a, [generated, *rows_a])
        generated = self.generated_row(returned_a, current=True)
        for row in rows_a[:-1]:
            row["fresh"] = False
        receipt_a2 = p.verify_build(returned_a, [generated, *rows_a])
        self.assertEqual(self.prepare([receipt_a, receipt_b, receipt_a2])["refresh_plan"], [])

    def test_changed_generated_input_and_removed_key_cannot_reuse_old_output(self):
        source = self.add_generator()
        prepared = self.prepare()
        rows = self.rows()
        generated = self.generated_row(prepared, current=True)
        receipt = p.verify_build(prepared, [generated, *rows])
        stamp = source.stat()
        source.write_text('other="new"\n')
        os.utime(source, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        prepared = self.prepare([receipt])
        for row in rows[:-1]:
            row["fresh"] = True
        with self.assertRaisesRegex(ValueError, "no current invocation"):
            p.verify_build(prepared, [generated, *rows])

    def test_tauri_build_script_environment_is_known_even_when_unset(self):
        self.add_generator()
        prepared = self.prepare()
        rows = self.rows()
        generated = self.generated_row(prepared, current=True)
        marker = Path(generated["out_dir"]).parent / "output"
        marker.write_text("cargo::rerun-if-env-changed=TAURI_CONFIG\n"
                          "cargo:rerun-if-env-changed=REMOVE_UNUSED_COMMANDS\n")
        receipt = p.verify_build(prepared, [generated, *rows])
        self.assertEqual(len(receipt["generated"]), 1)

    def gui_capabilities(self):
        package = self.metadata["packages"][1]
        previous = self.root / "app"
        directory = self.root / "apps/gui/src-tauri"
        directory.parent.mkdir(parents=True)
        previous.rename(directory)
        package.update(name="khor-gui", manifest_path=str(directory / "Cargo.toml"))
        package["targets"][0]["src_path"] = str(directory / "src/lib.rs")
        (directory / "Cargo.toml").write_text("[package]\nname='khor-gui'\nversion='0.1.0'\n[lib]\nname='app'\n")
        self.request["package"] = "khor-gui"
        return self.put("apps/gui/src-tauri/capabilities/default.json", '{"permissions":["A"]}\n').parent

    def test_gui_capabilities_directory_is_frozen_and_admitted_as_declared(self):
        directory = self.gui_capabilities()
        script = self.put("apps/gui/src-tauri/build.rs", "fn main() {}\n")
        self.metadata["packages"][1]["targets"].append(
            {"name": "build-script-build", "kind": ["custom-build"],
             "edition": "2024", "src_path": str(script)})
        prepared = self.prepare()
        out = self.target / "debug/build/app-output/out"
        out.mkdir(parents=True)
        (out / "permissions.rs").write_text("const PERMISSIONS: &[&str] = &[];\n")
        marker = out.parent / "output"
        marker.write_text(f"cargo::rerun-if-changed={directory}\n")
        generated = {"reason": "build-script-executed", "package_id": "app", "out_dir": str(out)}
        receipt = p.verify_build(prepared, [generated, *self.rows()])
        self.assertEqual(len(receipt["generated"]), 1)
        self.assertEqual(prepared["states"]["app"]["extra_inputs"]["apps/gui/src-tauri/capabilities"],
                         {"default.json": p.digest(directory / "default.json")})
        marker.write_text(f"cargo::rerun-if-changed={self.root}\n")
        with self.assertRaisesRegex(ValueError, "outside its producer key"):
            p.verify_build(prepared, [generated, *self.rows()])

    def test_gui_capabilities_set_change_and_remove_invalidate_the_producer_key(self):
        directory = self.gui_capabilities()
        source = directory / "default.json"
        original = source.read_bytes()
        before = self.prepare()
        stamp = source.stat()
        source.write_bytes(original + b" ")
        os.utime(source, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        changed = self.prepare()
        self.assertNotEqual(before["package_keys"], changed["package_keys"])
        source.write_bytes(original)
        self.assertEqual(before["package_keys"], self.prepare()["package_keys"])
        extra = directory / "new.json"
        extra.write_text("{}\n")
        added = self.prepare()
        self.assertNotEqual(before["package_keys"], added["package_keys"])
        extra.unlink()
        self.assertEqual(before["package_keys"], self.prepare()["package_keys"])
        source.unlink()
        self.assertNotEqual(before["package_keys"], self.prepare()["package_keys"])

    def test_gui_capabilities_symlink_root_and_member_are_rejected(self):
        directory = self.gui_capabilities()
        source = directory / "default.json"
        real = source.with_name("real.json")
        source.rename(real)
        source.symlink_to(real.name)
        with self.assertRaisesRegex(ValueError, "nonregular|regular.*symlink"):
            self.prepare()
        source.unlink()
        real.rename(source)
        moved = directory.with_name("real-capabilities")
        directory.rename(moved)
        directory.symlink_to(moved.name, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "nonregular|regular.*symlink"):
            self.prepare()

    def test_build_script_unrecorded_input_is_rejected(self):
        self.add_generator()
        prepared = self.prepare()
        rows = self.rows()
        generated = self.generated_row(prepared, current=True)
        marker = Path(generated["out_dir"]).parent / "output"
        for instruction in ("cargo::rerun-if-env-changed=UNRECORDED_INPUT\n",
                            "cargo::rerun-if-changed=/unknown/input\n"):
            marker.write_text(instruction)
            with self.assertRaisesRegex(ValueError, "unrecorded|unfrozen"):
                p.verify_build(prepared, [generated, *rows])

    def test_root_generated_input_preserving_mtime_fails_closed(self):
        self.add_generator()
        shared = self.put("shared-input.toml", 'value="A"\n')
        prepared = self.prepare()
        rows = self.rows()
        generated = self.generated_row(prepared, current=True)
        marker = Path(generated["out_dir"]).parent / "output"
        marker.write_text("cargo::rerun-if-changed=../shared-input.toml\n")
        with self.assertRaisesRegex(ValueError, "outside its producer key"):
            p.verify_build(prepared, [generated, *rows])
        stamp = shared.stat()
        shared.write_text('value="B"\n')
        os.utime(shared, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        changed = self.prepare()
        self.assertEqual(prepared["package_keys"], changed["package_keys"])
        self.assertEqual(shared.stat().st_mtime_ns, stamp.st_mtime_ns)
        for row in rows[:-1]:
            row["fresh"] = True
        with self.assertRaisesRegex(ValueError, "outside its producer key"):
            p.verify_build(changed, [generated, *rows])

    def test_root_rust_include_input_preserving_mtime_fails_closed(self):
        shared = self.put("shared-input.txt", "A\n")
        self.put("app/src/lib.rs", 'pub const SHARED: &str = include_str!("../../shared-input.txt");\n')
        prepared = self.prepare()
        dependency = p.prepare_build(
            self.root, self.target, self.metadata,
            {"package": "dep", "name": "dep", "kind": ["lib"], "test": False}, self.tools)
        rows = self.rows()
        dependency_receipt = p.verify_build(dependency, [rows[0], rows[-1]])
        executable = Path(rows[1]["executable"])
        dep = executable.with_suffix(".d")
        dep.write_text(f"{executable}: {rows[1]['target']['src_path']} {shared}\n{dep}:\n")
        with self.assertRaisesRegex(ValueError, "covered by its producer key"):
            p.verify_build(prepared, rows)
        stamp = shared.stat()
        shared.write_text("B\n")
        os.utime(shared, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
        changed = self.prepare([dependency_receipt])
        self.assertEqual(prepared["package_keys"], changed["package_keys"])
        self.assertEqual(shared.stat().st_mtime_ns, stamp.st_mtime_ns)
        for row in rows[:-1]:
            row["fresh"] = True
        with self.assertRaisesRegex(ValueError, "covered by its producer key"):
            p.verify_build(changed, rows)

    def test_node_known_root_includes_are_keyed_and_each_invalidates_with_old_mtime(self):
        self.metadata["packages"][0]["name"] = "khor-node"
        self.put("dep/Cargo.toml", "[package]\nname='khor-node'\nversion='0.1.0'\n[lib]\nname='dep'\n")
        providers = self.put("apps/agent/providers.json", '{"provider":"A"}\n')
        install = self.put("scripts/install.sh", "#!/bin/sh\nprintf A\n")
        self.put("dep/src/lib.rs", 'pub const PROVIDERS: &str = include_str!("../../apps/agent/providers.json");\n'
                 'pub const INSTALL: &str = include_str!("../../scripts/install.sh");\n')
        prepared = self.prepare()
        rows = self.rows()
        library = Path(rows[0]["filenames"][0])
        dep = library.with_name(library.stem.removeprefix("lib") + ".d")
        dep.write_text(f"{library}: {rows[0]['target']['src_path']} {providers} {install}\n{dep}:\n")
        receipt = p.verify_build(prepared, rows)
        self.assertEqual(set(receipt["extra_inputs"]),
                         {"apps/agent/providers.json", "scripts/install.sh"})
        p.require_provenance(receipt, self.root, rows[1])
        originals = {path: (path.read_bytes(), path.stat()) for path in (providers, install)}
        for path in (providers, install):
            with self.subTest(input=str(path.relative_to(self.root))):
                for item, (data, stamp) in originals.items():
                    item.write_bytes(data)
                    os.utime(item, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
                data, stamp = originals[path]
                path.write_bytes(data + b"changed without newer timestamp\n")
                os.utime(path, ns=(stamp.st_atime_ns, stamp.st_mtime_ns))
                changed = self.prepare([receipt])
                self.assertEqual(changed["refresh_plan"], ["app/src/lib.rs", "dep/src/lib.rs"])
                self.assertEqual(path.stat().st_mtime_ns, stamp.st_mtime_ns)
                for row in rows[:-1]:
                    row["fresh"] = True
                with self.assertRaisesRegex(ValueError, "lacks a matching producer"):
                    p.verify_build(changed, rows)
                with self.assertRaisesRegex(ValueError, "source frame"):
                    p.require_provenance(receipt, self.root, rows[1])

    def test_different_embedded_manifest_root_does_not_reuse_identical_bytes(self):
        receipt, rows = self.initial()
        self.root.rename(self.base / "old-source")
        old = self.root
        self.root = self.base / "new-source"
        import shutil
        shutil.copytree(self.base / "old-source", self.root)
        self.metadata = json.loads(json.dumps(self.metadata).replace(str(old), str(self.root)))
        prepared = self.prepare([receipt])
        rows = self.rows(fresh=True)
        with self.assertRaisesRegex(ValueError, "lacks a matching producer"):
            p.verify_build(prepared, rows)

    def test_failed_or_zero_artifact_build_never_produces_receipt(self):
        prepared = self.prepare()
        with self.assertRaisesRegex(ValueError, "successful actual Cargo build"):
            p.verify_build(prepared, [{"reason": "build-finished", "success": False}])
        with self.assertRaisesRegex(ValueError, "prepared build graph"):
            p.verify_build(prepared, [{"reason": "build-finished", "success": True}])


if __name__ == "__main__":
    unittest.main()
