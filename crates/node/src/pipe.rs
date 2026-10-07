//! **ssh inside khor** (round ROAD-1, audit/ROAD-BRIEF.md T2): a byte
//! pipe from this process's stdin and stdout to `host:port` as another
//! khor machine's network reaches it — what ssh's `ProxyCommand` wants:
//!
//! ```text
//! ssh -o ProxyCommand='khor pipe --via lab %h %p' gpu-box
//! ```
//!
//! reaches an intranet ssh host behind `lab` without `lab` adopting it.
//! The pipe is the exit's (docs/NET.md 借网): the same ALPN, the same
//! per-stream gate on the far side (in the table, on the roster, not
//! paused), the same status byte. ssh's own key check runs end to end
//! over it; the exit sees ciphertext.
//!
//! # Two shapes, by who holds the key
//!
//! With no resident this process binds an endpoint and dials the exit
//! itself. With one, it cannot (two endpoints on one key knock each other
//! off), so the resident dials the exit's stream and binds a loopback
//! port that takes **one** connection carrying a one-time cookie; this
//! process connects there and splices. The stream is dialled before the
//! port is handed out, so a refusal or an unreachable target is said to
//! the verb in words, and ssh prints them.

use std::time::Duration;

use khor_catalog::msg;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ipc;

/// How long the resident's one-shot port waits for the verb that asked
/// for it, and for that verb's cookie.
const CLAIM_WITHIN: Duration = Duration::from_secs(10);

/// Copies `input` to the far end and the far end to `output` until the
/// far end hangs up. This side running out of input half-closes the far
/// end's read and keeps reading — ssh's last bytes still come back.
pub async fn splice<I, O, R, W>(mut input: I, mut output: O, mut far_read: R, mut far_write: W)
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let up = async {
        let _ = tokio::io::copy(&mut input, &mut far_write).await;
        let _ = far_write.shutdown().await;
    };
    let down = async {
        let _ = tokio::io::copy(&mut far_read, &mut output).await;
        let _ = output.flush().await;
    };
    tokio::pin!(up, down);
    let mut up_done = false;
    loop {
        tokio::select! {
            _ = &mut up, if !up_done => up_done = true,
            _ = &mut down => return,
        }
    }
}

