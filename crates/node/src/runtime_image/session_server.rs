//! Session workers use a dedicated executable and catalog. Development harnesses can still self-exec.
use super::{HostImage, Role, Source, Store, current, inspect, prepare_entry, report_runtime_info};
use std::{io, path::Path, sync::OnceLock};

static EMBEDDED: OnceLock<&'static [u8]> = OnceLock::new();
static EMBEDDED_IMAGE: OnceLock<String> = OnceLock::new();

/// A standalone CLI carries the sidecar in its single distribution artifact.
pub fn register_embedded(bytes: &'static [u8]) {
    let _ = EMBEDDED.set(bytes);
}

pub(crate) fn enter() -> Result<(), String> {
    prepare_entry();
    report_runtime_info();
    khor_runtime::enter_current().map_err(|error| format!("cannot enter session runtime: {error}"))
}

pub(crate) fn select(
    legacy: impl FnOnce() -> Result<HostImage, String>,
) -> Result<HostImage, String> {
    let running = crate::self_exe()?;
    let installed = crate::installed_exe()?;
    if running
        .file_name()
        .is_some_and(|name| name == Role::SessionHost.executable())
    {
        let image = Store::image_for_program(&running)
            .map_err(|error| error.to_string())?
            .ok_or("running session host has no runtime lease")?;
        return Ok(HostImage {
            image,
            source: installed,
        });
    }

    if let Some(bytes) = EMBEDDED.get() {
        return select_embedded(bytes, installed);
    }

    let name = Role::SessionHost.executable();
    for candidate in [running.with_file_name(name), installed.with_file_name(name)] {
        if candidate.try_exists().map_err(|error| error.to_string())? {
            return select_file(&candidate, installed);
        }
    }

    if khor_update::build_identity()?.release.is_some() {
        return Err("khor-server is missing from this Khor installation".into());
    }
    legacy()
}

fn select_file(path: &Path, installed: std::path::PathBuf) -> Result<HostImage, String> {
    let expected = current()?.release;
    let store = Store::session_hosts().map_err(|error| error.to_string())?;
    let cache = khor_runtime::SourceCache::at(store.directory().join("sources.json"));
    if let Ok(Some(image)) = cache.lookup_with(path, |id| store.hold(id))
        && image.role() == Role::SessionHost
        && expected.contract.accepts(image.contract())
    {
        return Ok(HostImage {
            image,
            source: installed,
        });
    }

    let image = cache.remember_publication(path, || {
        let selected = inspect(path)?.release;
        if selected.role != Role::SessionHost
            || selected.platform != expected.platform
            || !expected.contract.accepts(&selected.contract)
        {
            return Err("khor-server does not match this Khor runtime contract".into());
        }
        store
            .publish_checked(
                Role::SessionHost,
                &selected.version,
                selected.contract.clone(),
                selected.embedded.clone(),
                Source::File(path),
                |image| {
                    if inspect(image.path()).map_err(io::Error::other)?.release != selected {
                        return Err(io::Error::other(
                            "khor-server changed during runtime publication",
                        ));
                    }
                    Ok(())
                },
            )
            .map_err(|error| error.to_string())
    })?;
    Ok(HostImage {
        image,
        source: installed,
    })
}

fn select_embedded(bytes: &[u8], installed: std::path::PathBuf) -> Result<HostImage, String> {
    let expected = current()?.release;
    let store = Store::session_hosts().map_err(|error| error.to_string())?;
    if let Some(id) = EMBEDDED_IMAGE.get()
        && let Ok(image) = store.hold(id)
    {
        return Ok(HostImage {
            image,
            source: installed,
        });
    }
    let image = store
        .publish_checked(
            Role::SessionHost,
            &expected.version,
            expected.contract.clone(),
            Default::default(),
            Source::Bytes(bytes),
            |image| {
                let actual = inspect(image.path()).map_err(io::Error::other)?.release;
                if actual.role != Role::SessionHost
                    || actual.platform != expected.platform
                    || actual.version != expected.version
                    || !expected.contract.accepts(&actual.contract)
                    || !actual.embedded.is_empty()
                {
                    return Err(io::Error::other(
                        "embedded khor-server does not match this Khor runtime",
                    ));
                }
                Ok(())
            },
        )
        .map_err(|error| error.to_string())?;
    if let Ok(id) = image.id() {
        let _ = EMBEDDED_IMAGE.set(id);
    }
    Ok(HostImage {
        image,
        source: installed,
    })
}
