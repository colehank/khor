---
title: Web interface
summary: Open your network from a phone or another computer
order: 4
---

Any device in your network can serve the Khor web interface. Open it on a phone or another computer to access the network without installing anything on that device.

```sh
khor web
```

This prints a link and a QR code.

## Where it runs

`khor serve` hosts the interface on your local network at port 5467 by default. Set `KHOR_WEB_PORT` to change the port. The viewing device must be able to reach the host’s local address.

This interface does not use TLS and is intended for local networks. Do not expose its port to the public internet.

Use `khor faces` to see the interfaces running on this device and the web interface’s port.

## Access key

The link contains an access key. Avoid placing it in synced documents. Generate a new key to invalidate all previously shared links:

```sh
khor web --new
```

## PIN lock

Add a PIN to the app window and web interface:

```sh
khor lock pin             # Read one line from standard input
khor lock on
khor lock timeout 5       # Minutes: 0, 1, 5, 15, 60; 0 means never
```

This does not lock the command-line client.

## Connection issues on macOS

The macOS firewall may initially block incoming local connections. Allow Khor in the firewall settings. Before printing a link, `khor web` checks whether its address is reachable. If it is not, the command explains the problem instead of printing an unusable link.
