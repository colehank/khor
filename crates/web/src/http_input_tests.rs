use super::*;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "khor-web-input-{}",
            khor_node::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn request(headers: &str, body: &[u8]) -> (u16, String) {
    let root = Scratch::new();
    let key = key::ensure(&root.0).unwrap();
    let server = tiny_http::Server::http(("127.0.0.1", 0)).unwrap();
    let tiny_http::ListenAddr::IP(addr) = server.server_addr() else {
        panic!("not a TCP server")
    };
    let worker = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut req = server
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
            .expect("fixture received no request");
        let reply = answer(&rt, &root.0, &mut req);
        let status = reply.code;
        let sentence = String::from_utf8(reply.body.clone()).unwrap();
        req.respond(reply.into_response()).unwrap();
        (status, sentence)
    });
    let mut client = TcpStream::connect(addr).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(client, "POST /api/console_catalog HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {key}\r\nConnection: close\r\n{headers}\r\n").unwrap();
    client.write_all(body).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    let result = worker.join().unwrap();
    assert!(
        response.starts_with(&format!("HTTP/1.1 {} ", result.0)),
        "{response}"
    );
    result
}

#[test]
fn malformed_json_is_refused_by_the_authenticated_http_route() {
    for body in [b"{".as_slice(), b"{} {}", b"null trailing", &[0xff]] {
        let (status, _) = request(&format!("Content-Length: {}\r\n", body.len()), body);
        assert_eq!(status, 400);
    }
}

#[test]
fn a_chunked_body_read_failure_is_refused_by_the_http_route() {
    let (status, sentence) = request("Transfer-Encoding: chunked\r\n", b"2\r\n{}\r\nZ\r\n");
    assert_eq!(status, 400);
    assert!(
        sentence.contains("Could not read JSON request"),
        "{sentence}"
    );
}

#[test]
fn empty_body_and_null_still_reach_a_legal_argumentless_command() {
    for body in ["", " \r\n\t", "null", "{}"] {
        let (status, reply) = request(
            &format!("Content-Length: {}\r\n", body.len()),
            body.as_bytes(),
        );
        assert_eq!(status, 200);
        assert!(
            serde_json::from_str::<serde_json::Value>(&reply)
                .unwrap()
                .is_array()
        );
    }
}

#[test]
fn a_resource_limit_is_reported_as_413_by_the_http_route() {
    let body = format!("[{}0]", "0,".repeat(65_536));
    let (status, sentence) = request(
        &format!("Content-Length: {}\r\n", body.len()),
        body.as_bytes(),
    );
    assert_eq!(status, 413);
    assert!(sentence.contains("container entries"), "{sentence}");
}
