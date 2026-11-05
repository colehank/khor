use crate::{runtime_image, updates};
use khor_update::{CliInstallation, Component, Download, Platform, ReleaseTag, TrustRoot};
use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) fn if_requested() -> Option<Result<(), String>> {
    if std::env::args_os().nth(1).as_deref() != Some(OsStr::new("_install_cli")) {
        return None;
    }
    Some(install(
        &std::env::args_os()
            .skip(2)
            .map(PathBuf::from)
            .collect::<Vec<_>>(),
    ))
}

fn install(arguments: &[PathBuf]) -> Result<(), String> {
    let [bin, tag, manifest, signature, archive] = arguments else {
        return Err(
            "CLI installation requires destination, release tag, signed manifest and archive"
                .into(),
        );
    };
    let tag: ReleaseTag = tag
        .to_str()
        .ok_or("invalid release tag")?
        .to_owned()
        .try_into()?;
    let trust = TrustRoot::bundled()?;
    let manifest = read(manifest, 256 * 1024)?;
    let signature = String::from_utf8(read(signature, 4096)?).map_err(|e| e.to_string())?;
    let release = trust.authenticate(&manifest, &signature)?;
    let platform = Platform::current()?;
    let channel = tag.channel();
    let offer = release
        .select(
            Component::Cli,
            platform,
            &release.manifest().identity,
            channel,
            Some(&tag),
        )?
        .ok_or("release has no CLI for this machine")?;
    updates::verify_program(&crate::self_exe()?, &offer)?;
    let runtime = khor_runtime::local_directory().map_err(|e| e.to_string())?;
    let mut installation = CliInstallation::prepare(bin, platform, &runtime)?;
    let download = Download::import(offer, &trust, archive, &installation.staging())?;
    if installation.current()?.is_none() {
        if let Some(previous) = installation.bootstrap_source()? {
            installation = installation.on_node(&runtime, || {
                let image = runtime_image::host_at(&previous)?;
                khor_runtime::Store::image_for_program(image.path())
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "existing CLI could not be retained locally".into())
            })?;
        }
    }
    updates::install_cli(&installation, &download, channel, None)?;
    installation.publish_launcher()?;
    println!(
        "{}",
        serde_json::to_string(&installation.receipt()?).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("installation metadata exceeds its limit".into());
    }
    Ok(bytes)
}
