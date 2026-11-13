//! Isolated, leased two-host network probe. Its controller owns the private
//! Unix socket; tickets travel only on that socket, never on stdout.
#[cfg(unix)]
mod unix {

    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use khor_node::Node;
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;
    use tokio::net::unix::OwnedWriteHalf;
    use tokio::time::{Instant, timeout, timeout_at};

    const CONTENT: &[u8] = b"khor isolated WAN fixture v1\n";

    fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str, String> {
        value[name]
            .as_str()
            .ok_or_else(|| format!("missing {name}"))
    }

    async fn reply(out: &mut OwnedWriteHalf, value: Value) -> Result<(), String> {
        let mut bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        timeout(Duration::from_secs(5), out.write_all(&bytes))
            .await
            .map_err(|_| "controller stopped reading".to_owned())?
            .map_err(|e| e.to_string())
    }

    fn peer(node: &Node, expected: &str) -> Result<String, String> {
        if expected == node.device_str() || expected.len() != 64 {
            return Err("invalid expected peer identity".to_owned());
        }
        let devices = node.devices()?;
        if !devices.iter().any(|d| d.id == expected)
            || devices
                .iter()
                .any(|d| d.id != expected && d.id != node.device_str())
        {
            return Err("roster does not contain exactly the isolated peer".to_owned());
        }
        Ok(devices
            .iter()
            .find(|d| d.id == expected)
            .unwrap()
            .name
            .clone())
    }

    async fn rpc(node: &Node, command: &Value, run: &str) -> Result<Value, String> {
        match field(command, "op")? {
            "mint" => Ok(json!({"ticket": node.invite()?})),
            "pair" => {
                let expected = field(command, "expected_id")?;
                let name = node.pair(field(command, "ticket")?).await?;
                peer(node, expected)?;
                Ok(json!({"name": name, "peer": expected}))
            }
            "devices" => Ok(Value::Array(
                node.devices()?
                    .into_iter()
                    .map(|d| json!({"id": d.id, "name": d.name}))
                    .collect(),
            )),
            "links" => serde_json::to_value(node.links().await.map_err(|e| format!("{e:?}"))?)
                .map_err(|e| e.to_string()),
            "read" => {
                let expected = field(command, "expected_id")?;
                let machine = peer(node, expected)?;
                if field(command, "machine")? != machine {
                    return Err("fixture machine does not match the expected identity".to_owned());
                }
                let path = Path::new(field(command, "fixture")?);
                if !path.is_absolute()
                    || path.file_name().and_then(|v| v.to_str()) != Some("fixture")
                {
                    return Err("only an isolated peer fixture may be listed".to_owned());
                }
                let marker = format!("wan-{run}.txt");
                let began = Instant::now();
                let (_, entries, truncated) = timeout(
                    Duration::from_secs(30),
                    node.ls_of(field(command, "machine")?, field(command, "fixture")?),
                )
                .await
                .map_err(|_| "fixture read deadline".to_owned())??;
                if truncated
                    || !entries
                        .iter()
                        .any(|e| e.name == marker && !e.dir && e.size == CONTENT.len() as u64)
                {
                    return Err("the isolated peer marker is absent".to_owned());
                }
                Ok(
                    json!({"peer": expected, "elapsed_us": began.elapsed().as_micros(), "marker_verified": true}),
                )
            }
            "read_content" => {
                let expected = field(command, "expected_id")?;
                let machine = peer(node, expected)?;
                if field(command, "machine")? != machine {
                    return Err("fixture machine does not match the expected identity".to_owned());
                }
                let fixture = Path::new(field(command, "fixture")?);
                if !fixture.is_absolute()
                    || fixture.file_name().and_then(|v| v.to_str()) != Some("fixture")
                {
                    return Err("only an isolated peer fixture may be read".to_owned());
                }
                let path = fixture.join(format!("wan-{run}.txt"));
                let path = path.to_str().ok_or("fixture path is not UTF-8")?;
                let began = Instant::now();
                let (total, _, bytes, cut) = timeout(
                    Duration::from_secs(30),
                    node.read_all(&machine, path, CONTENT.len() as u64),
                )
                .await
                .map_err(|_| "fixture content deadline".to_owned())??;
                if cut || total != CONTENT.len() as u64 || bytes.as_slice() != CONTENT {
                    return Err("the isolated peer marker content differs".to_owned());
                }
                Ok(
                    json!({"peer": expected, "elapsed_us": began.elapsed().as_micros(),
                    "marker_content_verified": true, "bytes": bytes.len()}),
                )
            }
            _ => Err("unsupported probe operation".to_owned()),
        }
    }

