//! Satellites: machines only ssh can reach, spoken for by a sponsor
//! (附属机器批; docs/NET.md 卫星).
//!
//! 卫星 = 看得见、够得着,但不是它自己在说话;行星 = 它自己在说话;
//! 转正 = 一条命令. A satellite sits in the device table like any
//! machine — id, name, a face — but its home is its **sponsor**: every
//! operation routes there and the sponsor executes it over ssh.
//!
//! # Only the user's own ssh road, never khor's
//!
//! Everything here shells out to `ssh` so the user's `~/.ssh/config`,
//! agent and known_hosts do all the deciding. khor stores no
//! credential, opens no socket of its own, and never calls
//! `ssh-keyscan`: the identity comes out of **known_hosts** — the trust
//! the user already extended — and a host they never connected to is
//! refused with directions, not probed behind their back.
//!
//! # The id is the host key's SHA256, as hex
//!
//! A SHA256 is exactly 32 bytes, so the fingerprint wears the same
//! shape as every device id (64 hex chars): `DeviceId` parses it, the
//! avatar mint seeds from it (a satellite has a face for free), and no
//! reader needs a second id grammar. It is not an iroh key — nothing
//! can dial it — and the row's empty `addrs` keeps every sync loop
//! walking past it, which is exactly right.
//!
//! # BatchMode everywhere
//!
//! Every ssh this module runs carries `-o BatchMode=yes`: a password
//! prompt inside a serve is a hang nobody can answer. A host that needs
//! interactive auth fails fast with ssh's own words, which the caller
//! wraps in directions.

use std::path::PathBuf;
use std::process::Command;

use khor_catalog::msg;

/// The test door (`KHOR_TMUX_SOCKET`'s precedent): a scratch ssh world
/// — a `config` and a `known_hosts` under one directory — so
/// verification never reads or writes the user's real `~/.ssh`. It has
/// to be an explicit door rather than a `HOME` override because OpenSSH
/// resolves `~` through the passwd database, not the environment
/// (measured: a HOME-overridden probe read the real config and sent the
/// alias to a proxy's fake-ip DNS). Production leaves it unset and
/// every call rides the user's own road, which is the whole point.
fn ssh_dir() -> Option<PathBuf> {
    std::env::var_os("KHOR_SSH_DIR").map(Into::into)
}

/// What one adoption probe learned over a single connection.
#[derive(Debug, Clone)]
pub struct Probe {
    /// `uname -s` (or "unknown" where uname is missing) — recorded for
    /// the report, not for branching: platform quirks are handled where
    /// they bite (the sampler), never pre-sorted here.
    pub uname: String,
    /// Whether tmux is on the satellite — what decides if a terminal
    /// there can be made persistent (docs/SESSION.md 例外墓碑).
    pub tmux: bool,
}

/// The flags every satellite ssh shares. `ConnectTimeout` because a
/// dead host must cost seconds, not a TCP stack's patience.
fn ssh_base(host: &str) -> Command {
    let mut c = Command::new("ssh");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    c.arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=8")
        .arg(host);
    c
}

/// One connection, two answers: what it is, and whether tmux is there.
pub fn probe(host: &str) -> Result<Probe, String> {
    let out = ssh_base(host)
        .arg("uname -s 2>/dev/null || echo unknown; command -v tmux >/dev/null 2>&1 && echo tmux-yes || echo tmux-no")
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let uname = lines.next().unwrap_or("unknown").trim().to_owned();
    let tmux = lines.next().is_some_and(|l| l.trim() == "tmux-yes");
    Ok(Probe { uname, tmux })
}

