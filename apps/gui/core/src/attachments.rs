//! One view lease owns one exact attachment, independently of its session id.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use ts_rs::TS;

#[derive(Clone, Copy, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Chat,
    Terminal,
}

pub fn attachment_begin(root: &Path, id: &str, kind: AttachmentKind) -> Result<String, String> {
    khor_node::session_content::refuse_control(&khor_core::SessionId(id.to_owned()))?;
    crate::chat::expire_views();
    crate::term::expire_views();
    match kind {
        AttachmentKind::Chat => crate::chat::begin_view(root, id),
        AttachmentKind::Terminal => crate::term::begin_view(root, id),
    }
}

const PENDING_LIMIT: usize = 64;
const PENDING_LIFETIME: Duration = Duration::from_secs(30);
const CLOSED: &str = "attachment is closed";

fn opening_slots() -> Arc<Semaphore> {
    static SLOTS: std::sync::OnceLock<Arc<Semaphore>> = std::sync::OnceLock::new();
    SLOTS
        .get_or_init(|| Arc::new(Semaphore::new(PENDING_LIMIT)))
        .clone()
}

pub(crate) trait Resource: Send + Sync {
    fn is_live(&self) -> bool;
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Owner {
    root: PathBuf,
    id: String,
}
impl Owner {
    fn new(root: &Path, id: &str) -> Self {
        Self {
            root: root.to_path_buf(),
            id: id.to_owned(),
        }
    }
}

struct InFlight {
    cancelled: AtomicBool,
    _slot: OwnedSemaphorePermit,
}
enum Stage {
    Pending {
        since: Instant,
        slot: OwnedSemaphorePermit,
    },
    Opening(Arc<InFlight>),
    Attached {
        resource: String,
        opening: Weak<InFlight>,
    },
}
struct View {
    owner: Owner,
    stage: Stage,
}
struct State<T> {
    views: HashMap<String, View>,
    current: HashMap<Owner, String>,
    resources: HashMap<String, Arc<T>>,
}
impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            views: HashMap::new(),
            current: HashMap::new(),
            resources: HashMap::new(),
        }
    }
}

