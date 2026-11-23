//! Select an executable and its compatibility contract before opening user data or detaching a host.
mod activation;
#[cfg(test)]
mod compatibility_tests;

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
// A newly copied Intel image took 12.7 seconds in Rosetta before reaching
// main on a fresh macOS runner. Keep the inspection bounded, but allow cold
// code validation/translation to finish before rejecting a valid runtime.
const INFO_TIMEOUT: Duration = Duration::from_secs(if cfg!(target_os = "macos") { 30 } else { 3 });

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Release {
    schema: u8,
    version: String,
    role: Role,
    platform: String,
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
    embedded.insert("pi".into(), env!("KHOR_AGENT_BUNDLE_BLAKE3").to_owned());
    let wire = include_str!("proto.rs").replace("\r\n", "\n");
    let fingerprint = format!("{:x}", Sha256::digest(wire.as_bytes()));
    // These exact audited revisions add FileResource, SessionImport, then
    // Execution and OpenTerminalOnce, then the optional Windows shell choice.
    // MessagePack carries operation names; existing words and positional
    // fields are unchanged, including the six-field LegacyShellStart, which
    // translates to the default shell.
    // Old peers refuse the new words before any command can execute or lose
    // its startup arguments. Existing viewer connections remain protocol v5;
    // automation requires a separate v6 control connection. Keep published
    // live residents eligible for their existing capabilities.
    // Every other source revision keeps its own fingerprint and fails closed.
    let compatible_fingerprint = match fingerprint.as_str() {
        "f317c5ec5fa618a54ef5848016cc23219c37cbc39d6abd52cd4b361526d21f29"
        | "f0c20d8ece8dda5bab4025bf550149bba8be5a4b399459f656a9449a566f4f4f"
        | "4c41bbe5e02bf1883c5cfa0764f53d862accdff50a2714516a3daa7caafc8773"
        | "80971cd65f85a60ad80c53f731ab7198fc3e59158865bf3b358113a4f209ef42" => {
            "7bdf9a8cedcc26b783f3a6714899df875cbcb429e2f1ee64aa5a7d065e75fcda"
        }
        other => other,
    };
    Ok(Info {
        release: Release {
            schema: 1,
            version: crate::VERSION.into(),
            role,
            platform: khor_runtime::platform(),
            contract: Contract {
                terminal: crate::host::HOST_VERSION,
                gui: crate::gui_host::GUI_GREETING.into(),
                node: format!(
                    "{}:sha256:{}",
                    String::from_utf8_lossy(khor_net::endpoint::ALPN),
                    compatible_fingerprint
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
    #[cfg(windows)]
    if let Err(error) = crate::win_process::privatize_standard_handles() {
        eprintln!("{error}");
        std::process::exit(1);
    }
    crate::startup_trace::begin();
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("_verify_release")) {
        let result = (|| -> Result<String, String> {
            let args = std::env::args_os().skip(2).collect::<Vec<_>>();
            let [document, signature, archives] = args.as_slice() else {
                return Err(
                    "release verification expects manifest, signature, and archive directory"
                        .into(),
                );
            };
            let release = khor_update::TrustRoot::bundled()?.authenticate_files(
                Path::new(document),
                Path::new(signature),
                Path::new(archives),
            )?;
            serde_json::to_string(&release.manifest().identity).map_err(|e| e.to_string())
        })();
        match result {
            Ok(identity) => {
                println!("{identity}");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(78);
            }
        }
    }
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("_release_info")) {
        match crate::updates::program_info()
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
    #[cfg(unix)]
    if let Some(result) = crate::cli_bootstrap::if_requested() {
        match result {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    if let Err(error) = khor_runtime::enter_current() {
        eprintln!("cannot enter runtime: {error}");
        std::process::exit(1);
    }
    crate::startup_trace::mark("runtime-entered");
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    if let Err(error) = crate::updates::recover_current_installation() {
        eprintln!("{error}");
        std::process::exit(78);
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    if let Err(error) = activation::check_local_contracts() {
        eprintln!("{error}");
        std::process::exit(78);
    }
    crate::startup_trace::mark("contracts-checked");
}

fn inspect(program: &Path) -> Result<Info, String> {
    let bytes = probe(program, &[std::ffi::OsStr::new(INFO)])?;
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

pub(crate) fn probe(program: &Path, args: &[&std::ffi::OsStr]) -> Result<Vec<u8>, String> {
    let mut command = Command::new(program);
    command.args(args).env_clear();
    for name in ["SYSTEMROOT", "WINDIR", "KHOR_RUNTIME_DIR"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let (mut child, mut pipes) =
        Session::spawn(command, Arc::new(Control::default())).map_err(|error| error.to_string())?;
    drop(pipes.stdin);
    let by = Instant::now() + INFO_TIMEOUT;
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
    Ok(bytes)
}

pub(crate) fn release_contract() -> Result<Contract, String> {
    Ok(current()?.release.contract)
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
    pub(crate) fn generation(&self) -> &std::ffi::OsStr {
        self.image
            .directory()
            .file_name()
            .expect("owned runtime generation")
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
    crate::startup_trace::mark("image-current");
    // **The same file, already published.** Inspecting and publishing a
    // source program costs ~600 ms here (a probe process, then a hash and a
    // copy of 130 MB), and it repeats on every launch because
    // `Store::image_for_program` only recognises a program that is already
    // a managed image. `SourceCache` remembers which image came out of
    // which file object — by the operating system's identity for that
    // file, not by name — and a hit still has to match the running
    // release's contract and version, so an upgrade is never skipped.
    // Everything else (a changed file, a missing image, an unreadable
    // cache) is a miss, and the full path below is the ordinary one.
    let cache = khor_runtime::SourceCache::local().ok();
    if let Some(cache) = &cache
        && let Ok(Some(image)) = cache.lookup(source)
        && image.contract() == &expected.contract
        && image.version() == expected.version
    {
        crate::startup_trace::mark("image-reused");
        return Ok(HostImage {
            image,
            source: source.into(),
        });
    }
    let info = inspect_supported(source)?;
    crate::startup_trace::mark("image-inspected");
    if expected.platform != info.release.platform
        || !expected.contract.accepts(&info.release.contract)
    {
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
    if let Some(image) = Store::image_for_program(&executable).map_err(|error| error.to_string())? {
        crate::startup_trace::mark("image-cached");
        if image.role() != selected.role || image.contract() != &selected.contract {
            return Err("managed runtime contract differs from its executable".into());
        }
        return Ok(HostImage {
            image,
            source: source.into(),
        });
    }
    crate::startup_trace::mark("image-miss");
    let payload = Source::File(&executable);
    #[cfg(target_os = "macos")]
    let payload =
        if selected.role == Role::DesktopHost && executable.ends_with("Contents/MacOS/khor-gui") {
            Source::MacApp(
                executable
                    .ancestors()
                    .nth(3)
                    .ok_or("desktop host bundle is incomplete")?,
            )
        } else {
            payload
        };
    let image = Store::local()
        .map_err(|error| error.to_string())?
        .publish_checked(
            selected.role,
            &selected.version,
            selected.contract.clone(),
            selected.embedded.clone(),
            payload,
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
    // Remember what this file published as, so the next launch in this
    // release reuses it instead of hashing 130 MB again. A failure here is
    // only a missed optimisation, never a reason this launch fails.
    if let Some(cache) = &cache {
        let _ = cache.remember(source, &image);
    }
    Ok(HostImage {
        image,
        source: source.into(),
    })
}