/// How often a sponsor samples each of its satellites. A minute, not
/// the 5s sync beat: every sample is an ssh connection and a one-second
/// on-host measurement, and a reading of a machine nobody khor runs on
/// does not need a heartbeat's cadence — the age rides the reading
/// anyway (docs/SESSION.md 离线: how fresh is its own axis).
pub const SAMPLE_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// One shell, both platforms, key=value lines out. The one-second sleep
/// inside is the CPU delta window (a utilisation is a rate; a single
/// glance at /proc/stat is a lifetime average). Every section tolerates
/// its own absence — a missing nvidia-smi prints nothing and the parser
/// reads nothing, the disk falling over leaves the reading without a
/// disk, never without a row.
const SAMPLE_SH: &str = r#"
U=$(uname -s)
echo cores=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 1)
case "$U" in
Linux)
  c1=$(head -1 /proc/stat); sleep 1; c2=$(head -1 /proc/stat)
  echo "$c1
$c2" | awk 'NR==1{for(i=2;i<=NF;i++)t1+=$i; i1=$5} NR==2{for(i=2;i<=NF;i++)t2+=$i; i2=$5; dt=t2-t1; if(dt>0) printf "cpu_pct=%.1f\n", (dt-(i2-i1))*100/dt}'
  awk '/MemTotal/{t=$2} /MemAvailable/{a=$2} END{printf "mem_total=%d\nmem_used=%d\n", t*1024, (t-a)*1024}' /proc/meminfo
  ;;
Darwin)
  top -l 2 -n 0 -s 1 2>/dev/null | awk '/CPU usage/{u=$3; s=$5} END{gsub("%","",u); gsub("%","",s); printf "cpu_pct=%.1f\n", u+s}'
  T=$(sysctl -n hw.memsize)
  P=$(sysctl -n hw.pagesize)
  vm_stat | awk -v t="$T" -v p="$P" '/Pages active/{a=$3} /Pages wired/{w=$4} /occupied by compressor/{c=$5} END{gsub("\\.","",a); gsub("\\.","",w); gsub("\\.","",c); printf "mem_total=%d\nmem_used=%d\n", t, (a+w+c)*p}'
  ;;
esac
df -k "$HOME" 2>/dev/null | awk 'NR==2{printf "disk_total=%d\ndisk_used=%d\n", ($3+$4)*1024, $3*1024}'
if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>/dev/null | awk -F', *' '{n++; u+=$1; mu+=$2; mt+=$3} END{if(n>0) printf "gpu_pct=%.1f\ngpu_cards=%d\nvram_used=%d\nvram_total=%d\n", u/n, n, mu*1048576, mt*1048576}'
fi
"#;

