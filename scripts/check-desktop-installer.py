"""Install and remove a desktop package on an isolated build runner."""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import uuid

from release_manifest import executable_info, source_sha

WINDOWS_SESSION_TITLE = "Khor installer terminal"
WINDOWS_READY_MARKER = "KHOR_TERMINAL_READY"
WINDOWS_PEER_SESSION_TITLE = "Khor installer peer terminal"
WINDOWS_PEER_READY_MARKER = "KHOR_PEER_TERMINAL_READY"


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
    elif os.name == "nt":
        # Previous published packages sent users to their installer. This is
        # also verified when a previous MSI is the baseline for an upgrade.
        assert "settings" in guidance or "installer" in guidance, guidance
    else:
        assert "installer" in guidance or "安装包" in guidance, guidance
    return {"cli": command, "app": application}


def retire_processes(processes, terminate, *, timeout=10, quiet=1,
                     clock=time.monotonic, sleep=time.sleep):
    """Retire only fixture-owned programs, including a worker starting late."""
    deadline = clock() + timeout
    vacant_since = None
    while True:
        active = processes()
        if active:
            vacant_since = None
            for process in active:
                terminate(process)
        else:
            if vacant_since is None:
                vacant_since = clock()
            if clock() - vacant_since >= quiet:
                return
        if clock() >= deadline:
            raise AssertionError(f"The installation check must retire its own processes: {active}")
        sleep(0.1)


def windows_app_window(pid):
    from ctypes import POINTER, Structure, WINFUNCTYPE, WinDLL, WinError, byref, sizeof
    from ctypes.wintypes import BOOL, DWORD, HWND, LPARAM, RECT, UINT, WORD

    class WindowInfo(Structure):
        _fields_ = [("size", DWORD), ("window", RECT), ("client", RECT),
                    ("style", DWORD), ("extended_style", DWORD), ("status", DWORD),
                    ("border_x", UINT), ("border_y", UINT), ("atom", WORD), ("version", WORD)]

    user32 = WinDLL("user32", use_last_error=True)
    callback = WINFUNCTYPE(BOOL, HWND, LPARAM)
    user32.EnumWindows.argtypes = (callback, LPARAM)
    user32.EnumWindows.restype = BOOL
    user32.GetWindowThreadProcessId.argtypes = (HWND, POINTER(DWORD))
    user32.GetWindowThreadProcessId.restype = DWORD
    user32.IsWindowVisible.argtypes = (HWND,)
    user32.IsWindowVisible.restype = BOOL
    user32.GetWindowInfo.argtypes = (HWND, POINTER(WindowInfo))
    user32.GetWindowInfo.restype = BOOL
    # WS_CAPTION, WS_SYSMENU, WS_MINIMIZEBOX and WS_MAXIMIZEBOX.
    caption_controls = 0x00C00000 | 0x00080000 | 0x00020000 | 0x00010000
    captioned = []

    @callback
    def collect(handle, _):
        owner = DWORD()
        user32.GetWindowThreadProcessId(handle, byref(owner))
        if owner.value == pid and user32.IsWindowVisible(handle):
            info = WindowInfo(size=sizeof(WindowInfo))
            if user32.GetWindowInfo(handle, byref(info)):
                # Tao can keep the caption style bits on a frameless window.
                # A drawn caption also occupies space between the top resize
                # border and the WebView client area.
                top_inset = info.client.top - info.window.top
                if (info.style & caption_controls == caption_controls
                        and top_inset > info.border_y):
                    captioned.append(handle)
        return True

    if not user32.EnumWindows(collect, 0):
        raise WinError()
    assert len(captioned) <= 1, "The installed App drew multiple native Windows captions"
    return captioned[0] if captioned else None


def request_windows_close(window):
    from ctypes import WinDLL, WinError
    from ctypes.wintypes import BOOL, HWND, LPARAM, UINT, WPARAM

    user32 = WinDLL("user32", use_last_error=True)
    user32.PostMessageW.argtypes = (HWND, UINT, WPARAM, LPARAM)
    user32.PostMessageW.restype = BOOL
    # WM_CLOSE follows the same close request and draft guard as the system X.
    if not user32.PostMessageW(window, 0x0010, 0, 0):
        raise WinError()


