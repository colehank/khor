//! One QUIC address discovery probe, the one iroh's net report sends, from
//! a chosen local address: the in-path test for QAD on a relay's UDP port
//! (audit/net-d.md §5.2, net-i). Binding to an interface's own address
//! picks the egress (`en8` CERNET, `en1` commercial); `0.0.0.0` follows
//! the routing table (through the proxy's TUN on MacLab).
//!
//! `cargo run -p khor-net --example qad_probe -- <server ip:port> <name> <ca.pem> [bind ip] [count]`
//!
//! `name` is what the certificate is checked against, the relay url's
//! host (an IP for Beijing's plain relay, so no SNI is sent). Prints one
//! line per probe: the address the server saw, the round trip, or why it
//! failed.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use n0_future::StreamExt;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: qad_probe <server ip:port> <name> <ca.pem> [bind ip] [count]");
        std::process::exit(2);
    }
    let server: SocketAddr = args[0].parse().expect("server ip:port");
    let name = args[1].clone();
    let bind: std::net::IpAddr = args.get(3).map_or("0.0.0.0", String::as_str).parse().expect("bind ip");
    let count: u32 = args.get(4).map_or(Ok(3), |c| c.parse()).expect("count");

    let mut roots = rustls::RootCertStore::empty();
    for c in CertificateDer::pem_file_iter(&args[2]).expect("ca.pem") {
        roots.add(c.expect("a certificate")).expect("a usable certificate");
    }
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("TLS 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth();

    for i in 0..count {
        // A fresh socket each time, as the net report opens one per
        // probe: a NAT that maps per destination shows its new port.
        let ep = noq::Endpoint::client(SocketAddr::new(bind, 0)).expect("bind");
        let local = ep.local_addr().expect("local addr");
        let client = iroh_relay::quic::QuicClient::new(ep.clone(), tls.clone());
        let began = Instant::now();
        let outcome = tokio::time::timeout(Duration::from_secs(10), async {
            let conn = client.create_conn(server, &name).await.map_err(|e| format!("{e:#}"))?;
            let seen = conn
                .observed_external_addr()
                .next()
                .await
                .ok_or_else(|| "no observed address".to_owned())?;
            let rtt = conn.rtt(noq_proto::PathId::ZERO).unwrap_or_default();
            conn.close(0u32.into(), b"probe done");
            Ok::<_, String>((seen, rtt))
        })
        .await;
        let took = began.elapsed().as_millis();
        match outcome {
            Ok(Ok((seen, rtt))) => println!("{i} from {local}: seen as {seen}, rtt {} ms, {took} ms", rtt.as_millis()),
            Ok(Err(e)) => println!("{i} from {local}: failed after {took} ms: {e}"),
            Err(_) => println!("{i} from {local}: no answer in 10 s"),
        }
        ep.wait_idle().await;
    }
}