/// One reading of the satellite, taken over the user's ssh road — the
/// same `khor_core::Vitals` shape every machine reports about itself,
/// with `version: None`: no khor runs there, and saying so is the row's
/// honest gap, never a guessed number.
pub fn sample(host: &str) -> Result<khor_core::Vitals, String> {
    let out = ssh_base(host)
        .arg(SAMPLE_SH)
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !out.status.success() {
        return Err(msg::sat_cant_reach(host, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let get = |key: &str| -> Option<f64> {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('=')?.trim().parse().ok())
    };
    let fill = |used: &str, total: &str| -> Option<khor_core::Fill> {
        Some(khor_core::Fill { used: get(used)? as u64, total: get(total)? as u64 })
    };
    Ok(khor_core::Vitals {
        cpu_pct: get("cpu_pct").unwrap_or(0.0) as f32,
        cores: get("cores").unwrap_or(1.0) as u32,
        mem: fill("mem_used", "mem_total").unwrap_or(khor_core::Fill { used: 0, total: 0 }),
        disk: fill("disk_used", "disk_total"),
        gpu: get("gpu_pct").map(|pct| khor_core::Gpu {
            util_pct: pct as f32,
            cards: get("gpu_cards").unwrap_or(1.0) as u32,
            mem: fill("vram_used", "vram_total"),
        }),
        version: None,
    })
}

/// The satellite's stable id: the SHA256 of its host key, hex — read
/// out of known_hosts (`ssh-keygen -F`), the module head's rule.
///
/// The alias the user types may not be what known_hosts is keyed by, so
/// a miss retries with the canonical hostname `ssh -G` resolves the
/// alias to — the same resolution the connection itself would use.
pub fn fingerprint_id(host: &str) -> Result<String, String> {
    if let Some(id) = known_host_id(host)? {
        return Ok(id);
    }
    if let Some(real) = canonical_hostname(host) {
        if real != host {
            if let Some(id) = known_host_id(&real)? {
                return Ok(id);
            }
        }
    }
    Err(msg::sat_not_in_known_hosts(host))
}

/// One name's known_hosts entry, fingerprinted — `None` when the file
/// has never seen it.
fn known_host_id(name: &str) -> Result<Option<String>, String> {
    let mut find = Command::new("ssh-keygen");
    if let Some(dir) = ssh_dir() {
        find.arg("-f").arg(dir.join("known_hosts"));
    }
    let found = find
        .arg("-F")
        .arg(name)
        .output()
        .map_err(|e| msg::ssh_wont_run(e))?;
    if !found.status.success() {
        return Ok(None);
    }
    // Key lines only; `-F` prefixes each match with a `# Host …` comment.
    let keys: String = String::from_utf8_lossy(&found.stdout)
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    if keys.is_empty() {
        return Ok(None);
    }
    let printed = {
        use std::io::Write;
        let mut c = Command::new("ssh-keygen")
            .arg("-lf")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| msg::ssh_wont_run(e))?;
        c.stdin
            .take()
            .ok_or_else(|| msg::ssh_wont_run("no stdin"))?
            .write_all(keys.as_bytes())
            .map_err(|e| msg::ssh_wont_run(e))?;
        let out = c.wait_with_output().map_err(|e| msg::ssh_wont_run(e))?;
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    // First fingerprint line wins: `256 SHA256:<b64> name (ED25519)`.
    // known_hosts may hold several key types for one host; any of them
    // identifies it, and "the first" is stable because -F preserves
    // file order.
    let Some(b64) = printed
        .lines()
        .find_map(|l| l.split_whitespace().nth(1)?.strip_prefix("SHA256:").map(str::to_owned))
    else {
        return Ok(None);
    };
    Ok(sha256_b64_to_hex(&b64))
}

/// What known_hosts would key this alias under — the hostname the
/// user's config resolves it to, spelled the way ssh itself records it:
/// bare for port 22, `[host]:port` for any other port.
fn canonical_hostname(host: &str) -> Option<String> {
    let mut c = Command::new("ssh");
    if let Some(dir) = ssh_dir() {
        c.arg("-F").arg(dir.join("config"));
    }
    let out = c.arg("-G").arg(host).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(name).map(|h| h.trim().to_owned()))
    };
    let hostname = field("hostname ")?;
    match field("port ").as_deref() {
        Some("22") | None => Some(hostname),
        Some(port) => Some(format!("[{hostname}]:{port}")),
    }
}

/// OpenSSH prints SHA256 fingerprints as unpadded base64 of the 32
/// digest bytes; the device table wants those bytes as hex. `None` on
/// anything that does not decode to exactly 32 bytes — a truncated or
/// foreign fingerprint must not mint a malformed id.
fn sha256_b64_to_hex(b64: &str) -> Option<String> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD_NO_PAD;
    let bytes = engine.decode(b64).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    Some(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fingerprint-to-id math on a fixed digest: 32 bytes in, the
    /// device table's 64-hex shape out — and the two malformed shapes
    /// (bad base64, wrong length) mint nothing rather than a wrong id.
    #[test]
    fn a_sha256_fingerprint_becomes_a_device_shaped_id() {
        let b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD_NO_PAD.encode([0xabu8; 32])
        };
        let id = sha256_b64_to_hex(&b64).expect("a whole digest mints");
        assert_eq!(id.len(), 64, "the id must wear the device shape");
        assert_eq!(id, "ab".repeat(32));
        assert_eq!(sha256_b64_to_hex("not-base64!"), None, "garbage mints nothing");
        let short = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD_NO_PAD.encode([1u8; 16])
        };
        assert_eq!(sha256_b64_to_hex(&short), None, "a wrong-sized digest mints nothing");
    }
}
