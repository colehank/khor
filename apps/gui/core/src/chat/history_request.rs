//! Single-use leases for bounded history work. Registration is acknowledged
//! before a client dispatches IO, so leaving during registration can cancel the
//! lease without racing a worker that has not registered yet.
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_REQUESTS: usize = 64;
const ADMISSION_DEADLINE: Duration = Duration::from_secs(30);
struct Pending {
    stop: Arc<AtomicBool>,
    claimed: bool,
    started: Instant,
}
fn requests() -> &'static Mutex<HashMap<String, Pending>> {
    static REQUESTS: OnceLock<Mutex<HashMap<String, Pending>>> = OnceLock::new();
    REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(super) fn begin() -> Result<String, String> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let mut requests = requests()
        .lock()
        .map_err(|_| "history request registry poisoned")?;
    requests.retain(|_, request| request.claimed || request.started.elapsed() < ADMISSION_DEADLINE);
    if requests.len() >= MAX_REQUESTS {
        return Err("too many pending history requests".to_owned());
    }
    let key = format!(
        "{}:{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    requests.insert(
        key.clone(),
        Pending {
            stop: Arc::new(AtomicBool::new(false)),
            claimed: false,
            started: Instant::now(),
        },
    );
    Ok(key)
}

pub(super) fn cancel(key: &str) -> Result<(), String> {
    if key.len() > 128 {
        return Err("invalid history request key".to_owned());
    }
    let mut requests = requests()
        .lock()
        .map_err(|_| "history request registry poisoned")?;
    if let Some(request) = requests.get(key) {
        request.stop.store(true, Ordering::Release);
        if !request.claimed {
            requests.remove(key);
        }
    }
    Ok(())
}

pub(super) struct Request {
    key: String,
    stop: Arc<AtomicBool>,
}
impl Request {
    pub(super) fn claim(key: &str) -> Result<Self, String> {
        if key.len() > 128 {
            return Err("invalid history request key".to_owned());
        }
        let mut requests = requests()
            .lock()
            .map_err(|_| "history request registry poisoned")?;
        let Some(request) = requests.get_mut(key) else {
            return Err("history request cancelled or expired".to_owned());
        };
        if request.claimed {
            return Err("history request already dispatched".to_owned());
        }
        if request.started.elapsed() >= ADMISSION_DEADLINE {
            requests.remove(key);
            return Err("history request admission expired".to_owned());
        }
        request.claimed = true;
        Ok(Self {
            key: key.to_owned(),
            stop: request.stop.clone(),
        })
    }
    pub(super) fn flag(&self) -> &AtomicBool {
        &self.stop
    }
    pub(super) fn check(&self) -> Result<(), String> {
        if self.stop.load(Ordering::Acquire) {
            Err("history request cancelled".to_owned())
        } else {
            Ok(())
        }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        if let Ok(mut requests) = requests().lock() {
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
    #[test]
    fn cancel_before_dispatch_prevents_admission() {
        let key = begin().unwrap();
        cancel(&key).unwrap();
        assert!(Request::claim(&key).is_err());
    }
    #[test]
    fn cancel_reaches_the_claimed_worker_and_drop_removes_only_its_lease() {
        let key = begin().unwrap();
        let other = begin().unwrap();
        let request = Request::claim(&key).unwrap();
        assert!(Request::claim(&key).is_err());
        cancel(&key).unwrap();
        assert!(request.check().is_err());
        assert!(requests().lock().unwrap().get(&key).unwrap().claimed);
        drop(request);
        assert!(!requests().lock().unwrap().contains_key(&key));
        assert!(Request::claim(&other).is_ok());
        assert!(!requests().lock().unwrap().contains_key(&other));
    }
    #[test]
    fn unclaimed_expired_lease_cannot_start_io() {
        let key = begin().unwrap();
        requests().lock().unwrap().get_mut(&key).unwrap().started =
            Instant::now() - ADMISSION_DEADLINE;
        assert!(Request::claim(&key).is_err());
        assert!(!requests().lock().unwrap().contains_key(&key));
    }
}
