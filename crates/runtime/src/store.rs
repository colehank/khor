use crate::{Contract, Lease, Role, local};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const OWNER: &[u8] = b"khor-runtime-v1\n";
const MANIFEST_LIMIT: u64 = 16 * 1024;

#[derive(Clone, Copy)]
pub struct Limits {
    pub cache_bytes: u64,
    pub free_bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            cache_bytes: 4 * 1024 * 1024 * 1024,
            free_bytes: 1024 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Source<'a> {
    File(&'a Path),
    Bytes(&'a [u8]),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    schema: u8,
    role: Role,
    version: String,
    platform: String,
    contract: Contract,
    digest: String,
    bytes: u64,
    embedded: BTreeMap<String, String>,
}
impl Identity {
    fn id(&self) -> io::Result<String> {
        Ok(blake3::hash(&serde_json::to_vec(self)?)
            .to_hex()
            .to_string())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    roles: BTreeMap<Role, Vec<String>>,
}
impl Index {
    fn retain(&mut self, role: Role, id: &str) {
        let recent = self.roles.entry(role).or_default();
        recent.retain(|entry| entry != id);
        recent.insert(0, id.into());
        recent.truncate(2);
    }
    fn protected(&self) -> BTreeSet<&str> {
        self.roles.values().flatten().map(String::as_str).collect()
    }
}

pub struct Image {
    path: PathBuf,
    identity: Identity,
    lease: Lease,
}
impl Image {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn contract(&self) -> &Contract {
        &self.identity.contract
    }
    pub fn version(&self) -> &str {
        &self.identity.version
    }
    pub fn role(&self) -> Role {
        self.identity.role
    }
    pub fn embedded(&self) -> &BTreeMap<String, String> {
        &self.identity.embedded
    }
    pub fn lease(&self) -> &Lease {
        &self.lease
    }
}

pub struct Store {
    root: PathBuf,
    limits: Limits,
}

#[derive(Default, Debug)]
pub struct Collection {
    pub removed: Vec<String>,
    pub occupied: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Usage {
    pub retained_bytes: u64,
    pub active_old_bytes: u64,
    pub idle_bytes: u64,
    pub unknown_items: u64,
}

impl Store {
    pub fn local() -> io::Result<Self> {
        Self::open(local::local_directory()?, Limits::default())
    }

    pub fn open(root: PathBuf, limits: Limits) -> io::Result<Self> {
        if !root.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime directory must be absolute",
            ));
        }
        if !root.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                match fs::DirBuilder::new().mode(0o700).create(&root) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => return Err(error),
                }
            }
            #[cfg(windows)]
            fs::create_dir_all(&root)?;
        }
        local::validate(&root)?;
        let store = Self {
            root: fs::canonicalize(root)?,
            limits,
        };
        let _catalog = store.catalog()?;
        let owner = store.root.join("owner");
        if !owner.exists() {
            for entry in fs::read_dir(&store.root)? {
                if entry?.file_name() != "catalog" {
                    return Err(io::Error::other(
                        "refusing to adopt a nonempty unowned runtime directory",
                    ));
                }
            }
        }
        match OpenOptions::new().write(true).create_new(true).open(&owner) {
            Ok(mut file) => {
                file.write_all(OWNER)?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
        if read_bounded(&owner, OWNER.len() as u64)? != OWNER {
            return Err(io::Error::other(
                "runtime directory has another owner or schema",
            ));
        }
        Ok(store)
    }

    pub fn lease_for_program(program: &Path) -> io::Result<Option<Lease>> {
        Ok(Self::image_for_program(program)?.map(|image| image.lease))
    }

    pub fn image_for_program(program: &Path) -> io::Result<Option<Image>> {
        if !program.is_absolute() {
            return Ok(None);
        }
        let Some(directory) = program.parent() else {
            return Ok(None);
        };
        let Some(id) = directory
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|id| valid_id(id))
        else {
            return Ok(None);
        };
        let Some(root) = directory.parent() else {
            return Ok(None);
        };
        match read_bounded(&root.join("owner"), OWNER.len() as u64) {
            Ok(bytes) if bytes == OWNER => {}
            Ok(_) => return Err(io::Error::other("unsupported runtime owner")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
        let store = Self::open(root.into(), Limits::default())?;
        let selected = store.hold(id)?;
        if selected.path != fs::canonicalize(program)? {
            return Err(io::Error::other("runtime executable differs from manifest"));
        }
        Ok(Some(selected))
    }

    pub fn publish(
        &self,
        role: Role,
        version: &str,
        contract: Contract,
        embedded: BTreeMap<String, String>,
        source: Source<'_>,
    ) -> io::Result<Image> {
        self.publish_checked(role, version, contract, embedded, source, |_| Ok(()))
    }

    /// Verification may run the selected program; occupancy protects it while catalog is released.
    pub fn publish_checked(
        &self,
        role: Role,
        version: &str,
        contract: Contract,
        embedded: BTreeMap<String, String>,
        source: Source<'_>,
        check: impl FnOnce(&Image) -> io::Result<()>,
    ) -> io::Result<Image> {
        let image = self.stage_checked(role, version, contract, embedded, source, check)?;
        self.select(&image)?;
        Ok(image)
    }

    /// Keep a verified candidate occupied without changing the selected rollback pair.
    pub fn stage_checked(
        &self,
        role: Role,
        version: &str,
        contract: Contract,
        embedded: BTreeMap<String, String>,
        source: Source<'_>,
        check: impl FnOnce(&Image) -> io::Result<()>,
    ) -> io::Result<Image> {
        let catalog = self.catalog()?;
        let (digest, bytes) = match source {
            Source::File(path) => hash_file(path)?,
            Source::Bytes(bytes) => (blake3::hash(bytes).to_hex().to_string(), bytes.len() as u64),
        };
        let identity = Identity {
            schema: 1,
            role,
            version: version.into(),
            platform: crate::platform(),
            contract,
            digest,
            bytes,
            embedded,
        };
        let manifest = serde_json::to_vec(&identity)?;
        if manifest.len() as u64 > MANIFEST_LIMIT || version.is_empty() {
            return Err(io::Error::other("invalid runtime identity"));
        }
        let id = identity.id()?;
        let destination = self.root.join(&id);
        let index = self.index()?;
        if !destination.exists() {
            // An unsuccessful publication must leave both current and rollback intact.
            self.collect_locked(&index)?;
            let size = tree_bytes(&self.root)?;
            if size.saturating_add(bytes).saturating_add(MANIFEST_LIMIT) > self.limits.cache_bytes
                || local::available(&self.root)?
                    < self
                        .limits
                        .free_bytes
                        .saturating_add(bytes)
                        .saturating_add(MANIFEST_LIMIT)
            {
                return Err(io::Error::new(
                    io::ErrorKind::StorageFull,
                    "runtime cache is full; occupied or unknown versions were retained",
                ));
            }
            let staging = self.root.join(format!(".publish-{}", random()?));
            fs::create_dir(&staging)?;
            let staged = (|| {
                write_new(&staging.join("transaction.json"), &manifest)?;
                let program = staging.join(role.executable());
                let mut output = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&program)?;
                match source {
                    Source::File(path) => {
                        io::copy(&mut File::open(path)?, &mut output)?;
                    }
                    Source::Bytes(bytes) => {
                        output.write_all(bytes)?;
                    }
                }
                output.sync_all()?;
                drop(output);
                let observed = hash_file(&program)?;
                if observed != (identity.digest.clone(), identity.bytes) {
                    return Err(io::Error::other(
                        "runtime source changed during publication",
                    ));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&program, fs::Permissions::from_mode(0o500))?;
                }
                write_new(&staging.join("manifest.json"), &manifest)?;
                write_new(&staging.join("lease"), b"")?;
                fs::remove_file(staging.join("transaction.json"))?;
                sync_dir(&staging)?;
                fs::rename(&staging, &destination)?;
                sync_dir(&self.root)
            })();
            if staged.is_err() {
                let _ = fs::remove_dir_all(&staging);
            }
            staged?;
        }
        let image = self.hold_locked(&id)?;
        drop(catalog);
        check(&image)?;
        Ok(image)
    }

    pub fn select(&self, image: &Image) -> io::Result<()> {
        let _catalog = self.catalog()?;
        let id = image.identity.id()?;
        if image.path != self.root.join(&id).join(image.identity.role.executable()) {
            return Err(io::Error::other(
                "runtime selection belongs to another store",
            ));
        }
        let mut index = self.index()?;
        index.retain(image.role(), &id);
        self.write_index(&index)?;
        self.collect_locked(&index)?;
        Ok(())
    }

    pub fn hold(&self, id: &str) -> io::Result<Image> {
        let _catalog = self.catalog()?;
        self.hold_locked(id)
    }

    pub fn collect(&self) -> io::Result<Collection> {
        let _catalog = self.catalog()?;
        self.collect_locked(&self.index()?)
    }

    pub fn directory(&self) -> &Path {
        &self.root
    }

    pub fn collect_all(&self) -> io::Result<Collection> {
        let mut result = self.collect()?;
        for child in self.cli_stores()? {
            let next = child.collect()?;
            result.removed.extend(next.removed);
            result.occupied.extend(next.occupied);
            result.unknown.extend(next.unknown);
        }
        Ok(result)
    }

    pub fn usage_all(&self) -> io::Result<Usage> {
        let mut result = self.usage()?;
        for child in self.cli_stores()? {
            let next = child.usage()?;
            result.retained_bytes = result.retained_bytes.saturating_add(next.retained_bytes);
            result.active_old_bytes = result
                .active_old_bytes
                .saturating_add(next.active_old_bytes);
            result.idle_bytes = result.idle_bytes.saturating_add(next.idle_bytes);
            result.unknown_items = result.unknown_items.saturating_add(next.unknown_items);
        }
        Ok(result)
    }

    fn usage(&self) -> io::Result<Usage> {
        let _catalog = self.catalog()?;
        let index = self.index()?;
        let protected = index.protected();
        let mut result = Usage::default();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !valid_id(&name) {
                continue;
            }
            let Ok(identity) = identity_at(&entry.path()) else {
                result.unknown_items += 1;
                continue;
            };
            let bytes = fs::metadata(entry.path().join(identity.role.executable()))?.len();
            if bytes != identity.bytes {
                result.unknown_items += 1;
            } else if protected.contains(name.as_str()) {
                result.retained_bytes = result.retained_bytes.saturating_add(bytes);
            } else {
                match Lease::vacant(&entry.path().join("lease")) {
                    Ok(true) => result.idle_bytes = result.idle_bytes.saturating_add(bytes),
                    Ok(false) => {
                        result.active_old_bytes = result.active_old_bytes.saturating_add(bytes)
                    }
                    Err(_) => result.unknown_items += 1,
                }
            }
        }
        Ok(result)
    }

    pub fn contracts(&self) -> io::Result<Vec<Contract>> {
        self.contracts_matching(|_| Ok(true))
    }

    pub fn occupied_contracts(&self) -> io::Result<Vec<Contract>> {
        let mut result =
            self.contracts_matching(|path| Ok(!Lease::vacant(&path.join("lease"))?))?;
        for child in self.cli_stores()? {
            result
                .extend(child.contracts_matching(|path| Ok(!Lease::vacant(&path.join("lease"))?))?);
        }
        Ok(result)
    }

    pub fn cli_stores(&self) -> io::Result<Vec<Self>> {
        let mut result = Vec::new();
        let cli = self.root.join("cli");
        if !cli.try_exists()? {
            return Ok(result);
        }
        local::validate(&cli)?;
        for entry in fs::read_dir(cli)? {
            let entry = entry?;
            if !entry.file_name().to_str().is_some_and(valid_id) {
                continue;
            }
            local::validate(&entry.path())?;
            for platform in [
                "khor-linux-x86_64",
                "khor-linux-aarch64",
                "khor-macos-arm64",
                "khor-macos-x86_64",
            ] {
                let root = entry.path().join(platform).join("versions");
                if !root.join("owner").try_exists()? {
                    continue;
                }
                result.push(Self::open(root, self.limits)?);
            }
        }
        Ok(result)
    }

    fn contracts_matching(
        &self,
        include: impl Fn(&Path) -> io::Result<bool>,
    ) -> io::Result<Vec<Contract>> {
        let _catalog = self.catalog()?;
        let mut result = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let path = entry?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(valid_id)
                && include(&path)?
            {
                result.push(identity_at(&path)?.contract);
            }
        }
        Ok(result)
    }

    fn catalog(&self) -> io::Result<File> {
        let path = self.root.join("catalog");
        match fs::symlink_metadata(&path) {
            Ok(metadata) if !metadata.is_file() => {
                return Err(io::Error::other("runtime catalog is not a regular file"));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.lock()?;
        Ok(file)
    }

    fn index(&self) -> io::Result<Index> {
        match read_bounded(&self.root.join("index.json"), MANIFEST_LIMIT) {
            Ok(bytes) => {
                let index: Index = serde_json::from_slice(&bytes)?;
                if index
                    .roles
                    .values()
                    .any(|ids| ids.len() > 2 || ids.iter().any(|id| !valid_id(id)))
                {
                    return Err(io::Error::other("invalid runtime retention index"));
                }
                Ok(index)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Index::default()),
            Err(error) => Err(error),
        }
    }

    fn write_index(&self, index: &Index) -> io::Result<()> {
        let temporary = self.root.join(format!(".index-{}", random()?));
        write_new(&temporary, &serde_json::to_vec(index)?)?;
        fs::rename(&temporary, self.root.join("index.json"))?;
        sync_dir(&self.root)
    }

    fn hold_locked(&self, id: &str) -> io::Result<Image> {
        if !valid_id(id) {
            return Err(io::Error::other("invalid runtime digest"));
        }
        let directory = self.root.join(id);
        let identity = identity_at(&directory)?;
        let path = directory.join(identity.role.executable());
        if hash_file(&path)? != (identity.digest.clone(), identity.bytes) {
            return Err(io::Error::other(
                "runtime program does not match its manifest",
            ));
        }
        let lease = Lease::take(&directory.join("lease"))?;
        Ok(Image {
            path,
            identity,
            lease,
        })
    }

    fn collect_locked(&self, index: &Index) -> io::Result<Collection> {
        let protected = index.protected();
        let mut result = Collection::default();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".publish-") {
                if cleanup_staging(&entry.path()).is_err() {
                    result.unknown.push(name);
                }
                continue;
            }
            if let Some(id) = name.strip_prefix(".gc-").filter(|id| valid_id(id)) {
                if finish_collection(&entry.path(), id).is_err() {
                    result.unknown.push(name);
                }
                continue;
            }
            if !valid_id(&name) {
                continue;
            }
            let directory = entry.path();
            let Ok(identity) = identity_at(&directory) else {
                result.unknown.push(name);
                continue;
            };
            if protected.contains(name.as_str()) {
                continue;
            }
            match Lease::vacant(&directory.join("lease")) {
                Ok(true) => {
                    if !hash_file(&directory.join(identity.role.executable()))
                        .is_ok_and(|observed| observed == (identity.digest, identity.bytes))
                    {
                        result.unknown.push(name);
                        continue;
                    }
                    let tombstone = self.root.join(format!(".gc-{name}"));
                    let removed = (|| {
                        fs::rename(&directory, &tombstone)?;
                        sync_dir(&self.root)?;
                        finish_collection(&tombstone, &name)
                    })();
                    if removed.is_ok() {
                        result.removed.push(name);
                    } else {
                        result.unknown.push(name);
                    }
                }
                Ok(false) => result.occupied.push(name),
                Err(_) => result.unknown.push(name),
            }
        }
        sync_dir(&self.root)?;
        Ok(result)
    }
}