pub(crate) struct Registry<T: Resource> {
    state: Mutex<State<T>>,
    slots: Arc<Semaphore>,
}
impl<T: Resource> Default for Registry<T> {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            slots: opening_slots(),
        }
    }
}
impl<T: Resource> Registry<T> {
    fn lock(&self) -> std::sync::MutexGuard<'_, State<T>> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(crate) fn expire(&self) {
        let now = Instant::now();
        self.lock().views.retain(|_, view| !matches!(&view.stage, Stage::Pending { since, .. } if now.saturating_duration_since(*since)>=PENDING_LIFETIME));
    }
    pub(crate) fn begin(&self, root: &Path, id: &str) -> Result<String, String> {
        if id.is_empty() || id.len() > 4096 {
            return Err("invalid attachment session id".into());
        }
        self.expire();
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| "too many attachments are opening".to_owned())?;
        let key = khor_node::link::fresh_hex()?;
        let mut state = self.lock();
        if state.views.contains_key(&key) {
            return Err("duplicate attachment lease".into());
        }
        state.views.insert(
            key.clone(),
            View {
                owner: Owner::new(root, id),
                stage: Stage::Pending {
                    since: Instant::now(),
                    slot,
                },
            },
        );
        Ok(key)
    }
    pub(crate) fn claim<'a>(
        &'a self,
        root: &Path,
        id: &str,
        key: &str,
    ) -> Result<Opening<'a, T>, String> {
        self.expire();
        let owner = Owner::new(root, id);
        let mut state = self.lock();
        if state.views.get(key).is_some_and(|view| view.owner != owner) {
            return Err("attachment lease belongs to another view".into());
        }
        let view = state.views.remove(key).ok_or_else(|| CLOSED.to_owned())?;
        match view.stage {
            Stage::Pending { slot, .. } => {
                let operation = Arc::new(InFlight {
                    cancelled: AtomicBool::new(false),
                    _slot: slot,
                });
                state.views.insert(
                    key.to_owned(),
                    View {
                        owner: owner.clone(),
                        stage: Stage::Opening(operation.clone()),
                    },
                );
                Ok(Opening {
                    registry: self,
                    key: key.to_owned(),
                    owner,
                    operation,
                })
            }
            stage => {
                state.views.insert(key.to_owned(), View { owner, stage });
                Err("attachment lease was already claimed".into())
            }
        }
    }
    pub(crate) fn read(&self, root: &Path, id: &str, key: &str) -> Result<Arc<T>, String> {
        self.resource(root, id, key, false)
    }
    pub(crate) fn control(&self, root: &Path, id: &str, key: &str) -> Result<Arc<T>, String> {
        self.resource(root, id, key, true)
    }
    fn resource(
        &self,
        root: &Path,
        id: &str,
        key: &str,
        controlling: bool,
    ) -> Result<Arc<T>, String> {
        let state = self.lock();
        let view = state.views.get(key).ok_or_else(|| CLOSED.to_owned())?;
        if view.owner != Owner::new(root, id) {
            return Err("attachment lease belongs to another view".into());
        }
        let Stage::Attached { resource, .. } = &view.stage else {
            return Err("attachment is not open".into());
        };
        let value = state
            .resources
            .get(resource)
            .ok_or_else(|| CLOSED.to_owned())?;
        if controlling && (state.current.get(&view.owner) != Some(resource) || !value.is_live()) {
            return Err(CLOSED.into());
        }
        Ok(value.clone())
    }
    /// The caller closes a returned last-holder resource after this lock is released.
    pub(crate) fn leave(&self, root: &Path, id: &str, key: &str) -> Result<Option<Arc<T>>, String> {
        let owner = Owner::new(root, id);
        let mut state = self.lock();
        if state.views.get(key).is_some_and(|view| view.owner != owner) {
            return Err("attachment lease belongs to another view".into());
        }
        let Some(view) = state.views.remove(key) else {
            return Ok(None);
        };
        match view.stage {
            Stage::Pending { .. } => Ok(None),
            Stage::Opening(operation) => {
                operation.cancelled.store(true, Ordering::Release);
                state.views.insert(
                    key.to_owned(),
                    View {
                        owner,
                        stage: Stage::Opening(operation),
                    },
                );
                Ok(None)
            }
            Stage::Attached { resource, opening } => {
                if let Some(operation) = opening.upgrade() {
                    operation.cancelled.store(true, Ordering::Release);
                }
                let held=state.views.values().any(|view|matches!(&view.stage,Stage::Attached {resource:other,..} if *other==resource));
                if held {
                    return Ok(None);
                }
                if state.current.get(&owner) == Some(&resource) {
                    state.current.remove(&owner);
                }
                Ok(state.resources.remove(&resource))
            }
        }
    }
}

