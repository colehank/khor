---
title: Pairing
summary: Pair with one device to join your whole network
order: 2
---

Your first device starts a network of its own. Pair each additional device once to bring it into that network.

## Add a device

Create an invitation on any device already in the network:

```sh
khor invite
```

The invitation expires after 15 minutes and can be used only once. On the new device:

```sh
khor pair <ticket>
```

Once paired, device information syncs across the network:

```sh
khor devices
```

## Root key and recovery words

The first device holds your network’s root key. It signs the certificates that identify your devices. Write down the 24 recovery words and store them separately:

```sh
khor root words <local-device-name>
```

If you lose the device holding the root key, recover it on another device:

```sh
khor root recover <24-recovery-words>
```

`khor root show` displays the root trusted by this device and whether it holds the key.

## Pending approval

A new device joins once it receives a certificate signed by the root holder. If the inviting device does not hold the root key, approve the request on the device that does:

```sh
khor pending
khor approve <device>
```

## SSH-only hosts

Bring in a host that cannot run Khor. Your local device handles operations on its behalf. Passwordless SSH is required:

```sh
khor adopt <ssh-host> [name]
```

## Remove a device or leave

| Command | Effect |
|---|---|
| `khor forget <device>` | Remove a device from the network; it must pair again to rejoin |
| `khor revoke <device>` | Revoke a device from the root holder; peers stop accepting it after receiving the updated list |
| `khor leave <local-device-name>` | Leave the network while keeping local sessions, workspaces, identity, and settings |
| `khor pause`, `khor resume` | Pause or resume network access |

Removing a device cannot erase credentials already stored on it. If a device is lost, rotate those credentials with the relevant providers.
