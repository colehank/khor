use super::*;
use std::sync::atomic::Ordering;

impl Node {
    /// New reading words leave every existing file frame unchanged. An
    /// older source still transfers files through the old slice road.
    pub(super) async fn tree_snapshot(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
    ) -> Result<Option<crate::tree::Manifest>, String> {
        let (channel, home) = self.resolve_or_here(machine)?;
        if home == self.device() {
            let path = path.to_owned();
            return tokio::task::spawn_blocking(move || {
                crate::tree::scan(std::path::Path::new(&path))
            })
            .await
            .map_err(|e| e.to_string())?;
        }
        let target = self
            .devices_loaded()?
            .doc
            .by_name(&channel)
            .ok_or_else(|| msg::machine_left_table(&channel))?;
        let answer = if let Some(sponsor) = target.sponsor.clone() {
            if sponsor == self.device_str() {
                let host = crate::satellite::Dest::of(&target)
                    .ok_or_else(|| msg::sat_no_ssh_host(&channel))?;
                let path = path.to_owned();
                return tokio::task::spawn_blocking(move || crate::satellite::tree(&host, &path))
                    .await
                    .map_err(|e| e.to_string())?;
            }
            let via = self
                .devices_loaded()?
                .doc
                .get(&sponsor)
                .ok_or_else(|| msg::machine_left_table(&channel))?;
            self.ask_on(
                ep,
                &via,
                &Request::SatFetchTree {
                    sat: target.id.clone(),
                    path: path.to_owned(),
                },
            )
            .await?
        } else {
            self.ask_on(
                ep,
                &target,
                &Request::FetchTree {
                    path: path.to_owned(),
                },
            )
            .await?
        };
        match answer {
            Response::Tree { manifest } => {
                if let Some(tree) = &manifest {
                    tree.validate()?;
                }
                Ok(manifest)
            }
            Response::Refused { why } if why.contains("FetchTree") => Ok(None),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    pub(super) async fn land_tree(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        path: &str,
        into: &crate::road::Dest,
        manifest: crate::tree::Manifest,
        job: &crate::drops::Job,
    ) -> Result<String, String> {
        manifest.validate()?;
        let dir = match into {
            crate::road::Dest::Receive => self.receive_dir()?,
            crate::road::Dest::Dir(dir) => dir.clone(),
        };
        let at = crate::tree::Landing::new(&dir, &crate::drops::name_of(path)?)?;
        job.total.store(manifest.total, Ordering::Relaxed);
        let mut moved = 0u64;
        for entry in &manifest.entries {
            if job.is_stopped() {
                return Err(crate::drops::cancelled());
            }
            let relative = crate::tree::relative(&entry.path)?;
            let dest = at.part.join(&relative);
            if entry.dir {
                fs::create_dir(&dest).map_err(|e| e.to_string())?;
                continue;
            }
            let name = dest
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| msg::tree_bad_path(&entry.path))?;
            let fell = crate::drops::landing(
                dest.parent()
                    .ok_or_else(|| msg::tree_bad_path(&entry.path))?,
                name,
            )?;
            let mut child = crate::drops::Job::default();
            child.stopped = job.stopped.clone();
            let source = format!("{}/{}", path.trim_end_matches('/'), entry.path);
            let pull = self.pull_into(
                ep,
                machine,
                &source,
                fell,
                Some(&child),
                Some(entry.size),
                false,
            );
            tokio::pin!(pull);
            let out = loop {
                tokio::select! {
                    done = &mut pull => break done,
                    () = tokio::time::sleep(Duration::from_millis(50)) => {
                        job.moved.store(moved + child.moved.load(Ordering::Relaxed), Ordering::Relaxed);
                    }
                }
            }?;
            if out.0 != entry.size {
                return Err(msg::tree_changed().into());
            }
            moved += out.0;
            job.moved.store(moved, Ordering::Relaxed);
        }
        if job.is_stopped() {
            return Err(crate::drops::cancelled());
        }
        if self.tree_snapshot(ep, machine, path).await?.as_ref() != Some(&manifest) {
            return Err(msg::tree_changed().into());
        }
        crate::tree::permissions(&at.part, &manifest)?;
        if job.is_stopped() {
            return Err(crate::drops::cancelled());
        }
        at.publish().map(|p| p.display().to_string())
    }
}