def check_windows_backdrop(window):
    from ctypes import WinDLL, byref, c_int, c_uint, c_void_p, sizeof
    from ctypes.wintypes import HWND

    compositor = WinDLL("dwmapi")
    read = compositor.DwmGetWindowAttribute
    read.argtypes = (HWND, c_uint, c_void_p, c_uint)
    mode = c_int()
    # DWMWA_SYSTEMBACKDROP_TYPE is available on Windows 11 and Server 2025.
    # DWMSBT_TRANSIENTWINDOW applies Desktop Acrylic to the whole window.
    deadline = time.monotonic() + 8
    while True:
        result = read(window, 38, byref(mode), sizeof(mode))
        assert result == 0, f"The Windows runner cannot read its DWM backdrop: HRESULT 0x{result & 0xffffffff:08x}"
        if mode.value == 3:
            print("The installed Windows App requested Desktop Acrylic from DWM.", flush=True)
            return
        if time.monotonic() >= deadline:
            initial = mode.value
            set_mode = compositor.DwmSetWindowAttribute
            set_mode.argtypes = read.argtypes
            control = c_int(3)
            set_result = set_mode(window, 38, byref(control), sizeof(control))
            control_read = read(window, 38, byref(mode), sizeof(mode))
            restored = c_int(initial)
            set_mode(window, 38, byref(restored), sizeof(restored))
            raise AssertionError(
                f"The App backdrop was {initial}, expected Desktop Acrylic (3). "
                f"Direct DWM control: set 0x{set_result & 0xffffffff:08x}, "
                f"read 0x{control_read & 0xffffffff:08x}, backdrop {mode.value}"
            )
        time.sleep(0.1)


def wait_for_resident(service, home, log):
    endpoint = Path(home) / ".khor/endpoint.json"
    deadline = time.monotonic() + 45
    while not endpoint.is_file():
        if service.poll() is not None or time.monotonic() >= deadline:
            details = log.read_text(errors="replace")[-16000:]
            raise AssertionError(f"Packaged serve did not publish its endpoint ({service.returncode}): {details}")
        time.sleep(0.1)


def test_direct_network(environment):
    # Use only the runner's direct link. Never advertise these test devices
    # on the physical network or submit their identities to a public relay.
    environment.update(KHOR_NO_MDNS="1", KHOR_WEB_PORT="0")
    directory = Path(environment["KHOR_HOME"]) / ".khor"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "relays.json").write_text(
        '{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}', encoding="utf-8")


def check_installed_powershell(cli, environment, scratch):
    legacy = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
    modern = shutil.which("pwsh.exe")
    assert legacy.is_file() and modern, "The Windows installer runner needs both PowerShell versions"
    for name, executable in (("Windows PowerShell", legacy), ("PowerShell 7", modern)):
        marker = scratch / f"{name} marker.txt"
        literal = str(marker).replace("'", "''")
        command = f"Set-Content -LiteralPath '{literal}' -Value 'KHOR_POWERSHELL_OK' -Encoding ASCII"
        opened = subprocess.run([str(cli), "open", "-d", "--title", name, "--", str(executable),
                                 "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", command],
                                env=environment, cwd=scratch, stdout=subprocess.PIPE,
                                text=True, encoding="utf-8", check=True, timeout=60)
        session_id = opened.stdout.strip()
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if marker.is_file() and marker.read_text(encoding="ascii").strip() == "KHOR_POWERSHELL_OK":
                break
            time.sleep(0.1)
        else:
            kind, _, leaf = session_id.partition("/")
            assert kind == "shell" and leaf.isalnum(), f"Invalid test session ID: {session_id!r}"
            session = Path(environment["KHOR_HOME"]) / ".khor/sessions" / f"{kind}-{leaf}"
            for filename in ("state.json", "host-error.txt", "last.txt", "ring.bin"):
                path = session / filename
                if path.is_file():
                    with path.open("rb") as stream:
                        stream.seek(max(0, path.stat().st_size - 2400))
                        print(f"{name} {filename}: {stream.read(2400).decode(errors='replace')!r}", flush=True)
            raise AssertionError(f"The installed Khor did not finish its command in {name} ({session_id})")
        print(f"The installed Khor executed a hosted command in {name}.", flush=True)


