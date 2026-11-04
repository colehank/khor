use crate::{Offer, TrustRoot};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, Debug)]
pub struct DownloadProgress {
    pub received: u64,
    pub total: u64,
}

pub struct Download {
    directory: tempfile::TempDir,
    archive: PathBuf,
    offer: Offer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};

    fn serve(bytes: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/asset.gz", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 8192];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            let _ = stream.write_all(&bytes);
        });
        (url, worker)
    }

    #[tokio::test]
    async fn download_verifies_signed_original_and_cleans_cancelled_or_invalid_staging() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = include_bytes!("../tests/fixtures/asset.gz");
        let (url, server) = serve(bytes.to_vec());
        let (trust, offer) = crate::trust::fixture_offer(url);
        let mut progress = Vec::new();
        let ready = Download::fetch(offer, &trust, temp.path(), |step| progress.push(step))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(
            std::fs::read(ready.unpack_cli().unwrap()).unwrap(),
            b"Khor update fixture program\n"
        );
        assert_eq!(progress.last().unwrap().received, bytes.len() as u64);
        drop(ready);
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
        for bad in [b"wrong size".to_vec(), vec![b'x'; bytes.len()]] {
            let (url, server) = serve(bad);
            let (trust, offer) = crate::trust::fixture_offer(url);
            assert!(
                Download::fetch(offer, &trust, temp.path(), |_| {})
                    .await
                    .is_err()
            );
            server.join().unwrap();
            assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
        }
    }
}

impl Download {
    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        let (trust, offer) = crate::trust::fixture_offer("http://127.0.0.1/asset.gz".into());
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("asset");
        std::fs::write(&archive, include_bytes!("../tests/fixtures/asset.gz")).unwrap();
        trust
            .verify_file(&archive, &offer.artifact().signature)
            .unwrap();
        Self {
            directory,
            archive,
            offer,
        }
    }

    pub async fn fetch(
        offer: Offer,
        trust: &TrustRoot,
        staging: &Path,
        mut progress: impl FnMut(DownloadProgress),
    ) -> Result<Self, String> {
        std::fs::create_dir_all(staging).map_err(|e| e.to_string())?;
        if khor_runtime::available_space(staging).map_err(|e| e.to_string())?
            < offer.artifact().required_space()?
        {
            return Err("not enough free space to download and install this release".into());
        }
        let directory = tempfile::Builder::new()
            .prefix("download-")
            .tempdir_in(staging)
            .map_err(|e| e.to_string())?;
        let archive = directory.path().join("asset");
        let asset = offer.artifact();
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .user_agent(concat!("khor-update/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(&asset.url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        if response
            .content_length()
            .is_some_and(|size| size != asset.bytes)
        {
            return Err("release size does not match its manifest".into());
        }
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&archive)
            .await
            .map_err(|e| e.to_string())?;
        let mut received = 0u64;
        let mut hash = Sha256::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            received = received
                .checked_add(chunk.len() as u64)
                .ok_or("download size overflow")?;
            if received > asset.bytes {
                return Err("release download exceeds its manifest size".into());
            }
            hash.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            progress(DownloadProgress {
                received,
                total: asset.bytes,
            });
        }
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        if received != asset.bytes || format!("{:x}", hash.finalize()) != asset.sha256.as_str() {
            return Err("release download failed its size or checksum verification".into());
        }
        trust.verify_file(&archive, &asset.signature)?;
        Ok(Self {
            directory,
            archive,
            offer,
        })
    }

    pub fn archive(&self) -> &Path {
        &self.archive
    }
    pub fn offer(&self) -> &Offer {
        &self.offer
    }

    pub fn unpack_cli(&self) -> Result<PathBuf, String> {
        if self.offer.artifact().component != crate::Component::Cli {
            return Err("this asset is not a CLI program".into());
        }
        let program = self
            .directory
            .path()
            .join(if cfg!(windows) { "khor.exe" } else { "khor" });
        let mut decoder = flate2::read::GzDecoder::new(
            std::fs::File::open(&self.archive).map_err(|e| e.to_string())?,
        );
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&program)
            .map_err(|e| e.to_string())?;
        let expected = self.offer.artifact().unpacked_bytes;
        let count = io::copy(
            &mut std::io::Read::by_ref(&mut decoder).take(expected + 1),
            &mut output,
        )
        .map_err(|e| e.to_string())?;
        if count != expected {
            return Err("unpacked program size does not match its manifest".into());
        }
        output.sync_all().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            output
                .set_permissions(std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        Ok(program)
    }
}
