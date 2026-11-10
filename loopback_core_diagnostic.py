import hashlib
from pathlib import Path

swift = Path('apps/gui/core/tests/fixtures/webkit_loopback.swift')
source = swift.read_text()
print('original Swift sha256', hashlib.sha256(source.encode()).hexdigest())
source = source.replace('finish(["error": error.localizedDescription])', 'finish(["error": error.localizedDescription, "domain": (error as NSError).domain, "code": (error as NSError).code, "details": String(describing: (error as NSError).userInfo)])')
swift.write_text(source)

rust = Path('apps/gui/core/src/web.rs')
source = rust.read_text()
print('original Rust sha256', hashlib.sha256(source.encode()).hexdigest())
source = source.replace('''            process.args([
                "default",''', '''            let mode = std::env::var("KHOR_WEBKIT_MODE").unwrap_or_else(|_| "default".into());
            process.args([
                &mode,''')
old = '''            answering.abort();
            let _ = answering.await;
            let facts = facts.lock().unwrap();
            eprintln!("native loopback {host}: {facts:?}");'''
new = '''            eprintln!("native loopback {host} BEFORE control: {:?}", facts.lock().unwrap());
            let control = tokio::time::timeout(std::time::Duration::from_secs(3), async {
                let mut sock = tokio::net::TcpStream::connect(format!("{host}:{port}")).await?;
                sock.write_all(format!("GET / HTTP/1.1\\r\\nHost: {host}:{port}\\r\\nConnection: close\\r\\n\\r\\n").as_bytes()).await?;
                let mut response = Vec::new();
                sock.read_to_end(&mut response).await?;
                Ok::<_, std::io::Error>(String::from_utf8_lossy(&response).into_owned())
            }).await;
            eprintln!("native loopback {host} raw TCP control: {control:?}");
            answering.abort();
            let _ = answering.await;
            let facts = facts.lock().unwrap();
            eprintln!("native loopback {host}: {facts:?}");'''
assert source.count(old) == 1
source = source.replace(old, new)
rust.write_text(source)
