# Apple IPv4 source-address control

Vendored from crates.io `noq-udp 1.5.0`, upstream commit
`c7b87d5843c2335c8a15b21324fb4fce99572ed8`. The Apple transmit patch
below is carried forward. License: MIT OR Apache-2.0.

## Temporary patch: transmit `src_ip` with `IP_PKTINFO` on Apple

Both `sendmsg` and optional `sendmsg_x` encoded an IPv4 source with
`IP_RECVDSTADDR`. Darwin receives destination information with that
option but does not use it to choose a transmit source. `IP_PKTINFO`
with zero interface index and `ipi_spec_dst` carries the requested source.
The receive path, IPv6, Linux, Windows, BSD and Solaris are unchanged.
No socket, port, interface binding, discovery task or routing policy is added.

On MacLab with two physical NICs and an active TUN, a wildcard diagnostic
socket using the old control message timed out to Air, while the same
socket type using `IP_PKTINFO` received a valid QUIC version-negotiation
reply in 4.89 ms. A loopback receiver independently saw 127.0.0.1 from
the old encoding and the requested assigned physical IPv4 from the new
encoding. This proves the source-control defect; it is not full Khor
business or mobile recovery acceptance.

Apple's public contract: [in.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/netinet/in.h),
`IP_PKTINFO` and `struct in_pktinfo`. Local SDK headers match this contract.
Upstream source checked 2026-11-17 still uses the old encoding:
[unix.rs](https://github.com/n0-computer/noq/blob/main/noq-udp/src/unix.rs),
[apple_fast.rs](https://github.com/n0-computer/noq/blob/main/noq-udp/src/apple_fast.rs).

Remove this vendor and root patch after an upstream release uses the correct
Apple transmit encoding on both paths, and the source-selection regression,
authenticated direct connection, default-route control, and relay/recovery
regressions pass with that release. Do not remove the test with the patch.
