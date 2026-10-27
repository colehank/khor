//! Process-local admission for a resident's final agent shutdown.
//! Only an explicit new resident can advance a retired generation. Final
//! process exit additionally seals the Root against any new generation.
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    thread::JoinHandle,
};

const ROOT_LIMIT: usize = 256;
const CALL_LIMIT: usize = 256;
#[derive(Default)]
struct Root {
    generation: u64,
    terminal: bool,
    sealed: bool,
    retired: bool,
    draining: bool,
    serving: bool,
    calls: usize,
    producers: Vec<JoinHandle<()>>,
    unconfirmed: bool,
}
fn roots() -> &'static Mutex<HashMap<PathBuf, Root>> {
    static ROOTS: OnceLock<Mutex<HashMap<PathBuf, Root>>> = OnceLock::new();
    ROOTS.get_or_init(Default::default)
}
fn reserve<'a>(roots: &'a mut HashMap<PathBuf, Root>, root: &Path) -> Result<&'a mut Root, String> {
    if !roots.contains_key(root) && roots.len() >= ROOT_LIMIT {
        return Err("ResourceLimit".into());
    }
    Ok(roots.entry(root.into()).or_default())
}
/// Includes the complete API future: a handoff timeout or canceled GUI call
/// drops its admission, while its separately registered executor stays owned.
pub(crate) struct Call(PathBuf);
pub(crate) fn enter(root: &Path, generation: &OnceLock<u64>) -> Result<Call, String> {
    reap_finished(root);
    let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
    let state = reserve(&mut roots, root)?;
    if state.sealed || *generation.get_or_init(|| state.generation) != state.generation {
        return Err("RuntimeShutDown".into());
    }
    if state.calls + state.producers.len() >= CALL_LIMIT {
        return Err("ResourceLimit".into());
    }
    state.calls += 1;
    Ok(Call(root.into()))
}
impl Drop for Call {
    fn drop(&mut self) {
        let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
        let state = roots.get_mut(&self.0).expect("admitted agent call");
        state.calls -= 1;
    }
}
pub(crate) fn register_producer(root: &Path, producer: JoinHandle<()>) {
    reap_finished(root);
    // The starting API still holds its Call. Shutdown cannot observe zero calls until
    // this handle is installed, including a thread that has already finished.
    roots()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get_mut(root)
        .expect("producer admission")
        .producers
        .push(producer);
}
pub(super) fn reap_finished(root: &Path) {
    let finished = {
        let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
        let Some(state) = roots.get_mut(root) else {
            return;
        };
        let mut finished = Vec::new();
        let mut index = 0;
        while index < state.producers.len() {
            if state.producers[index].is_finished() {
                finished.push(state.producers.swap_remove(index));
            } else {
                index += 1;
            }
        }
        // Keep the reservation until joins finish, even when the last API
        // future went away. No root lock is held across a join.
        state.calls += 1;
        finished
    };
    for producer in finished {
        if producer.join().is_err() {
            unconfirmed(root);
        }
    }
    drop(Call(root.into()));
}
pub(super) fn idle(root: &Path) -> bool {
    reap_finished(root);
    roots()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(root)
        .is_none_or(|state| state.calls == 0 && state.producers.is_empty() && !state.unconfirmed)
}
/// Retained after Busy, errors, and dropped async waiters. Final process exit
/// also prohibits an explicit new resident from advancing the generation.
pub(crate) fn seal(root: &Path, terminal: bool) -> Result<(), String> {
    let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
    let state = reserve(&mut roots, root)?;
    state.retired = false;
    state.sealed = true;
    state.terminal |= terminal;
    Ok(())
}
/// A process/reader owner ended without a trustworthy completion receipt.
/// Retain its Root: a joined Rust thread alone cannot assert native retirement.
pub(crate) fn unconfirmed(root: &Path) {
    roots()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get_mut(root)
        .expect("producer reservation")
        .unconfirmed = true;
}
pub(super) fn retired(root: &Path) {
    roots()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get_mut(root)
        .expect("shutdown reservation")
        .retired = true;
}
pub(super) struct Drain(PathBuf);
pub(super) fn drain(root: &Path, terminal: bool) -> Result<Option<Drain>, String> {
    let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
    let state = reserve(&mut roots, root)?;
    state.sealed = true;
    state.terminal |= terminal;
    if state.draining {
        return Ok(None);
    }
    state.retired = false;
    state.draining = true;
    Ok(Some(Drain(root.into())))
}
impl Drop for Drain {
    fn drop(&mut self) {
        roots()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_mut(&self.0)
            .expect("shutdown reservation")
            .draining = false;
    }
}
/// Startup errors release only this reservation. Once listening, a canceled
/// serve future fails closed; its owner must still await normal shutdown.
pub(crate) struct Serve {
    root: PathBuf,
    listening: bool,
    generation: u64,
}
pub(crate) fn serve(root: &Path, generation: &OnceLock<u64>) -> Result<Serve, String> {
    reap_finished(root);
    let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
    let state = reserve(&mut roots, root)?;
    if state.terminal {
        return Err("RuntimeShutDown".into());
    }
    if state.serving
        || state.draining
        || state.calls != 0
        || !state.producers.is_empty()
        || state.unconfirmed
        || (state.sealed && !state.retired)
    {
        return Err("AgentResidentBusy".into());
    }
    let next = if state.sealed {
        state.generation + 1
    } else {
        state.generation
    };
    if *generation.get_or_init(|| next) != next {
        return Err("AgentResidentStaleOwner".into());
    }
    state.generation = next;
    state.sealed = false;
    state.serving = true;
    Ok(Serve {
        root: root.into(),
        listening: false,
        generation: state.generation,
    })
}
impl Serve {
    pub(crate) fn listening(&mut self) -> Result<(), String> {
        if roots()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(&self.root)
            .expect("resident reservation")
            .sealed
        {
            return Err("RuntimeShutDown".into());
        }
        self.listening = true;
        Ok(())
    }
}
impl Drop for Serve {
    fn drop(&mut self) {
        {
            let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
            let state = roots.get_mut(&self.root).expect("resident reservation");
            if self.listening && !state.sealed {
                state.retired = false;
            }
            state.sealed |= self.listening;
        }
        if self.listening {
            // Keep this generation's serving reservation through the last
            // root-scoped stop, including after its drain marked it retired.
            // A new resident cannot advance and be stopped by this old owner.
            #[cfg(all(test, unix))]
            super::runtime_close_tests::producer_point(&self.root, "serve-final-stop");
            super::stop_local_producers(&self.root);
        }
        let mut roots = roots().lock().unwrap_or_else(|p| p.into_inner());
        let state = roots.get_mut(&self.root).expect("resident reservation");
        if state.generation == self.generation {
            state.serving = false;
        }
    }
}
