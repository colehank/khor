---
title: Relays
summary: Connect automatically, or run a relay of your own
order: 5
---

Devices connect directly whenever possible. Relays help them exchange addresses and carry traffic when a direct connection is unavailable. Relayed content is end-to-end encrypted: the relay forwards it without storing or reading it.

## Defaults

No configuration is needed. Khor’s relays are preferred, with n0’s public relays as fallbacks. Each device selects a nearby relay it can reach.

## Run your own

Use a device already in your network with a public address and a domain:

```sh
khor relay --acme relay.example.com --only-mine
```

`--acme` obtains a certificate from Let’s Encrypt. `--only-mine` admits only trusted, non-revoked devices in your network, without a shared token.

| Option | Effect |
|---|---|
| `--acme <domain>` | Obtain a Let’s Encrypt certificate and serve HTTPS on port 443 |
| `--tls-cert <cert> --tls-key <key>` | Use an existing certificate |
| `--dev` | Serve plain HTTP on port 3340 by default; for testing only |
| `--only-mine` | Admit only devices trusted by your network |
| `--token <secret>` | Require a shared token; cannot be combined with `--only-mine` |
| `--rate <bytes-per-second>` | Limit each client’s upload rate |
| `--metrics <address>` | Expose metrics |
| `--stats` | Print usage totals without starting a relay |

## Connect through it

Relay settings are local to each device. One device cannot change which relays another uses.

- In the app: **Settings → Network → Add relay…**
- With `KHOR_RELAY`: separate relay URLs with commas and add a `token` parameter when needed

```sh
KHOR_RELAY="https://relay.example.com?token=secret,https://relay2.example.com"
```

Tokens stay on the local device. Every device reaching you through a token-protected relay needs that token configured locally. `--only-mine` avoids this requirement.

Enable **Only use mine** to stop using the default Khor and n0 relays.

## Usage totals

```sh
khor relay --stats
```

Only totals are stored: connection counts and online time per device, plus bytes sent and received by the relay. It does not record which devices connect to each other.
