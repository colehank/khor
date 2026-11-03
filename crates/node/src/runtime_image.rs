//! Select an executable and its compatibility contract before opening user data or detaching a host.
mod activation;

use crate::agent_process::{Control, Session};
use khor_runtime::{Contract, Image, Role, Source, Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::Arc,
    time::{Duration, Instant},
};

pub const INSTALL_SOURCE: &str = "KHOR_RUNTIME_INSTALL_SOURCE";
const INFO: &str = "_runtime_info";
const INFO_LIMIT: u64 = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    schema: u8,
    version: String,
    role: Role,
    contract: Contract,
    embedded: BTreeMap<String, String>,
    portable: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Info {
    release: Release,
    executable: PathBuf,
}

fn current() -> Result<Info, String> {
    let executable = crate::self_exe()?;
    let role = match executable.file_name().and_then(|name| name.to_str()) {
        Some("khor-gui" | "khor-gui.exe") => Role::DesktopHost,
        _ => Role::Cli,
    };
    #[allow(unused_mut)]
    let mut embedded = BTreeMap::new();
    #[cfg(khor_embedded_agent)]
    embedded.insert(
        "pi".into(),
        blake3::hash(include_bytes!(env!("KHOR_AGENT_BUNDLE")))
            .to_hex()
            .to_string(),
    );
    let wire = include_str!("proto.rs").replace("\r\n", "\n");
    Ok(Info {
        release: Release {
            schema: 1,
            version: crate::VERSION.into(),
            role,
            contract: Contract {
                terminal: crate::host::HOST_VERSION,
                gui: crate::gui_host::GUI_GREETING.into(),
                node: format!(
                    "{}:sha256:{:x}",
                    String::from_utf8_lossy(khor_net::endpoint::ALPN),
                    Sha256::digest(wire.as_bytes())
                ),
                store: 1,
                journal: crate::session_content::journal::VERSION,
                agent: 1,
            },
            embedded,
            portable: cfg!(khor_embedded_agent),
        },
        executable,
    })
}

/// Every native face and the re-executing integration harness enter here before Node::open.
pub fn process_entry() {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new(INFO)) {
        match current()
            .and_then(|info| serde_json::to_string(&info).map_err(|error| error.to_string()))
        {
            Ok(info) => {
                println!("{info}");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("_runtime_check")) {
        let result = match std::env::args_os().skip(2).collect::<Vec<_>>().as_slice() {
            [other] => activation::check(Path::new(other)),
            _ => Err("runtime activation check requires one existing program".into()),
        };
        match result {
            Ok(()) => {
                println!("compatible");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(78);
            }
        }
    }
    if let Err(error) = khor_runtime::enter_current() {
        eprintln!("cannot enter runtime: {error}");
        std::process::exit(1);
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    if let Err(error) = activation::check_local_contracts() {
        eprintln!("{error}");
        std::process::exit(78);
    }
}

fn inspect(program: &Path) -> Result<Info, String> {
    let mut command = Command::new(program);
    command.arg(INFO).env_clear();
    for name in ["SYSTEMROOT", "WINDIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let (mut child, mut pipes) =
        Session::spawn(command, Arc::new(Control::default())).map_err(|error| error.to_string())?;
    drop(pipes.stdin);
    let by = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.poll_exit() {
            break status.map_err(|error| error.to_string())?;
        }
        if Instant::now() >= by {
            child.finish(true).map_err(|error| error.to_string())?;
            return Err("runtime inspection timed out".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    if !status.success() {
        child.finish(false).map_err(|error| error.to_string())?;
        return Err("program did not provide a runtime contract".into());
    }
    let mut bytes = Vec::new();
    let read = pipes
        .stdout
        .by_ref()
        .take(INFO_LIMIT + 1)
        .read_to_end(&mut bytes);
    child.finish(false).map_err(|error| error.to_string())?;
    read.map_err(|error| error.to_string())?;
    if bytes.len() as u64 > INFO_LIMIT {
        return Err("runtime contract exceeds its byte limit".into());
    }
    let info: Info =
        serde_json::from_slice(&bytes).map_err(|_| "invalid runtime contract".to_owned())?;
    if info.release.schema != 1
        || info.release.version.is_empty()
        || info.release.version.len() > 64
        || !info.executable.is_absolute()
        || info.release.role == Role::Agent
    {
        return Err("unsupported runtime contract".into());
    }
    Ok(info)
}

fn inspect_supported(program: &Path) -> Result<Info, String> {
    match activation::legacy_info(program)? {
        Some(info) => Ok(info),
        None => inspect(program),
    }
}

pub struct HostImage {
    image: Image,
    source: PathBuf,
}
impl HostImage {
    pub fn path(&self) -> &Path {
        self.image.path()
    }
    pub fn version(&self) -> &str {
        self.image.version()
    }
    pub(crate) fn prepare_command(&self, command: &mut Command) -> io::Result<()> {
        if command.get_program() != self.path() {
            return Err(io::Error::other("host command names another runtime"));
        }
        self.image.lease().attach_host(command)?;
        command.env(INSTALL_SOURCE, &self.source);
        Ok(())
    }
    pub fn spawn(&self, command: &mut Command) -> io::Result<Child> {
        self.prepare_command(command)?;
        command.spawn()
    }
    #[cfg(unix)]
    pub fn exec(&self, command: &mut Command) -> io::Error {
        use std::os::unix::process::CommandExt;
        if let Err(error) = self.prepare_command(command) {
            return error;
        }
        command.exec()
    }
}

pub fn host() -> Result<HostImage, String> {
    host_at(&crate::installed_exe()?)
}

pub(crate) fn running_host() -> Result<HostImage, String> {
    let mut image = host_at(&crate::self_exe()?)?;
    image.source = crate::installed_exe()?;
    Ok(image)
}

#[cfg(khor_embedded_agent)]
pub(crate) fn agent(bundle: &[u8]) -> Result<Image, String> {
    let release = current()?.release;
    Store::local()
        .map_err(|error| error.to_string())?
        .publish(
            Role::Agent,
            &release.version,
            release.contract,
            release.embedded,
            Source::Bytes(bundle),
        )
        .map_err(|error| error.to_string())
}

pub fn host_at(source: &Path) -> Result<HostImage, String> {
    let expected = current()?.release;
    let info = inspect_supported(source)?;
    if !expected.contract.accepts(&info.release.contract) {
        return Err(
            "runtime protocols or storage format are incompatible; existing sessions were retained"
                .into(),
        );
    }
    if cfg!(khor_embedded_agent) && !info.release.portable {
        return Err("release runtime is missing its embedded resources".into());
    }
    let selected = info.release;
    let executable = std::fs::canonicalize(&info.executable).map_err(|error| error.to_string())?;
    let image = Store::local()
        .map_err(|error| error.to_string())?
        .publish_checked(
            selected.role,
            &selected.version,
            selected.contract.clone(),
            selected.embedded.clone(),
            Source::File(&executable),
            |image| {
                let observed = inspect_supported(image.path()).map_err(io::Error::other)?;
                if observed.release != selected {
                    return Err(io::Error::other(
                        "runtime changed between inspection and publication",
                    ));
                }
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(HostImage {
        image,
        source: source.into(),
    })
}