fn identity_at(directory: &Path) -> io::Result<Identity> {
    let metadata = fs::symlink_metadata(directory)?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(io::Error::other("runtime generation is not a directory"));
    }
    let identity: Identity = serde_json::from_slice(&read_bounded(
        &directory.join("manifest.json"),
        MANIFEST_LIMIT,
    )?)?;
    if identity.schema != 1
        || identity.platform != crate::platform()
        || directory.file_name().and_then(|name| name.to_str()) != Some(identity.id()?.as_str())
    {
        return Err(io::Error::other("runtime manifest identity mismatch"));
    }
    let allowed = [identity.role.executable(), "manifest.json", "lease"];
    let mut files = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !allowed.contains(&name.as_str()) || !entry.file_type()?.is_file() {
            return Err(io::Error::other(
                "runtime generation contains unknown resources",
            ));
        }
        files.insert(name);
    }
    if files.len() != allowed.len() {
        return Err(io::Error::other("runtime generation is incomplete"));
    }
    Ok(identity)
}

fn finish_collection(directory: &Path, expected: &str) -> io::Result<()> {
    if fs::read_dir(directory)?.next().is_none() {
        return fs::remove_dir(directory);
    }
    let identity: Identity = serde_json::from_slice(&read_bounded(
        &directory.join("manifest.json"),
        MANIFEST_LIMIT,
    )?)?;
    if identity.schema != 1 || identity.id()? != expected {
        return Err(io::Error::other("unknown runtime tombstone"));
    }
    check_files(
        directory,
        &[identity.role.executable(), "lease", "manifest.json"],
    )?;
    let program = directory.join(identity.role.executable());
    if program.exists() && hash_file(&program)? != (identity.digest, identity.bytes) {
        return Err(io::Error::other("runtime tombstone program changed"));
    }
    let lease = directory.join("lease");
    if lease.exists() && !Lease::vacant(&lease)? {
        return Err(io::Error::other("runtime tombstone is occupied"));
    }
    // Catalog is held. Manifest is removed last so interrupted collection can resume.
    for name in [identity.role.executable(), "lease", "manifest.json"] {
        match fs::remove_file(directory.join(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    fs::remove_dir(directory)
}

fn cleanup_staging(directory: &Path) -> io::Result<()> {
    if fs::read_dir(directory)?.next().is_none() {
        return fs::remove_dir(directory);
    }
    let transaction = read_bounded(&directory.join("transaction.json"), MANIFEST_LIMIT);
    let identity: Identity = serde_json::from_slice(&match transaction {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            read_bounded(&directory.join("manifest.json"), MANIFEST_LIMIT)?
        }
        Err(error) => return Err(error),
    })?;
    if identity.schema != 1 {
        return Err(io::Error::other("unknown runtime publication"));
    }
    check_files(
        directory,
        &[
            identity.role.executable(),
            "lease",
            "manifest.json",
            "transaction.json",
        ],
    )?;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .is_some_and(|name| name != "transaction.json")
        {
            fs::remove_file(path)?;
        }
    }
    match fs::remove_file(directory.join("transaction.json")) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::remove_dir(directory)
}

fn check_files(directory: &Path, allowed: &[&str]) -> io::Result<()> {
    if !fs::symlink_metadata(directory)?.is_dir() {
        return Err(io::Error::other("runtime transaction is not a directory"));
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !allowed.contains(&entry.file_name().to_string_lossy().as_ref())
        {
            return Err(io::Error::other(
                "runtime transaction contains unknown files",
            ));
        }
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn random() -> io::Result<String> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn read_bounded(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("runtime metadata is not a regular file"));
    }
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::other("runtime metadata exceeds its byte limit"));
    }
    Ok(bytes)
}

fn hash_file(path: &Path) -> io::Result<(String, u64)> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("runtime source is not a regular file"));
    }
    let mut file = File::open(path)?;
    let mut hash = blake3::Hasher::new();
    let mut total = 0u64;
    let mut bytes = [0; 64 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
        total = total.saturating_add(count as u64);
    }
    Ok((hash.finalize().to_hex().to_string(), total))
}

fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(windows)]
    let _ = path;
    Ok(())
}

fn tree_bytes(path: &Path) -> io::Result<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        total = total.saturating_add(if kind.is_dir() {
            tree_bytes(&entry.path())?
        } else {
            entry.metadata()?.len()
        });
    }
    Ok(total)
}
