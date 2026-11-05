"""Run real scratch SSH installation using a temporary signing root, never release credentials."""
from contextlib import contextmanager
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

SOURCE = Path(__file__).resolve().parent.parent


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


@contextmanager
def temporary_trust(public, directory, signer, environment):
    key = directory / "fixture.key"
    original = public.read_bytes()
    subprocess.run([str(signer), "signer", "generate", "--ci", "--password", "public-ssh-fixture",
                    "--write-keys", str(key)], env=environment, check=True, stdout=subprocess.DEVNULL)
    try:
        public.write_bytes(key.with_suffix(".key.pub").read_bytes())
        yield key
    finally:
        public.write_bytes(original)


def executable(messages, name, kind):
    rows = [row for row in messages if row.get("reason") == "compiler-artifact"
            and row.get("target", {}).get("name") == name and row["target"]["kind"] == [kind]
            and row.get("profile", {}).get("test") is (kind == "test")
            and row.get("executable") and Path(row["manifest_path"]).resolve() == SOURCE / "crates/cli/Cargo.toml"]
    if len(rows) != 1:
        raise RuntimeError(f"expected one actual CLI {kind} artifact: {name}")
    program = Path(rows[0]["executable"])
    if program.is_symlink() or not program.is_file():
        raise RuntimeError("fixture executable is absent or a symlink")
    return program


def sign(signer, key, path, environment):
    subprocess.run([str(signer), "signer", "sign", "--private-key-path", str(key),
                    "--password", "public-ssh-fixture", str(path)], env=environment,
                   check=True, stdout=subprocess.DEVNULL)
    return Path(str(path) + ".sig").read_text().strip()


def package(program, directory, signer, key, environment):
    info = json.loads(subprocess.check_output([str(program), "_release_info"], env=environment, text=True, timeout=30))
    if info["component"] != "cli" or info["build"]["release"]["tag"] != environment["KHOR_RELEASE_TAG"]:
        raise RuntimeError("SSH fixture binary has a different release identity")
    platform = info["platform"]
    names = {("macos", "aarch64"): "khor-macos-arm64", ("macos", "x86_64"): "khor-macos-x86_64",
             ("linux", "aarch64"): "khor-linux-aarch64", ("linux", "x86_64"): "khor-linux-x86_64"}
    archive = directory / (names[(platform["os"], platform["arch"])] + ".gz")
    with program.open("rb") as source, gzip.open(archive, "wb", compresslevel=1) as output:
        while block := source.read(1024 * 1024):
            output.write(block)
    asset = dict(component="cli", platform=platform, file=archive.name,
                 url=f"https://ssh-fixture.invalid/{environment['KHOR_RELEASE_TAG']}/{archive.name}",
                 bytes=archive.stat().st_size, unpacked_bytes=program.stat().st_size,
                 sha256=digest(archive), signature=sign(signer, key, archive, environment), contract=info["contract"])
    manifest = directory / "khor-release.json"
    manifest.write_text(json.dumps(dict(schema=1, identity=info["build"]["release"],
                                       built_from=environment["KHOR_RELEASE_TAG"], notes="Local SSH test fixture",
                                       artifacts=[asset])))
    sign(signer, key, manifest, environment)
    archive.with_suffix(".gz.sha256").write_text(f"{asset['sha256']}  {archive.name}\n")
    return dict(identity=info["build"]["release"], asset=asset, program_sha256=digest(program))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    output = parser.parse_args().output.resolve()
    if output.is_relative_to(SOURCE):
        parser.error("keep SSH acceptance output outside the source tree")
    output.mkdir(parents=True, exist_ok=True)
    signer = SOURCE / "apps/gui/node_modules/.bin/tauri"
    if not signer.is_file():
        parser.error("install the locked apps/gui dependencies to use the official Tauri signer")
    environment = {name: os.environ[name] for name in os.environ
                   if not name.startswith(("TAURI_SIGNING_", "APPLE_", "IOS_CERTIFICATE", "MAC_CERTIFICATE"))
                   and name not in {"IOS_PROFILE", "GH_TOKEN", "GITHUB_TOKEN"}}
    version = tomllib.loads((SOURCE / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    environment.update(KHOR_RELEASE_TAG=f"v{version}-rc.900101", KHOR_RELEASE_BUILD="900101",
                       KHOR_BUILD_SOURCE=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=SOURCE, text=True).strip())
    public = SOURCE / "crates/update/updater.pub"
    evidence = dict(source=environment["KHOR_BUILD_SOURCE"], signed_with_temporary_fixture=True,
                    published=False, original_public_sha256=digest(public), passed=False)
    try:
        with tempfile.TemporaryDirectory(prefix="khor-signed-ssh-") as temporary:
            directory = Path(temporary)
            with temporary_trust(public, directory, signer, environment) as key:
                (output / "fixture-public-key.txt").write_bytes(public.read_bytes())
                print("Building the real SSH fixture CLI and test with a temporary public root", flush=True)
                artifacts = output / "ssh-fixture-artifacts.json"
                with artifacts.open("w") as stream:
                    subprocess.run(["cargo", "test", "--locked", "-p", "khor-cli", "--test", "add_ssh",
                                    "--no-run", "--message-format=json"], cwd=SOURCE, env=environment, stdout=stream, check=True)
                messages = [json.loads(line) for line in artifacts.read_text().splitlines() if line.startswith("{")]
                program = executable(messages, "khor", "bin")
                harness = executable(messages, "add_ssh", "test")
                evidence.update(package(program, directory, signer, key, environment))
                environment["KHOR_TEST_SSH_RELEASE_DIR"] = str(directory)
                log = output / "ssh-install.log"
                print("Running signed download, upload fallback and SSH authentication cases", flush=True)
                try:
                    with log.open("w") as stream:
                        subprocess.run([str(harness), "--ignored", "--exact", "add_over_ssh_in_a_scratch_world",
                                        "--nocapture", "--test-threads=1"], cwd=SOURCE, env=environment,
                                       stdout=stream, stderr=subprocess.STDOUT, check=True, timeout=900)
                    if "test result: ok. 1 passed; 0 failed; 0 ignored;" not in log.read_text():
                        raise RuntimeError("signed SSH acceptance did not execute its real test")
                    evidence["passed"] = True
                finally:
                    print(log.read_text(errors="replace")[-12000:], flush=True)
    finally:
        evidence["public_root_restored"] = digest(public) == evidence["original_public_sha256"]
        (output / "signed-ssh.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
