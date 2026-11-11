"""Install and remove a desktop package on an isolated build runner."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from release_manifest import executable_info, source_sha


def run(*args):
    return subprocess.run(list(map(str, args)), check=True, capture_output=True, text=True, timeout=600)


def same_programs(directory, expected_source):
    suffix = ".exe" if os.name == "nt" else ""
    cli, gui = directory / ("khor" + suffix), directory / ("khor-gui" + suffix)
    command, application = executable_info(cli), executable_info(gui)
    assert command["component"] == "cli"
    assert application["component"] == "desktop-app"
    assert all(command[key] == application[key] for key in ("build", "platform", "contract"))
    assert command["build"]["source"] == expected_source
    upgraded = subprocess.run([str(cli), "upgrade", "--check"], capture_output=True, timeout=30)
    assert upgraded.returncode != 0, "The bundled CLI must follow its package owner"
    guidance = upgraded.stderr.decode(errors="replace").lower()
    if sys.platform == "darwin":
        assert "settings" in guidance or "设置" in guidance, guidance
    else:
        assert "installer" in guidance or "安装包" in guidance, guidance
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
            elif sys.platform == "darwin":
                lines = run("ps", "-wwax", "-o", "pid=", "-o", "comm=").stdout.splitlines()
                rows = [(int(parts[0]), Path(parts[1])) for line in lines
                        if len(parts := line.strip().split(maxsplit=1)) == 2]
            else:
                rows = []
                for entry in Path("/proc").iterdir():
                    if entry.name.isdecimal():
                        try:
                            rows.append((int(entry.name), (entry / "exe").resolve(strict=True)))
                        except (OSError, RuntimeError):
                            pass
            return [(pid, path) for pid, path in rows if path == application or path.is_relative_to(runtime)]

        service = None
        log = None
        try:
            # The packaged CLI must run a real node, not only print metadata.
            log = (Path(temporary) / "serve.log").open("wb")
            service = subprocess.Popen([str(directory / ("khor" + suffix)), "serve"],
                                       env=environment, stdout=log, stderr=subprocess.STDOUT)
            endpoint = Path(environment["KHOR_HOME"]) / ".khor/endpoint.json"
            deadline = time.monotonic() + 30
            while not endpoint.is_file() and time.monotonic() < deadline:
                if service.poll() is not None:
                    raise AssertionError("Packaged serve exited before publishing its endpoint")
                time.sleep(0.1)
            assert endpoint.is_file(), "The packaged CLI did not start its resident"
            subprocess.run([str(directory / ("khor" + suffix)), "quit"], env=environment,
                           check=True, timeout=30)
            service.wait(timeout=10)
            deadline = time.monotonic() + 10
            while processes() and time.monotonic() < deadline:
                time.sleep(0.1)
            assert not processes(), "Quitting the packaged node must not leave or respawn its resident"
            subprocess.run([str(directory / ("khor" + suffix)), "gui"], env=environment,
                           check=True, timeout=30)
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if any(path.name == application.name for _, path in processes()):
                    time.sleep(1)
                    assert any(path.name == application.name for _, path in processes()), "The installed App exited during startup"
                    return
                time.sleep(0.1)
            raise AssertionError("khor gui did not launch the installed application")
        finally:
            if service is not None and service.poll() is None:
                service.kill()
                service.wait(timeout=10)
            if log is not None:
                log.close()
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


def linux(package, expected_source):
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
        proof = same_programs(Path("/usr/bin"), expected_source)
        assert Path("/usr/share/khor/cli-owner.json").is_file()
        launch_installed(Path("/usr/bin"))
        return proof
    finally:
        run("sudo", "dpkg", "-r", "khor")
        assert not cli.exists()


def macos(package, expected_source):
    import mac_installer
    application = Path("/Applications/Khor.app")
    cli = Path("/usr/local/bin/khor")
    assert not application.exists() and not cli.exists() and not cli.is_symlink(), "Use a fresh runner"
    installed = False
    try:
        # The first real installation must preserve a pre-existing CLI.
        run("sudo", "mkdir", "-p", cli.parent)
        with tempfile.TemporaryDirectory(prefix="khor-independent-") as temporary:
            independent = Path(temporary) / "khor"
            independent.write_text("Independent CLI installation\n")
            run("sudo", "cp", independent, cli)
            run("sudo", "/usr/sbin/installer", "-pkg", package, "-target", "/")
            installed = True
            assert cli.read_bytes() == independent.read_bytes()
        run("sudo", "rm", cli)
        # Reinstall after removing our sentinel: the App's public CLI link is
        # established and remains paired with the installed application's bytes.
        run("sudo", "/usr/sbin/installer", "-pkg", package, "-target", "/")
        assert cli.is_symlink() and str(cli.readlink()) == mac_installer.CLI
        programs = application / "Contents/MacOS"
        proof = same_programs(programs, expected_source)
        launch_installed(programs)
        return proof
    finally:
        if cli.is_symlink() and str(cli.readlink()) == mac_installer.CLI:
            run("sudo", "rm", cli)
        elif cli.is_file() and cli.read_text(errors="replace") == "Independent CLI installation\n":
            run("sudo", "rm", cli)
        if installed:
            run("sudo", "rm", "-rf", application)
            run("sudo", "/usr/sbin/pkgutil", "--forget", mac_installer.IDENTIFIER)


def invoke_msi(action, package, destination=None):
    executable = Path(os.environ["SystemRoot"]) / "System32/msiexec.exe"
    log = package.with_suffix(".install.log" if action == "/i" else ".uninstall.log")
    # MSI has its own parser: quote the property value, not PROPERTY=value
    # as one CRT argv element. The latter opens an invisible usage dialog.
    command = f'"{executable}" {action} "{package}" /qn /norestart /L*V "{log}"'
    if destination is not None:
        command += f' INSTALLDIR="{destination}"'
    return subprocess.run(command, timeout=600)


def windows(package, expected_source):
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
            result = invoke_msi("/i", package, destination)
            assert result.returncode in (0, 3010), result.returncode
            installed = True
            proof = same_programs(destination, expected_source)
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
                result = invoke_msi("/x", package)
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
    parser.add_argument("--expected-source", default=None, help="Pinned package source when checking an earlier build")
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Installation checks require an isolated GitHub runner")
    package = args.package.resolve(strict=True)
    expected_source = args.expected_source or source_sha()
    if len(expected_source) != 40 or any(c not in "0123456789abcdef" for c in expected_source):
        parser.error("Package source must be a complete commit SHA")
    proof = windows(package, expected_source) if os.name == "nt" else macos(package, expected_source) if sys.platform == "darwin" else linux(package, expected_source)
    proof["validation"] = {"test_source": source_sha(), "package_source": expected_source, "resident_start_stop": True, "app_launch": True}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(proof, indent=2) + "\n")
