use khor_gui_core::updates::UpdateBlocker;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};

struct Request {
    nonce: String,
    replies: BTreeMap<String, Option<Vec<UpdateBlocker>>>,
}

#[derive(Default)]
pub(crate) struct UpdateAdmission {
    request: Mutex<Option<Request>>,
    changed: tokio::sync::Notify,
}

pub(crate) struct RestartPermit {
    app: AppHandle,
    admission: Arc<UpdateAdmission>,
    nonce: String,
}

impl Drop for RestartPermit {
    fn drop(&mut self) {
        self.admission.release(&self.app, &self.nonce);
    }
}

impl UpdateAdmission {
    pub(crate) async fn prepare(
        self: &Arc<Self>,
        app: &AppHandle,
    ) -> Result<RestartPermit, Vec<UpdateBlocker>> {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1);
        let nonce = format!(
            "{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let labels = content_windows(app);
        if labels.is_empty() {
            return Err(vec![UpdateBlocker::WindowNotReady]);
        }
        {
            let mut request = self
                .request
                .lock()
                .map_err(|_| vec![UpdateBlocker::WindowNotReady])?;
            if request.is_some() {
                return Err(vec![UpdateBlocker::WindowNotReady]);
            }
            *request = Some(Request {
                nonce: nonce.clone(),
                replies: labels.iter().map(|label| (label.clone(), None)).collect(),
            });
        }
        let permit = RestartPermit {
            app: app.clone(),
            admission: self.clone(),
            nonce: nonce.clone(),
        };
        for label in &labels {
            let _ = app.emit_to(label, "app-update:prepare", &nonce);
        }
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.changed.notified();
                if let Some(result) = self.result(&nonce) {
                    break result;
                }
                changed.await;
            }
        })
        .await
        .unwrap_or_else(|_| Err(vec![UpdateBlocker::WindowNotReady]));
        result?;
        if content_windows(app) != labels {
            return Err(vec![UpdateBlocker::WindowNotReady]);
        }
        Ok(permit)
    }

    fn result(&self, nonce: &str) -> Option<Result<(), Vec<UpdateBlocker>>> {
        let request = self.request.lock().ok()?;
        let request = request.as_ref().filter(|request| request.nonce == nonce)?;
        if request.replies.values().any(Option::is_none) {
            return None;
        }
        let mut blocked = Vec::new();
        for reason in request.replies.values().flatten().flatten() {
            if !blocked.contains(reason) {
                blocked.push(*reason)
            }
        }
        Some(if blocked.is_empty() {
            Ok(())
        } else {
            Err(blocked)
        })
    }

    pub(crate) fn reply(
        &self,
        label: &str,
        nonce: &str,
        blocked: Vec<UpdateBlocker>,
    ) -> Result<(), String> {
        let mut request = self
            .request
            .lock()
            .map_err(|_| "update admission is unavailable")?;
        let request = request
            .as_mut()
            .filter(|request| request.nonce == nonce)
            .ok_or("update preparation has expired")?;
        let reply = request
            .replies
            .get_mut(label)
            .ok_or("this window does not own an update preparation")?;
        if reply.is_some() {
            return Err("this window already answered the preparation".into());
        }
        *reply = Some(blocked);
        self.changed.notify_one();
        Ok(())
    }

    fn release(&self, app: &AppHandle, nonce: &str) {
        if let Ok(mut request) = self.request.lock() {
            if !request
                .as_ref()
                .is_some_and(|request| request.nonce == nonce)
            {
                return;
            }
            *request = None;
        }
        let _ = app.emit("app-update:resume", nonce);
    }
}

fn content_windows(app: &AppHandle) -> BTreeSet<String> {
    app.webview_windows()
        .keys()
        .filter(|label| {
            label.as_str() == "main" || label.as_str() == "settings" || label.starts_with("code:")
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_original_window_must_checkpoint_and_late_or_duplicate_answers_cannot_replace_it() {
        let admission = UpdateAdmission::default();
        *admission.request.lock().unwrap() = Some(Request {
            nonce: "new".into(),
            replies: [("main".into(), None), ("code:one".into(), None)].into(),
        });
        assert!(admission.reply("main", "old", vec![]).is_err());
        assert!(admission.reply("page:external", "new", vec![]).is_err());
        admission.reply("main", "new", vec![]).unwrap();
        assert!(
            admission.result("new").is_none(),
            "main cannot declare another editor safe"
        );
        admission
            .reply("code:one", "new", vec![UpdateBlocker::UnsavedFiles])
            .unwrap();
        assert_eq!(
            admission.result("new"),
            Some(Err(vec![UpdateBlocker::UnsavedFiles]))
        );
        assert!(admission.reply("code:one", "new", vec![]).is_err());
        assert_eq!(
            admission.result("new"),
            Some(Err(vec![UpdateBlocker::UnsavedFiles]))
        );
    }
}
