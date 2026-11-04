use khor_update::{Component, DownloadProgress, Offer, Platform, TrustRoot};
use std::{
    io::{Read, Write},
    path::Path,
    time::Duration,
};
use tauri::Runtime;
use tauri_plugin_updater::{Update, UpdaterExt};

pub(crate) struct DesktopPackage {
    update: Update,
    offer: Offer,
    trust: TrustRoot,
}

impl DesktopPackage {
    pub(crate) async fn check<R: Runtime>(
        app: &tauri::AppHandle<R>,
        offer: Offer,
        trust: TrustRoot,
    ) -> Result<Self, String> {
        if offer.artifact().component != Component::DesktopApp
            || offer.artifact().platform != Platform::current()?
        {
            return Err("update offer belongs to another component or platform".into());
        }
        let version = offer.identity().tag.version();
        let expected = version.clone();
        let address = format!(
            "https://github.com/colehank/khor/releases/download/{}/khor-tauri-update.json",
            offer.identity().tag.as_str()
        );
        let endpoint = tauri::Url::parse(&address).map_err(|error| error.to_string())?;
        let updater = app
            .updater_builder()
            .pubkey(trust.tauri_key())
            .target(offer.artifact().platform.updater_target())
            .timeout(Duration::from_secs(30))
            .version_comparator(move |_, release| release.version == expected)
            .endpoints(vec![endpoint])
            .map_err(|error| error.to_string())?
            .build()
            .map_err(|error| error.to_string())?;
        let update = updater
            .check()
            .await
            .map_err(|error| error.to_string())?
            .ok_or("SDK did not return the selected signed update")?;
        let url = tauri::Url::parse(&offer.artifact().url).map_err(|error| error.to_string())?;
        if update.version != version.to_string()
            || update.download_url != url
            || update.signature.trim() != offer.artifact().signature.trim()
        {
            return Err("SDK update metadata disagrees with the authenticated release".into());
        }
        Ok(Self {
            update,
            offer,
            trust,
        })
    }

    pub(crate) fn offer(&self) -> &Offer {
        &self.offer
    }

    pub(crate) async fn download(
        &self,
        path: &Path,
        mut progress: impl FnMut(DownloadProgress),
    ) -> Result<(), String> {
        let parent = path.parent().ok_or("update staging file has no parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        if khor_runtime::available_space(parent).map_err(|error| error.to_string())?
            < self.offer.artifact().required_space()?
        {
            return Err("not enough space to prepare this application update".into());
        }
        let mut received = 0u64;
        let total = self.offer.artifact().bytes;
        let bytes = self
            .update
            .download(
                |size, _| {
                    received = received.saturating_add(size as u64);
                    progress(DownloadProgress { received, total });
                },
                || {},
            )
            .await
            .map_err(|error| error.to_string())?;
        self.trust
            .verify_asset_bytes(&bytes, self.offer.artifact())?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        temporary
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| error.to_string())?;
        drop(bytes);
        temporary.persist(path).map_err(|error| error.to_string())?;
        sync_directory(parent)
    }

    pub(crate) fn install(&self, path: &Path) -> Result<(), String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|error| error.to_string())?
            .take(self.offer.artifact().bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        self.trust
            .verify_asset_bytes(&bytes, self.offer.artifact())?;
        self.update
            .install(bytes)
            .map_err(|error| error.to_string())
    }
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    std::fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    Ok(())
}
