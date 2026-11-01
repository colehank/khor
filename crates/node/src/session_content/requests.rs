//! Admission belongs to the authenticated reader, separately from agent control.
use khor_core::DeviceId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_REQUESTS: usize = 64;
const ADMISSION_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Reader {
    Client(PathBuf),
    Local(PathBuf),
    Peer(DeviceId),
}
impl Reader {
    pub(crate) fn local(root: &Path) -> Self {
        Self::Local(root.to_path_buf())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Scope {
    home: DeviceId,
    reader: Reader,
}
impl Scope {
    pub(crate) fn new(home: DeviceId, reader: Reader) -> Self {
        Self { home, reader }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    scope: Scope,
    token: String,
}
struct Pending {
    source: String,
    stop: Arc<AtomicBool>,
    claimed: bool,
    started: Instant,
}
#[derive(Default)]
struct Registry(Mutex<HashMap<Key, Pending>>);
fn registry() -> &'static Arc<Registry> {
    static REQUESTS: OnceLock<Arc<Registry>> = OnceLock::new();
    REQUESTS.get_or_init(|| Arc::new(Registry::default()))
}
pub(crate) fn validate_token(token: &str) -> Result<(), String> {
    if token.len() != 32 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid content read request key".into());
    }
    Ok(())
}
fn key(scope: &Scope, token: &str) -> Result<Key, String> {
    validate_token(token)?;
    Ok(Key {
        scope: scope.clone(),
        token: token.into(),
    })
}
impl Registry {
    fn begin(&self, scope: &Scope, token: &str, source: &str) -> Result<(), String> {
        let key = key(scope, token)?;
        if source.is_empty() || source.len() > 128 {
            return Err("invalid content read source identity".into());
        }
        let mut requests = self
            .0
            .lock()
            .map_err(|_| "content read registry poisoned")?;
        requests
            .retain(|_, request| request.claimed || request.started.elapsed() < ADMISSION_DEADLINE);
        if let Some(request) = requests.get(&key) {
            if request.source != source {
                return Err("content read request belongs to another source".into());
            }
            if request.claimed {
                return Err("content read request already dispatched".into());
            }
            return Ok(());
        }
        if requests.len() >= MAX_REQUESTS {
            return Err("too many pending content read requests".into());
        }
        requests.insert(
            key,
            Pending {
                source: source.into(),
                stop: Arc::new(AtomicBool::new(false)),
                claimed: false,
                started: Instant::now(),
            },
        );
        Ok(())
    }
    fn cancel(&self, scope: &Scope, token: &str) -> Result<(), String> {
        let key = key(scope, token)?;
        let mut requests = self
            .0
            .lock()
            .map_err(|_| "content read registry poisoned")?;
        if let Some(request) = requests.get(&key) {
            request.stop.store(true, Ordering::Release);
            if !request.claimed {
                requests.remove(&key);
            }
        }
        Ok(())
    }
    fn claim(
        self: &Arc<Self>,
        scope: &Scope,
        token: &str,
        source: &str,
    ) -> Result<Request, String> {
        let key = key(scope, token)?;
        let mut requests = self
            .0
            .lock()
            .map_err(|_| "content read registry poisoned")?;
        let request = requests
            .get_mut(&key)
            .ok_or("content read request cancelled or expired")?;
        if request.source != source {
            return Err("content read request belongs to another source".into());
        }
        if request.claimed {
            return Err("content read request already dispatched".into());
        }
        if request.started.elapsed() >= ADMISSION_DEADLINE {
            requests.remove(&key);
            return Err("content read request admission expired".into());
        }
        request.claimed = true;
        Ok(Request {
            registry: self.clone(),
            key,
            stop: request.stop.clone(),
        })
    }
}

pub(crate) fn begin(scope: &Scope, token: &str, source: &str) -> Result<(), String> {
    registry().begin(scope, token, source)
}
pub(crate) fn cancel(scope: &Scope, token: &str) -> Result<(), String> {
    registry().cancel(scope, token)
}
pub(crate) fn claim(scope: &Scope, token: &str, source: &str) -> Result<Request, String> {
    registry().claim(scope, token, source)
}

pub(crate) struct Request {
    registry: Arc<Registry>,
    key: Key,
    stop: Arc<AtomicBool>,
}
impl Request {
    pub(crate) fn flag(&self) -> &AtomicBool {
        &self.stop
    }
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) {
            Err("content read request cancelled".into())
        } else {
            Ok(())
        }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.registry.0.lock() {
            if requests
                .get(&self.key)
                .is_some_and(|request| Arc::ptr_eq(&request.stop, &self.stop))
            {
                requests.remove(&self.key);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope(peer: u8) -> Scope {
        Scope::new(DeviceId([1; 32]), Reader::Peer(DeviceId([peer; 32])))
    }
    fn token(n: u64) -> String {
        format!("{n:032x}")
    }
    #[test]
    fn acknowledged_cancel_before_dispatch_prevents_io() {
        let registry = Arc::new(Registry::default());
        registry.begin(&scope(2), &token(1), "source").unwrap();
        registry.cancel(&scope(2), &token(1)).unwrap();
        assert!(registry.claim(&scope(2), &token(1), "source").is_err());
    }
    #[test]
    fn authenticated_reader_and_source_scope_cannot_be_borrowed() {
        let registry = Arc::new(Registry::default());
        registry.begin(&scope(2), &token(1), "source").unwrap();
        registry.cancel(&scope(3), &token(1)).unwrap();
        assert!(registry.claim(&scope(3), &token(1), "source").is_err());
        assert!(registry.claim(&scope(2), &token(1), "other").is_err());
        let request = registry.claim(&scope(2), &token(1), "source").unwrap();
        assert!(request.check().is_ok());
        assert!(registry.claim(&scope(2), &token(1), "source").is_err());
    }
    #[test]
    fn claimed_cancel_keeps_capacity_until_the_actual_worker_drops() {
        let registry = Arc::new(Registry::default());
        for n in 0..MAX_REQUESTS as u64 {
            registry.begin(&scope(2), &token(n), "source").unwrap();
        }
        let request = registry.claim(&scope(2), &token(0), "source").unwrap();
        registry.cancel(&scope(2), &token(0)).unwrap();
        assert!(request.check().is_err());
        assert!(registry.begin(&scope(2), &token(100), "source").is_err());
        drop(request);
        assert!(registry.begin(&scope(2), &token(100), "source").is_ok());
    }
    #[test]
    fn begin_does_not_extend_pending_ttl_and_expired_admission_is_rejected() {
        let registry = Arc::new(Registry::default());
        registry.begin(&scope(2), &token(1), "source").unwrap();
        let key = key(&scope(2), &token(1)).unwrap();
        let started = registry.0.lock().unwrap().get(&key).unwrap().started;
        registry.begin(&scope(2), &token(1), "source").unwrap();
        assert_eq!(
            registry.0.lock().unwrap().get(&key).unwrap().started,
            started
        );
        registry.0.lock().unwrap().get_mut(&key).unwrap().started =
            Instant::now() - ADMISSION_DEADLINE;
        assert!(registry.claim(&scope(2), &token(1), "source").is_err());
        assert!(!registry.0.lock().unwrap().contains_key(&key));
    }
}
