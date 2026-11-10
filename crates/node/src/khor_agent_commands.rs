//! A command owned by the current MCP worker and recorded under its parent turn.
use super::*;
use crate::{delegation::TaskRef, remote_cancel};
use std::io::Read;

const OUTPUT: usize = 1024;
const RECEIPT: u64 = 16 * 1024;

#[derive(Serialize, Deserialize)]
struct Receipt {
    fingerprint: String,
    title: String,
    state: String,
    exit: Option<i32>,
    stdout: String,
    stderr: String,
    truncated: bool,
}

fn tasks(root: &Path, run: &str) -> PathBuf {
    directory(root)
        .join("commands")
        .join(blake3::hash(run.as_bytes()).to_hex().as_str())
}

pub(crate) fn execute(
    node: &Node,
    parent: &TaskRef,
    request: &str,
    command: &str,
    cwd: &str,
) -> Result<String, String> {
    if parent.home != node.device() || !safe_token(request) || command.len() > 32 * 1024 {
        return Err(msg::agent_arguments().into());
    }
    let folder = if cwd.trim().is_empty() {
        node.root()
    } else {
        Path::new(cwd)
    };
    let fingerprint =
        blake3::hash(&serde_json::to_vec(&(command, folder)).map_err(|e| e.to_string())?)
            .to_hex()
            .to_string();
    let dir = tasks(node.root(), &parent.id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let key = blake3::hash(request.as_bytes()).to_hex().to_string();
    let path = dir.join(format!("{key}.json"));
    let _lease = remote_cancel::gate(&path.with_extension("lock"))?;
    if path.try_exists().map_err(|e| e.to_string())? {
        let prior: Receipt = remote_cancel::read(&path, RECEIPT)?;
        if prior.fingerprint != fingerprint {
            return Err(msg::console_token_conflict().into());
        }
        return serde_json::to_string(&prior).map_err(|e| e.to_string());
    }
    if !folder.is_absolute() || !folder.is_dir() {
        return Err(msg::no_such_dir(folder.display()).into());
    }
    if remote_cancel::names(&dir, 128)?.len() >= 32 {
        return Err("Command task limit for this turn".into());
    }
    let mut receipt = Receipt {
        fingerprint,
        title: command
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(160)
            .collect(),
        state: "working".into(),
        exit: None,
        stdout: String::new(),
        stderr: String::new(),
        truncated: false,
    };
    // Claim the exact input before starting a process. An uncertain result is
    // never permission to run the same command again.
    remote_cancel::write(&path, &receipt).map_err(|(why, _)| why)?;
    match run(command, folder) {
        Ok((status, stdout, stderr, truncated)) => {
            receipt.state = if status.success() {
                "completed"
            } else {
                "failed"
            }
            .into();
            receipt.exit = status.code();
            receipt.stdout = stdout;
            receipt.stderr = stderr;
            receipt.truncated = truncated;
        }
        Err(error) => {
            receipt.state = "failed".into();
            receipt.stderr = error.chars().take(OUTPUT / 4).collect();
        }
    }
    remote_cancel::write(&path, &receipt).map_err(|(why, _)| why)?;
    serde_json::to_string(&receipt).map_err(|e| e.to_string())
}

fn drain(mut input: impl Read) -> std::io::Result<(String, bool)> {
    let mut kept = Vec::new();
    let mut cut = false;
    let mut buffer = [0; 8192];
    loop {
        let size = input.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        let keep = size.min(OUTPUT - kept.len());
        kept.extend_from_slice(&buffer[..keep]);
        cut |= keep != size;
    }
    Ok((String::from_utf8_lossy(&kept).into_owned(), cut))
}

fn run(
    command: &str,
    folder: &Path,
) -> Result<(std::process::ExitStatus, String, String, bool), String> {
    #[cfg(windows)]
    let mut process = Command::new("cmd.exe");
    #[cfg(windows)]
    process.args(["/d", "/s", "/c", command]);
    #[cfg(not(windows))]
    let mut process = Command::new("sh");
    #[cfg(not(windows))]
    process.args(["-c", command]);
    // Stay within the MCP worker's process group / Job Object, which
    // already owns cancellation, timeout and confirmed reaping.
    let mut child = process
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    std::thread::scope(|scope| {
        let output = scope.spawn(|| drain(stdout));
        let errors = scope.spawn(|| drain(stderr));
        let status = child.wait().map_err(|e| e.to_string())?;
        let (stdout, stdout_cut) = output
            .join()
            .map_err(|_| "Command stdout reader failed")?
            .map_err(|e| e.to_string())?;
        let (stderr, stderr_cut) = errors
            .join()
            .map_err(|_| "Command stderr reader failed")?
            .map_err(|e| e.to_string())?;
        Ok((status, stdout, stderr, stdout_cut || stderr_cut))
    })
}

pub(super) fn summaries(
    node: &Node,
    board: &str,
    runs: &[CommandRecord],
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::new();
    for run in runs {
        let dir = tasks(node.root(), &run.id);
        let names = remote_cancel::names(&dir, 128)?;
        if names.is_empty() {
            continue;
        }
        let saved: Saved = remote_cancel::read(&file(node.root(), &run.id), 8 * 1024 * 1024)?;
        if saved.board != board
            || saved.record.id != run.id
            || saved.record.owner != node.device_str()
        {
            continue;
        }
        for key in names {
            let path = dir.join(&key);
            let receipt: Receipt = remote_cancel::read(&path, RECEIPT)?;
            let state = if receipt.state == "working" {
                if saved.record.status == "cancelled" {
                    "canceled"
                } else if remote_cancel::gate(&path.with_extension("lock"))
                    .as_ref()
                    .err()
                    .is_some_and(|why| why == "Busy")
                {
                    "working"
                } else {
                    "unavailable"
                }
            } else {
                &receipt.state
            };
            let result = [receipt.stdout.trim(), receipt.stderr.trim()]
                .into_iter()
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            rows.push(json!({"run":run.id,"key":key,"agent":receipt.title,"machine":node.shown_name(),
                "state":state,"task":null,"context":null,"result":result,
                "can_cancel":false,"can_retry":false,"exit":receipt.exit,"truncated":receipt.truncated}));
            if rows.len() >= 24 {
                return Ok(rows);
            }
        }
    }
    Ok(rows)
}