@contextmanager
def windows_file_association(scratch, workspace):
    import winreg

    extension = "kh" + uuid.uuid4().hex[:8]
    progid = "Khor.Installer." + extension
    classes = r"Software\Classes"
    extension_key = f"{classes}\\.{extension}"
    program_key = f"{classes}\\{progid}"
    reader_source = scratch / "FileReceipt.cs"
    reader = scratch / "FileReceipt.exe"
    receipt = workspace / "file-handoff-receipt.txt"
    reader_source.write_text(
        'using System; using System.IO; class FileReceipt { static void Main(string[] a) { '
        'string contents; try { contents = File.ReadAllText(a[0]); } '
        'catch (Exception e) { contents = "KHOR_READER_ERROR:" + e.GetType().Name; } '
        'var temp = a[1] + ".tmp"; File.WriteAllText(temp, a[0] + "\\n" + contents); '
        'File.Move(temp, a[1]); } }',
        encoding="ascii",
    )
    source_arg = str(reader_source).replace("'", "''")
    reader_arg = str(reader).replace("'", "''")
    run("powershell.exe", "-NoProfile", "-NonInteractive", "-Command",
        f"$ErrorActionPreference='Stop'; Add-Type -Path '{source_arg}' "
        f"-OutputAssembly '{reader_arg}' -OutputType WindowsApplication")
    scale = float(run(
        "powershell.exe", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference='Stop'; "
        "$settings=[Windows.UI.ViewManagement.UISettings,Windows.UI.ViewManagement,ContentType=WindowsRuntime]::new(); "
        "$settings.TextScaleFactor.ToString([CultureInfo]::InvariantCulture)",
    ).stdout.strip())
    try:
        with winreg.CreateKey(winreg.HKEY_CURRENT_USER, extension_key) as key:
            winreg.SetValueEx(key, "", 0, winreg.REG_SZ, progid)
        with winreg.CreateKey(winreg.HKEY_CURRENT_USER, program_key + r"\shell\open\command") as key:
            winreg.SetValueEx(key, "", 0, winreg.REG_SZ, f'"{reader}" "%1" "{receipt}"')
        control = workspace / f"Association control.{extension}"
        control.write_text("KHOR_SHELL_CONTROL", encoding="ascii")
        os.startfile(control)
        deadline = time.monotonic() + 10
        while not receipt.is_file() and time.monotonic() < deadline:
            time.sleep(0.1)
        assert receipt.is_file() and receipt.read_text(encoding="utf-8").endswith("KHOR_SHELL_CONTROL"), (
            "Windows did not honor the test-only file association for the local control"
        )
        receipt.unlink()
        yield f"--desktop-parity={extension},{scale}"
    finally:
        for key in (extension_key, program_key + r"\shell\open\command",
                    program_key + r"\shell\open", program_key + r"\shell", program_key):
            try:
                winreg.DeleteKey(winreg.HKEY_CURRENT_USER, key)
            except FileNotFoundError:
                pass