    async fn run(root: PathBuf, run: String, role: String) -> Result<(), String> {
        if run.len() != 16
            || !run.bytes().all(|b| b.is_ascii_hexdigit())
            || !matches!(role.as_str(), "mac" | "linux")
            || !root.is_absolute()
            || root.canonicalize().map_err(|e| e.to_string())? != root
        {
            return Err("invalid owned probe root or role".to_owned());
        }
        let metadata = std::fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
        let owner_path = root.join("owner.json");
        let owner_metadata = std::fs::symlink_metadata(&owner_path).map_err(|e| e.to_string())?;
        let owner: Value =
            serde_json::from_slice(&std::fs::read(&owner_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if !metadata.is_dir()
            || metadata.permissions().mode() & 0o777 != 0o700
            || !owner_metadata.is_file()
            || owner_metadata.permissions().mode() & 0o777 != 0o600
            || owner_metadata.uid() != metadata.uid()
            || owner["uid"] != metadata.uid()
            || owner["schema"] != 1
            || owner["owner"] != "khor feat/net wan"
            || owner["run_id"] != run
            || owner["role"] != role
            || std::env::var_os("KHOR_NETLAB").is_some()
            || std::env::var("KHOR_NO_MDNS").as_deref() != Ok("1")
        {
            return Err("probe ownership or network environment mismatch".to_owned());
        }
        let by = Instant::now() + Duration::from_secs(600);
        let socket = root.join("lease.sock");
        let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
        let (stream, _) = timeout(Duration::from_secs(15), listener.accept())
            .await
            .map_err(|_| "no controller lease".to_owned())?
            .map_err(|e| e.to_string())?;
        drop(listener);
        let (input, mut output) = stream.into_split();
        let mut commands = BufReader::new(input).lines();
        let home = root.join("home");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&home)
            .map_err(|e| e.to_string())?;
        let fixture = home.join("fixture");
        std::fs::create_dir(&fixture).map_err(|e| e.to_string())?;
        std::fs::write(fixture.join(format!("wan-{run}.txt")), CONTENT)
            .map_err(|e| e.to_string())?;
        let name = format!("wan-{role}");
        let resident = Node::open_as(home.clone(), &name)?;
        let id = resident.device_str().to_owned();
        let mut serving = tokio::spawn(resident.serve());
        let session = async {
            let ready = timeout(Duration::from_secs(90), async {
                while !home.join(".khor/endpoint.json").is_file() {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            });
            tokio::select! {
                ready = ready => ready.map_err(|_| "resident did not become ready".to_owned())?,
                _ = commands.next_line() => return Err("controller lease ended before readiness".to_owned()),
            }
            let client = Node::open_as(home.clone(), &name)?;
            reply(&mut output, json!({"seq": 0, "ok": true, "data": {
            "id": id, "name": client.name(), "fixture": fixture, "run_id": run, "role": role,
        }})).await?;
            loop {
                let line = timeout_at(by, commands.next_line())
                    .await
                    .map_err(|_| "probe absolute deadline".to_owned())?
                    .map_err(|e| e.to_string())?;
                let Some(line) = line else { return Ok(None) };
                if line.len() > 65536 {
                    return Err("oversized controller command".to_owned());
                }
                let command: Value = serde_json::from_str(&line)
                    .map_err(|_| "invalid controller JSON".to_owned())?;
                let seq = command["seq"].as_u64().ok_or("missing sequence")?;
                if command["op"] == "stop" {
                    return Ok(Some(seq));
                }
                let result = tokio::select! {
                    result = timeout_at(by, rpc(&client, &command, &run)) => result.map_err(|_| "probe absolute deadline".to_owned())?,
                    line = commands.next_line() => {
                        return match line.map_err(|e| e.to_string())? {
                            None => Ok(None),
                            Some(_) => Err("pipelined controller command".to_owned()),
                        };
                    },
                };
                let response = match result {
                    Ok(data) => json!({"seq": seq, "ok": true, "data": data}),
                    Err(_) => json!({"seq": seq, "ok": false, "error": "probe operation failed"}),
                };
                reply(&mut output, response).await?;
            }
        };
        let result: Result<Option<u64>, String> = timeout_at(by, session)
            .await
            .map_err(|_| "probe absolute deadline".to_owned())
            .and_then(|v| v);
        let closed = khor_node::link::stop_serving(&home).await;
        let joined = match timeout(Duration::from_secs(30), &mut serving).await {
            Ok(Ok(Ok(()))) => true,
            Ok(_) => false,
            Err(_) => {
                serving.abort();
                let _ = serving.await;
                false
            }
        };
        let clean = joined && matches!(closed, Ok(khor_node::khor_agent::RuntimeClose::Closed));
        std::fs::remove_file(&socket).map_err(|e| e.to_string())?;
        eprintln!(
            "[khor-wan-final] {}",
            json!({"run_id": run, "role": role, "resident_joined": clean})
        );
        if let Ok(Some(seq)) = &result {
            reply(
                &mut output,
                json!({"seq": seq, "ok": clean, "data": {"resident_joined": clean}}),
            )
            .await?;
        }
        if !clean {
            return Err("resident cleanup was not confirmed".to_owned());
        }
        result.map(|_| ())
    }

    #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
    pub async fn main() -> Result<(), String> {
        let args: Vec<_> = std::env::args().collect();
        if args.len() != 4 {
            return Err("usage: net_wan_probe <owned-root> <run-id> <mac|linux>".to_owned());
        }
        run(PathBuf::from(&args[1]), args[2].clone(), args[3].clone()).await
    }
}

#[cfg(unix)]
fn main() -> Result<(), String> {
    unix::main()
}

#[cfg(not(unix))]
fn main() {
    eprintln!("net_wan_probe requires Unix");
    std::process::exit(2);
}