/// `host:port` as a target header; an IPv6 literal gets its brackets.
pub fn target(host: &str, port: u16) -> String {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

impl crate::Node {
    /// **The pipe through `via` to `host:port`**, spliced to `input` and
    /// `output` until the far end hangs up.
    pub async fn pipe_via<I, O>(&self, via: &str, host: &str, port: u16, input: I, output: O) -> Result<(), String>
    where
        I: AsyncRead + Unpin,
        O: AsyncWrite + Unpin,
    {
        let to = target(host, port);
        self.refuse_satellite(via)?;
        // Through this machine is no hop at all: connect from here.
        let (_, home) = self.resolve(via)?;
        if home == self.device() {
            let tcp = tokio::time::timeout(CLAIM_WITHIN, tokio::net::TcpStream::connect(&to))
                .await
                .map_err(|_| msg::tunnel_no_route(&to))?
                .map_err(|_| msg::tunnel_no_route(&to))?;
            let (r, w) = tcp.into_split();
            splice(input, output, r, w).await;
            return Ok(());
        }
        if let Some(reply) = self
            .via_serve(ipc::Op::PipeOn {
                machine: via.to_owned(),
                target: to.clone(),
            })
            .await
        {
            let (addr, cookie) = match reply? {
                ipc::Reply::Piping { addr, cookie } => (addr, cookie),
                ipc::Reply::Refused { why } => return Err(why),
                other => return Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
            let mut tcp = tokio::net::TcpStream::connect(&addr)
                .await
                .map_err(msg::serve_unreachable)?;
            tcp.write_all(format!("{cookie}\n").as_bytes())
                .await
                .map_err(msg::handoff_failed)?;
            let _ = tcp.set_nodelay(true);
            let (r, w) = tcp.into_split();
            splice(input, output, r, w).await;
            return Ok(());
        }
        let ep = khor_net::endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = async {
            let conn = self.pipe_line(&ep, via).await?;
            let (send, recv) = crate::tunnel::dial(&conn, &to).await?;
            splice(input, output, recv, send).await;
            Ok(())
        }
        .await;
        ep.close().await;
        out
    }

    /// The tunnel connection to `via`, refusing a satellite in words: it
    /// is not khor, and ssh's own `-J` through it is the road there.
    async fn pipe_line(&self, ep: &iroh::Endpoint, via: &str) -> Result<iroh::endpoint::Connection, String> {
        self.refuse_satellite(via)?;
        self.dial_tunnel(ep, via).await
    }

    fn refuse_satellite(&self, via: &str) -> Result<(), String> {
        let (channel, _) = self.resolve(via)?;
        match self.devices_loaded()?.doc.by_name(&channel) {
            Some(d) if d.sponsor.is_some() => Err(msg::pipe_via_satellite(d.shown())),
            _ => Ok(()),
        }
    }

    /// The resident's half of [`Self::pipe_via`]: the stream to `target`
    /// through `machine`, dialled now, and a loopback port that hands it
    /// to the one connection that says the cookie first.
    pub(crate) async fn pipe_hosted(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        target: &str,
    ) -> Result<(String, String), String> {
        let conn = self.pipe_line(ep, machine).await?;
        let (send, recv) = crate::tunnel::dial(&conn, target).await?;
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?.to_string();
        let cookie = crate::link::fresh_hex()?;
        let want = cookie.clone();
        tokio::spawn(async move {
            // The connection keeps the stream's wire open for as long as
            // the pipe runs.
            let _conn = conn;
            let claimed = tokio::time::timeout(CLAIM_WITHIN, async {
                loop {
                    let Ok((mut sock, _)) = listener.accept().await else {
                        return None;
                    };
                    let mut said = vec![0u8; want.len() + 1];
                    // A connection that does not say the cookie is not
                    // the verb: dropped, and the port waits on.
                    if sock.read_exact(&mut said).await.is_ok() && said == format!("{want}\n").as_bytes() {
                        return Some(sock);
                    }
                }
            })
            .await;
            if let Ok(Some(sock)) = claimed {
                let _ = sock.set_nodelay(true);
                let (r, w) = sock.into_split();
                // The verb is the input; the exit's stream is the far end.
                splice(r, w, recv, send).await;
            }
        });
        Ok((addr, cookie))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_brackets_an_ipv6_literal() {
        assert_eq!(target("10.0.0.5", 22), "10.0.0.5:22");
        assert_eq!(target("gpu-box", 2222), "gpu-box:2222");
        assert_eq!(target("::1", 22), "[::1]:22");
        assert_eq!(target("[fe80::1]", 22), "[fe80::1]:22");
    }

    /// The input running out is not the end: the far end's last bytes
    /// still come back, and the far end hanging up is.
    #[tokio::test]
    async fn the_input_ending_half_closes_and_the_far_end_ending_ends_it() {
        let (near, far) = tokio::io::duplex(64);
        let (far_r, mut far_w) = tokio::io::split(far);
        let (near_r, near_w) = tokio::io::split(near);
        let mut out = Vec::new();
        let echo = tokio::spawn(async move {
            let mut far_r = far_r;
            let mut got = Vec::new();
            far_r.read_to_end(&mut got).await.unwrap();
            far_w.write_all(b"bye after ").await.unwrap();
            far_w.write_all(&got).await.unwrap();
            far_w.shutdown().await.unwrap();
        });
        tokio::time::timeout(
            Duration::from_secs(5),
            splice(&b"hello"[..], &mut out, near_r, near_w),
        )
        .await
        .expect("it ends when the far end hangs up");
        echo.await.unwrap();
        assert_eq!(out, b"bye after hello");
    }
}
