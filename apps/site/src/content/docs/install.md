---
title: Installation
summary: Get the desktop app or use Khor from your terminal
order: 1
---

<h2 id="desktop">Desktop app</h2>

Choose Apple silicon or Intel on the [download page](/download/). Check **About This Mac** if you’re unsure which chip you have.

1. Unzip the download, move **Khor.app** to **Applications**, and open it.
2. Open **Machines** and check that your device appears. One device is enough to get started.
3. Open **Sessions** and select **＋** beside the list. In the new tab, choose a target machine, directory, and available launch method, then select **Create**.

### Connect another device

Each device needs Khor. Install the app on a Mac with a desktop, or use the CLI below for a headless Linux or Mac host.

In **Machines**, select **＋**, then **Khor host**:

- On an existing device, select **Invite device** and copy the invitation.
- On the new device, select **Join network** from the same menu and enter the invitation.
- Check that both devices appear in the machine list. You can now choose the other device when creating a session. If approval is pending, approve it on the device holding the root key.

Open an existing session from the list to continue working. Connections, credentials, and launch methods depend on your devices. The website preview uses local demo data.

See [Pairing](/docs/pair/) for the command-line workflow.

<h2 id="cli">Command-line client</h2>

Use Khor in your terminal, or install the background service on a headless host:

```sh
curl -fsSL https://khor.cloud/install.sh | sh
```

No root access or toolchain required. Supports Linux (x86_64 and aarch64) and macOS (Apple silicon and Intel).

## What the installer does

- Installs `khor` in `~/.local/bin` and adds it to `~/.profile` if the directory is missing from `PATH`
- Creates an identity for this device
- Starts `khor serve` and registers it to start automatically: a LaunchAgent on macOS, systemd on Linux, or a login hook when systemd is unavailable

Open a new terminal and check the installation:

```sh
khor version
khor doctor
```

`khor doctor` reports each check with a name, status, and explanation. An unmeasured check does not mean a healthy result; it means no reading is available.

## Options

Place environment variables before `sh`:

```sh
curl -fsSL https://khor.cloud/install.sh | KHOR_NO_SERVE=1 sh
```

| Variable | Effect |
|---|---|
| `KHOR_NO_SERVE` | Install without starting the background service |
| `KHOR_MIRROR` | Set a download mirror; defaults to GitHub Releases |

## Install from GitHub

```sh
curl -fsSL https://github.com/colehank/khor/releases/latest/download/install.sh | sh
```

Use `KHOR_VERSION` with this command to select a specific version.

## Update

Run the install command again. It replaces the binary and restarts the background service.
