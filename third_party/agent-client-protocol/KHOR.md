# Khor ACP SDK resource patch

This directory starts from the locked `agent-client-protocol` **3.5.0** crate,
repository https://github.com/agentclientprotocol/rust-sdk, under its preserved
Apache-2.0 license. `README.md`, both upstream Cargo manifests, the upstream
crate lockfile, license and all other original files remain present. The
original 114 files (2,228,415 bytes) were committed unchanged before local work.

Original crates.io archive SHA-256 / locked checksum:
`9bbe69c878aef64a3d92041e256e75acdb1bf2ea8e55ef3313e69dbb2f5bd75a`.
All 114 archive files also matched the active registry source byte for byte.
The registry-only `.cargo-ok` file is not part of this copy.

## Why the local resource boundary is necessary

`ByteStreams` reads a complete JSON line, `Lines` parses it and pushes a
`TransportFrame` into an unbounded channel. The protocol actor can be awaiting
a notification callback while physical input keeps growing this earlier
queue. Limiting Khor's later event queues alone therefore does not bound
retained history. Default `ConnectTo` adapters can introduce further channels,
so ownership cannot be returned merely when an intermediate bridge forwards.

The selected local change is byte/count admission at physical ingress. A
transparent frame-owned credit follows the existing frame through every bridge
and clone until full dispatch or queued-frame drop. Batch, unknown response,
malformed input, error and cancellation must all retire that ownership. The
wire value, JSON parser, RPC identity tracking and dispatch semantics remain
the original SDK's. Derive and schema packages are not changed or upgraded.

The encoded admission window is not a total-history limit. A single frame may
exceed that window and must enter exclusively with its real byte size recorded.
The existing reader has no finite single-line bound: the resource statement
is `O(window + maxSingleFrame)` plus documented parse/decode transients, not
a hard 8 MiB whole-process RSS limit. Legitimate long loads must continue via
Khor's separately owned FIFO event spool rather than be truncated or refused
at a fabricated 32 MiB history threshold.

## Current checkpoint and validation

This package-declaration checkpoint adds the local dependency mapping and
workspace exclusion only. Resource behavior implementation and validation are
subsequent commits. The root `Cargo.lock` is deliberately not hand-edited; its
source change requires a separately authorized real Cargo lock generation. No
new Cargo command, metadata generation or native test has run for this copy.

The focused verification must exercise the actual
`AcpAgent -> ConnectTo -> ByteStreams -> protocol` path, distinguish encoded
window/count/oversize/line/decode terms from measured RSS, cover saturation
and cancellation, and preserve the existing finite event-fence/4096 contract.
It must not use a side parser or a protocol stand-in as product evidence.

## Removal

Replace this patch only with an upstream version that preserves the same
whole-frame admission lifetime through adapters and has the focused pressure,
cancellation and event-fence evidence. Remove the root path mapping and
workspace exclusion together; do not drop the bound solely because a later
Khor queue is bounded.