pub(crate) struct Opening<'a, T: Resource> {
    registry: &'a Registry<T>,
    key: String,
    owner: Owner,
    operation: Arc<InFlight>,
}
impl<T: Resource> Opening<'_, T> {
    pub(crate) fn check(&self) -> Result<(), String> {
        if self.operation.cancelled.load(Ordering::Acquire) {
            Err(CLOSED.into())
        } else {
            Ok(())
        }
    }
    fn admitted(&self, state: &State<T>) -> Result<(), String> {
        self.check()?;
        if !state.views.get(&self.key).is_some_and(|view|view.owner==self.owner && matches!(&view.stage,Stage::Opening(operation) if Arc::ptr_eq(operation,&self.operation))) { return Err(CLOSED.into()); }
        Ok(())
    }
    fn current(state: &mut State<T>, owner: &Owner) -> Option<(String, Arc<T>)> {
        let key = state.current.get(owner)?.clone();
        if let Some(value) = state.resources.get(&key).filter(|value| value.is_live()) {
            return Some((key, value.clone()));
        }
        state.current.remove(owner);
        None
    }
    fn bind(&self, state: &mut State<T>, resource: String) {
        state.views.insert(
            self.key.clone(),
            View {
                owner: self.owner.clone(),
                stage: Stage::Attached {
                    resource,
                    opening: Arc::downgrade(&self.operation),
                },
            },
        );
    }
    pub(crate) fn share(&self) -> Result<Option<Arc<T>>, String> {
        let mut state = self.registry.lock();
        self.admitted(&state)?;
        let Some((key, value)) = Self::current(&mut state, &self.owner) else {
            return Ok(None);
        };
        self.bind(&mut state, key);
        Ok(Some(value))
    }
    pub(crate) fn publish(&self, candidate: Arc<T>) -> Result<(Arc<T>, bool), String> {
        let key = khor_node::link::fresh_hex()?;
        let mut state = self.registry.lock();
        self.admitted(&state)?;
        if let Some((key, value)) = Self::current(&mut state, &self.owner) {
            self.bind(&mut state, key);
            return Ok((value, false));
        }
        if state.resources.contains_key(&key) {
            return Err("duplicate attachment resource".into());
        }
        state.resources.insert(key.clone(), candidate.clone());
        state.current.insert(self.owner.clone(), key.clone());
        self.bind(&mut state, key);
        Ok((candidate, true))
    }
}
impl<T: Resource> Drop for Opening<'_, T> {
    fn drop(&mut self) {
        let mut state = self.registry.lock();
        if state.views.get(&self.key).is_some_and(|view|matches!(&view.stage,Stage::Opening(operation) if Arc::ptr_eq(operation,&self.operation))) { state.views.remove(&self.key); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Handle(AtomicBool);
    impl Resource for Handle {
        fn is_live(&self) -> bool {
            self.0.load(Ordering::Acquire)
        }
    }
    fn book(slots: Arc<Semaphore>) -> Registry<Handle> {
        Registry {
            state: Mutex::new(State::default()),
            slots,
        }
    }
    fn live() -> Arc<Handle> {
        Arc::new(Handle(AtomicBool::new(true)))
    }
    fn key(book: &Registry<Handle>) -> String {
        book.begin(Path::new("/owned"), "session").unwrap()
    }
    fn claim<'a>(book: &'a Registry<Handle>, key: &str) -> Opening<'a, Handle> {
        book.claim(Path::new("/owned"), "session", key).unwrap()
    }
    #[test]
    fn view_lease_is_scoped_and_single_use() {
        let book = book(Arc::new(Semaphore::new(2)));
        let key = key(&book);
        assert!(book.claim(Path::new("/other"), "session", &key).is_err());
        assert!(book.claim(Path::new("/owned"), "different", &key).is_err());
        assert!(book.leave(Path::new("/other"), "session", &key).is_err());
        let _attempt = claim(&book, &key);
        assert!(book.claim(Path::new("/owned"), "session", &key).is_err());
    }
    #[test]
    fn leave_before_worker_claim_prevents_any_connection() {
        let slots = Arc::new(Semaphore::new(1));
        let book = book(slots.clone());
        let key = key(&book);
        assert!(
            book.leave(Path::new("/owned"), "session", &key)
                .unwrap()
                .is_none()
        );
        assert!(book.claim(Path::new("/owned"), "session", &key).is_err());
        assert_eq!(slots.available_permits(), 1);
    }
    #[test]
    fn cancelled_claim_holds_global_opening_capacity_until_worker_drop() {
        let slots = Arc::new(Semaphore::new(1));
        let first = book(slots.clone());
        let second = book(slots.clone());
        let key = key(&first);
        let attempt = claim(&first, &key);
        first.leave(Path::new("/owned"), "session", &key).unwrap();
        assert!(attempt.check().is_err());
        assert!(second.begin(Path::new("/owned"), "another").is_err());
        drop(attempt);
        assert!(second.begin(Path::new("/owned"), "another").is_ok());
    }
    #[test]
    fn abandoned_worker_releases_its_pending_owner() {
        let slots = Arc::new(Semaphore::new(1));
        let book = book(slots.clone());
        let key = key(&book);
        drop(claim(&book, &key));
        assert!(book.read(Path::new("/owned"), "session", &key).is_err());
        assert_eq!(slots.available_permits(), 1);
    }
    #[test]
    fn only_unclaimed_pending_views_expire() {
        let slots = Arc::new(Semaphore::new(2));
        let book = book(slots.clone());
        let pending = key(&book);
        let active = key(&book);
        let attempt = claim(&book, &active);
        let value = live();
        attempt.publish(value.clone()).unwrap();
        drop(attempt);
        {
            let mut state = book.lock();
            let Stage::Pending { since, .. } = &mut state.views.get_mut(&pending).unwrap().stage
            else {
                panic!("pending")
            };
            *since = Instant::now() - PENDING_LIFETIME - Duration::from_secs(1);
        }
        book.expire();
        assert!(
            book.claim(Path::new("/owned"), "session", &pending)
                .is_err()
        );
        assert!(Arc::ptr_eq(
            &book.read(Path::new("/owned"), "session", &active).unwrap(),
            &value
        ));
        assert_eq!(slots.available_permits(), 2);
    }
    #[test]
    fn simultaneous_views_share_one_resource_and_release_independently() {
        let book = book(Arc::new(Semaphore::new(2)));
        let first = key(&book);
        let a = claim(&book, &first);
        let resource = live();
        a.publish(resource.clone()).unwrap();
        drop(a);
        let second = key(&book);
        let b = claim(&book, &second);
        assert!(Arc::ptr_eq(&b.share().unwrap().unwrap(), &resource));
        drop(b);
        assert!(
            book.leave(Path::new("/owned"), "session", &first)
                .unwrap()
                .is_none()
        );
        assert!(Arc::ptr_eq(
            &book
                .control(Path::new("/owned"), "session", &second)
                .unwrap(),
            &resource
        ));
        assert!(Arc::ptr_eq(
            &book
                .leave(Path::new("/owned"), "session", &second)
                .unwrap()
                .unwrap(),
            &resource
        ));
    }
    #[test]
    fn late_old_view_cannot_control_or_release_the_new_resource() {
        let book = book(Arc::new(Semaphore::new(2)));
        let old_key = key(&book);
        let a = claim(&book, &old_key);
        let old = live();
        a.publish(old.clone()).unwrap();
        drop(a);
        old.0.store(false, Ordering::Release);
        let new_key = key(&book);
        let b = claim(&book, &new_key);
        assert!(b.share().unwrap().is_none());
        let new = live();
        assert!(b.publish(new.clone()).unwrap().1);
        drop(b);
        assert!(Arc::ptr_eq(
            &book.read(Path::new("/owned"), "session", &old_key).unwrap(),
            &old
        ));
        assert!(
            book.control(Path::new("/owned"), "session", &old_key)
                .is_err()
        );
        assert!(Arc::ptr_eq(
            &book
                .leave(Path::new("/owned"), "session", &old_key)
                .unwrap()
                .unwrap(),
            &old
        ));
        assert!(Arc::ptr_eq(
            &book
                .control(Path::new("/owned"), "session", &new_key)
                .unwrap(),
            &new
        ));
    }
    #[test]
    fn leave_after_publish_still_cancels_the_actual_open_worker() {
        let slots = Arc::new(Semaphore::new(1));
        let book = book(slots.clone());
        let key = key(&book);
        let attempt = claim(&book, &key);
        attempt.publish(live()).unwrap();
        book.leave(Path::new("/owned"), "session", &key).unwrap();
        assert!(attempt.check().is_err());
        assert_eq!(slots.available_permits(), 0);
        drop(attempt);
        assert_eq!(slots.available_permits(), 1);
    }
    #[test]
    fn default_registries_share_one_global_admission_budget() {
        let first = Registry::<Handle>::default();
        let second = Registry::<Handle>::default();
        assert!(Arc::ptr_eq(&first.slots, &second.slots));
    }
}