def launch_installed(directory, acp_fixture=None, native_drag=False, legacy_shell_ui=False,
                     shell_selection=None, desktop_parity=False, windows_backdrop=False):
    prefix = "Khor 验收 home " if os.name == "nt" else "khor-install-home-"
    # The runner reaps this scratch directory. Process shutdown is asserted
    # separately, and short-lived Windows file locks must not hide a failure.
    with tempfile.TemporaryDirectory(prefix=prefix, ignore_cleanup_errors=os.name == "nt") as temporary:
        # Windows may give Python an 8.3 TEMP path while CIM reports the
        # executable with the long spelling. Match the same owned directory.
        scratch = Path(temporary).resolve()
        runtime = scratch / "runtime"
        environment = {**os.environ, "KHOR_HOME": str(scratch / "node"),
                       "KHOR_RUNTIME_DIR": str(runtime), "LANG": "en_US.UTF-8"}
        suffix = ".exe" if os.name == "nt" else ""
        application = (directory / ("khor-gui" + suffix)).resolve()
        cli = (directory / ("khor" + suffix)).resolve()
        if os.name == "nt":
            # Debugging is enabled only on this installed-package check, with the
            # private WebView2 profile selected by KHOR_HOME.
            with socket.socket() as listener:
                listener.bind(("127.0.0.1", 0))
                debug_port = listener.getsockname()[1]
            logs = Path(os.environ.get("RUNNER_TEMP") or tempfile.gettempdir()) / "khor-installer-logs"
            logs.mkdir(parents=True, exist_ok=True)
            environment["KHOR_LOG_FILE"] = str(logs / "window.log")
            environment["KHOR_GUI_TRACE"] = "1"
            environment["KHOR_STARTUP_TRACE"] = str(logs / "startup.log")

        def processes():
            if os.name == "nt":
                source = run("powershell.exe", "-NoProfile", "-Command",
                             "@(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath } | Select-Object ProcessId,ExecutablePath) | ConvertTo-Json -Compress").stdout
                rows = [(row["ProcessId"], Path(row["ExecutablePath"]).resolve()) for row in json.loads(source)]
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
        exit_service = None
        log = None
        exit_log = None
        try:
            # The packaged CLI must run a real node, not only print metadata.
            log_path = scratch / "serve.log"
            log = log_path.open("wb")
            if os.name == "nt":
                environment["KHOR_NAME"] = "installer-client"
                test_direct_network(environment)
            service = subprocess.Popen([str(cli), "serve"],
                                       env=environment, stdout=log, stderr=subprocess.STDOUT)
            wait_for_resident(service, environment["KHOR_HOME"], log_path)
            if os.name == "nt":
                check_installed_powershell(cli, environment, scratch)
                if acp_fixture is not None:
                    subprocess.run([str(cli), "agents", "add", "installer-acp", "--", str(acp_fixture)],
                                   env=environment, cwd=scratch, check=True, timeout=60)
            subprocess.run([str(cli), "quit"], env=environment, check=True, timeout=30)
            service.wait(timeout=10)
            deadline = time.monotonic() + 10
            while processes() and time.monotonic() < deadline:
                time.sleep(0.1)
            assert not processes(), "Quitting the packaged node must not leave or respawn its resident"
            if windows_backdrop:
                subprocess.run([str(cli), "config", "set", "window-material", "frost"],
                               env=environment, check=True, timeout=30)
            elif os.name == "nt":
                exit_name = "installer-exit"
                exit_env = {**environment, "KHOR_HOME": str(scratch / "exit"),
                            "KHOR_NAME": exit_name, "KHOR_LOG_FILE": str(logs / "exit.log")}
                test_direct_network(exit_env)
                exit_log_path = logs / "exit-serve.log"
                exit_log = exit_log_path.open("wb")
                exit_service = subprocess.Popen([str(cli), "serve"], env=exit_env,
                                                stdout=exit_log, stderr=subprocess.STDOUT)
                wait_for_resident(exit_service, exit_env["KHOR_HOME"], exit_log_path)
                ticket = subprocess.run([str(cli), "invite"], env=exit_env,
                                        stdout=subprocess.PIPE, check=True, timeout=30).stdout
                subprocess.run([str(cli), "pair", "-"], env=environment,
                               input=ticket, check=True, timeout=90)
                cmd = Path(os.environ["SystemRoot"]) / "System32/cmd.exe"
                subprocess.run([str(cli), "open", "-d", "--title", WINDOWS_SESSION_TITLE, "--",
                                str(cmd), "/d", "/k", "echo", WINDOWS_READY_MARKER],
                               env=environment, cwd=scratch, check=True, timeout=60)
                subprocess.run([str(cli), "open", "-d", "--title", WINDOWS_PEER_SESSION_TITLE, "--",
                                str(cmd), "/d", "/k", "echo", WINDOWS_PEER_READY_MARKER],
                               env=exit_env, cwd=scratch, check=True, timeout=60)
                environment["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = f"--remote-debugging-port={debug_port}"
            subprocess.run([str(cli), "gui"], env=environment, check=True, timeout=30)
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                app_pids = [pid for pid, path in processes() if path == application]
                if app_pids:
                    if os.name == "nt":
                        check = Path(__file__).resolve().parents[1] / "apps/gui/scripts/check-windows-installed.mjs"
                        workspace = scratch / "Khor 项目"
                        folder = workspace / "资料"
                        folder.mkdir(parents=True)
                        (folder / "readme.md").write_text("# Khor installation workspace\n", encoding="utf-8")
                        assert len(app_pids) == 1, "The installer check expects one App process"
                        pid = app_pids[0]
                        deadline = time.monotonic() + 45
                        while not (window := windows_app_window(pid)) and time.monotonic() < deadline:
                            time.sleep(0.2)
                        assert window, "The installed App never drew a visible native Windows caption"
                        print("The installed App drew one native Windows caption.", flush=True)
                        try:
                            if windows_backdrop:
                                check_windows_backdrop(window)
                                return
                            args = ["node", str(check), str(debug_port), WINDOWS_SESSION_TITLE,
                                    WINDOWS_READY_MARKER, str(workspace), exit_name,
                                    WINDOWS_PEER_SESSION_TITLE, WINDOWS_PEER_READY_MARKER]
                            if acp_fixture is not None:
                                args.append("--acp")
                            if native_drag:
                                args.append(f"--native-drag={window}")
                            if legacy_shell_ui:
                                args.append("--legacy-shell-ui")
                            if shell_selection:
                                args.append(f"--shell-selection={shell_selection}")
                            if desktop_parity:
                                with windows_file_association(scratch, workspace) as parity:
                                    subprocess.run([*args, parity], check=True, timeout=300)
                            else:
                                subprocess.run(args, check=True, timeout=300)
                        finally:
                            request_windows_close(window)
                            deadline = time.monotonic() + 15
                            while any(path == application for _, path in processes()):
                                assert time.monotonic() < deadline, "Closing the window must exit the installed App"
                                time.sleep(0.1)
                            print("The installed App exited after the Windows close request.", flush=True)
                    else:
                        time.sleep(1)
                        assert any(path == application for _, path in processes()), "The installed App exited during startup"
                    return
                time.sleep(0.1)
            raise AssertionError("khor gui did not launch the installed application")
        finally:
            if exit_service is not None and exit_service.poll() is None:
                exit_service.kill()
                exit_service.wait(timeout=10)
            if exit_log is not None:
                exit_log.close()
            if service is not None and service.poll() is None:
                service.kill()
                service.wait(timeout=10)
            if log is not None:
                log.close()
            def terminate(process):
                pid, path = process
                if (pid, path) not in processes():
                    return
                if os.name == "nt":
                    subprocess.run(["taskkill.exe", "/PID", str(pid), "/T", "/F"], capture_output=True)
                else:
                    try:
                        os.kill(pid, 15)
                    except ProcessLookupError:
                        pass
            for process in processes():
                terminate(process)
            # Closing the App intentionally leaves its node running. Stop the
            # node in this private home as well, before retiring late workers.
            try:
                subprocess.run([str(cli), "quit"], env=environment,
                               capture_output=True, timeout=30)
            finally:
                retire_processes(processes, terminate)


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
    diagnostics = Path(os.environ.get("RUNNER_TEMP") or tempfile.gettempdir()) / "khor-installer-logs"
    diagnostics.mkdir(parents=True, exist_ok=True)
    log = diagnostics / (package.stem + (".install.log" if action == "/i" else ".uninstall.log"))
    # MSI has its own parser: quote the property value, not PROPERTY=value
    # as one CRT argv element. The latter opens an invisible usage dialog.
    command = f'"{executable}" {action} "{package}" /qn /norestart /L*V "{log}"'
    if destination is not None:
        command += f' INSTALLDIR="{destination}"'
    return subprocess.run(command, timeout=600)


def windows(package, expected_source, acp_fixture=None, previous=None, native_drag=False,
            legacy_shell_ui=False, shell_selection=None, desktop_parity=False, windows_backdrop=False):
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
        installed = None
        try:
            if previous is not None:
                previous_package, previous_source = previous
                baseline = Path(temporary) / "Previous Khor.msi"
                shutil.copy2(previous_package, baseline)
                result = invoke_msi("/i", baseline, destination)
                assert result.returncode in (0, 3010), result.returncode
                installed = baseline
                same_programs(destination, previous_source)
                print("Installed and verified the previous Windows App and CLI.", flush=True)
            # An in-app upgrade uses the installer's registered location.
            result = invoke_msi("/i", package, None if previous is not None else destination)
            assert result.returncode in (0, 3010), result.returncode
            installed = package
            proof = same_programs(destination, expected_source)
            if previous is not None:
                proof["installed_windows_msi_upgrade"] = {"previous_source": previous_source}
                print("The new MSI upgraded both programs in the existing custom installation directory.", flush=True)
            assert (destination / "cli-owner.json").is_file()
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                after, _ = winreg.QueryValueEx(key, "Path")
            entries = [value.rstrip("\\").casefold() for value in after.split(";")]
            assert entries[0] == str(independent).casefold(), "Independent CLI keeps its PATH priority"
            assert str(destination).casefold() in entries
            launch_installed(destination, acp_fixture, native_drag, legacy_shell_ui, shell_selection,
                             desktop_parity, windows_backdrop)
            if windows_backdrop:
                proof["installed_windows_native_acrylic"] = True
                return proof
            if desktop_parity:
                proof["installed_system_text_and_remote_file_handoff"] = True
            if acp_fixture is not None:
                proof["installed_acp_conversation"] = True
            proof["installed_powershell_commands"] = True
            checked_shells = (shell_selection,) if shell_selection else ("cmd", "powershell")
            proof["installed_powershell_interaction"] = "powershell" in checked_shells
            proof["installed_windows_shell_selection"] = checked_shells
            proof["installed_terminal_roundtrip"] = True
            proof["installed_paired_node_terminal"] = True
            if not legacy_shell_ui:
                proof["installed_terminal_attachment_paths"] = True
                proof["installed_paired_node_attachment"] = True
            if native_drag:
                proof["installed_native_file_drag"] = True
            proof["installed_native_caption"] = True
            proof["installed_workspace_preview"] = True
            proof["installed_browser_lifecycle"] = True
            proof["installed_browser_borrowed_loopback"] = True
            return proof
        finally:
            try:
                if installed is not None:
                    result = invoke_msi("/x", installed)
                    assert result.returncode in (0, 3010), result.returncode
                    assert not (destination / "khor.exe").exists()
                    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
                        after, _ = winreg.QueryValueEx(key, "Path")
                    assert str(independent) in after and str(destination).casefold() not in after.casefold()
            finally:
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
    parser.add_argument("--acp-fixture", type=Path, help="Scripted ACP agent for the installed Windows App")
    parser.add_argument("--upgrade-from", type=Path, help="Earlier MSI to upgrade in place")
    parser.add_argument("--upgrade-source", help="Complete source SHA of the previous MSI")
    parser.add_argument("--native-drag", action="store_true", help="Exercise Windows OLE file drag on the runner desktop")
    parser.add_argument("--legacy-shell-ui", action="store_true", help="Replay a package before shell-aware attachments")
    parser.add_argument("--shell-selection", choices=["cmd", "powershell"], help="Replay only this explicit GUI shell choice")
    parser.add_argument("--desktop-parity", action="store_true", help="Verify Windows system text scaling and native file handoff")
    parser.add_argument("--windows-backdrop", action="store_true", help="Verify the installed Windows App's compositor backdrop without WebView debugging")
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Installation checks require an isolated GitHub runner")
    if args.legacy_shell_ui and args.native_drag:
        parser.error("Native file drag requires the attachment check")
    package = args.package.resolve(strict=True)
    expected_source = args.expected_source or source_sha()
    if len(expected_source) != 40 or any(c not in "0123456789abcdef" for c in expected_source):
        parser.error("Package source must be a complete commit SHA")
    acp_fixture = args.acp_fixture.resolve(strict=True) if args.acp_fixture else None
    previous = None
    if args.upgrade_from or args.upgrade_source:
        source = args.upgrade_source
        if (os.name != "nt" or not args.upgrade_from or not source or source == expected_source
                or len(source) != 40 or any(c not in "0123456789abcdef" for c in source)):
            parser.error("In-place upgrade checks require a previous Windows MSI from a different complete source SHA")
        previous = (args.upgrade_from.resolve(strict=True), source)
    if os.name == "nt":
        proof = windows(package, expected_source, acp_fixture, previous, args.native_drag,
                        args.legacy_shell_ui, args.shell_selection, args.desktop_parity,
                        args.windows_backdrop)
    elif sys.platform == "darwin":
        proof = macos(package, expected_source)
    else:
        proof = linux(package, expected_source)
    proof["validation"] = {"test_source": source_sha(), "package_source": expected_source, "resident_start_stop": True, "app_launch": True}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(proof, indent=2) + "\n")
