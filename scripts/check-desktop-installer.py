"""Install and remove a desktop package on an isolated build runner."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

from release_manifest import executable_info, source_sha


def run(*args):
    return subprocess.run(list(map(str, args)), check=True, capture_output=True, text=True, timeout=600)


def same_programs(directory):
    suffix = ".exe" if os.name == "nt" else ""
    cli, gui = directory / ("khor" + suffix), directory / ("khor-gui" + suffix)
    command, application = executable_info(cli), executable_info(gui)
    assert command["component"] == "cli"
    assert application["component"] == "desktop-app"
    assert all(command[key] == application[key] for key in ("build", "platform", "contract"))
    assert command["build"]["source"] == source_sha()
    upgraded = subprocess.run([str(cli), "upgrade", "--check"], capture_output=True, timeout=30)
    assert upgraded.returncode != 0, "The bundled CLI must follow its package owner"
    assert b"installer" in upgraded.stderr or "安装包" in upgraded.stderr.decode(errors="replace")
    return {"cli": command, "app": application}


def launch_installed(directory):
    with tempfile.TemporaryDirectory(prefix="khor-install-home-") as temporary:
        runtime = Path(temporary) / "runtime"
        environment = {**os.environ, "KHOR_HOME": str(Path(temporary) / "node"),
                       "KHOR_RUNTIME_DIR": str(runtime), "LANG": "en_US.UTF-8"}
        suffix = ".exe" if os.name == "nt" else ""
        application = directory / ("khor-gui" + suffix)

        def processes():
            if os.name == "nt":
                source = run("powershell.exe", "-NoProfile", "-Command",
                             "@(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath } | Select-Object ProcessId,ExecutablePath) | ConvertTo-Json -Compress").stdout
                rows = [(row["ProcessId"], Path(row["ExecutablePath"])) for row in json.loads(source)]
            else:
                rows = []
                for entry in Path("/proc").iterdir():
                    if entry.name.isdecimal():
                        try:
                            rows.append((int(entry.name), (entry / "exe").resolve(strict=True)))
                        except (OSError, RuntimeError):
                            pass
            return [(pid, path) for pid, path in rows if path == application or path.is_relative_to(runtime)]

        try:
            subprocess.run([str(directory / ("khor" + suffix)), "gui"], env=environment,
                           check=True, timeout=30)
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if any(path.name == application.name for _, path in processes()):
                    return
                time.sleep(0.1)
            raise AssertionError("khor gui did not launch the installed application")
        finally:
            for pid, _ in processes():
                if os.name == "nt":
                    subprocess.run(["taskkill.exe", "/PID", str(pid), "/T", "/F"], capture_output=True)
                else:
                    try:
                        os.kill(pid, 15)
                    except ProcessLookupError:
                        pass
            deadline = time.monotonic() + 10
            while processes() and time.monotonic() < deadline:
                time.sleep(0.1)
            assert not processes(), "The installation check must retire its own processes"


def linux(package):
    cli = Path("/usr/bin/khor")
    assert not cli.exists() and not cli.is_symlink(), "Use a fresh runner"
    with tempfile.TemporaryDirectory(prefix="khor-independent-") as temporary:
        independent = Path(temporary) / "khor"
        independent.write_text("independent CLI\n")
        run("sudo", "cp", independent, cli)
        try:
            refused = subprocess.run(["sudo", "dpkg", "-i", str(package)], capture_output=True, timeout=120)
            assert refused.returncode != 0, "An unrelated command cannot be overwritten"
            assert cli.read_bytes() == independent.read_bytes()
        finally:
            run("sudo", "rm", cli)
    run("sudo", "dpkg", "-i", package)
    try:
        proof = same_programs(Path("/usr/bin"))
        assert Path("/usr/share/khor/cli-owner.json").is_file()
        launch_installed(Path("/usr/bin"))
        return proof
    finally:
        run("sudo", "dpkg", "-r", "khor")
        assert not cli.exists()


def windows(package):
    import winreg
    with tempfile.TemporaryDirectory(prefix="khor-desktop-") as temporary:
        destination = Path(temporary) / "Installed Khor"
        independent = Path(temporary) / "Independent CLI"
        independent.mkdir()
        (independent / "khor.cmd").write_text("@echo independent CLI\n")
        with winreg.CreateKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
            try:
                original, kind = winreg.QueryValueEx(key, "Path")
            except FileNotFoundError:
                original, kind = None, winreg.REG_EXPAND_SZ
            before = str(independent) + (";" + original if original else "")
            winreg.SetValueEx(key, "Path", 0, kind, before)
        installed = False
        try:
            result = subprocess.run(["msiexec.exe", "/i", str(package), "/qn", "/norestart",
                                     "INSTALLDIR=" + str(destination)], timeout=600)
            assert result.returncode in (0, 3010), result.returncode
            installed = True
            proof = same_programs(destination)
            assert (destination / "cli-owner.json").is_file()
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                after, _ = winreg.QueryValueEx(key, "Path")
            entries = [value.rstrip("\\").casefold() for value in after.split(";")]
            assert entries[0] == str(independent).casefold(), "Independent CLI keeps its PATH priority"
            assert str(destination).casefold() in entries
            launch_installed(destination)
            return proof
        finally:
            if installed:
                result = subprocess.run(["msiexec.exe", "/x", str(package), "/qn", "/norestart"], timeout=600)
                assert result.returncode in (0, 3010), result.returncode
                assert not (destination / "khor.exe").exists()
                with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                    after, _ = winreg.QueryValueEx(key, "Path")
                assert str(independent) in after and str(destination).casefold() not in after.casefold()
            with winreg.CreateKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                if original is None:
                    winreg.DeleteValue(key, "Path")
                else:
                    winreg.SetValueEx(key, "Path", 0, kind, original)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Installation checks require an isolated GitHub runner")
    package = args.package.resolve(strict=True)
    proof = windows(package) if os.name == "nt" else linux(package)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(proof, indent=2) + "\n")
