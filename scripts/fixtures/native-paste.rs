// The driver inserts the current production functions and scheduler here.
// Only the screen oracle and write sink are adapters; the lock gate makes the
// cancellation window deterministic without sleeps or a simulated paste body.
// COMMAND_QUEUE_MODULE

use std::{
    any::TypeId,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

mod khor_node {
    pub mod drops {
        pub fn cancelled() -> String {
            "cancelled".into()
        }
    }
}

struct Parser;
impl Parser {
    fn screen(&self) -> &Self {
        self
    }
    fn bracketed_paste(&self) -> bool {
        false
    }
}

struct Term {
    commands: command_queue::CommandQueue,
    left: AtomicBool,
    parser: Mutex<Parser>,
    conn: Mutex<Vec<u8>>,
}

struct Gate {
    kind: TypeId,
    reached: mpsc::Sender<()>,
}
fn gate() -> &'static Mutex<Option<Gate>> {
    static GATE: OnceLock<Mutex<Option<Gate>>> = OnceLock::new();
    GATE.get_or_init(Default::default)
}

fn plock<T: 'static>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    let notify = {
        let mut gate = gate().lock().unwrap();
        if gate.as_ref().is_some_and(|g| g.kind == TypeId::of::<T>()) {
            gate.take()
        } else {
            None
        }
    };
    if let Some(gate) = notify {
        gate.reached.send(()).unwrap();
    }
    mutex.lock().unwrap()
}

fn write_input(conn: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    conn.extend_from_slice(bytes);
    Ok(())
}

// PRODUCTION_PASTE_FUNCTIONS

fn blocked_paste<T: 'static>(term: &Arc<Term>, lock: &Mutex<T>, leave: bool) {
    let held = lock.lock().unwrap();
    let (reached, at_lock) = mpsc::channel();
    *gate().lock().unwrap() = Some(Gate {
        kind: TypeId::of::<T>(),
        reached,
    });
    let stopped = Arc::new(AtomicBool::new(false));
    let pending = term_paste(term.clone(), "cancelled-file-path".into(), stopped.clone()).unwrap();
    at_lock
        .recv_timeout(Duration::from_secs(2))
        .expect("paste reached the held production lock");
    if leave {
        term.left.store(true, Ordering::Relaxed);
    } else {
        stopped.store(true, Ordering::Relaxed);
    }
    drop(held);
    let result = pending.wait();
    assert!(
        term.conn.lock().unwrap().is_empty(),
        "cancelled paste reached the input sink"
    );
    let expected = if leave {
        "attachment is closed"
    } else {
        "cancelled"
    };
    assert_eq!(result, Err(expected.into()));
}

fn term() -> Arc<Term> {
    Arc::new(Term {
        commands: Default::default(),
        left: AtomicBool::new(false),
        parser: Mutex::new(Parser),
        conn: Mutex::new(Vec::new()),
    })
}

#[test]
fn cancelled_paste_waiting_for_parser_sends_no_input() {
    let term = term();
    blocked_paste(&term, &term.parser, false);
}

#[test]
fn cancelled_paste_waiting_for_writer_sends_no_input() {
    let term = term();
    blocked_paste(&term, &term.conn, false);
}

#[test]
fn closed_attachment_waiting_for_writer_sends_no_input() {
    let term = term();
    blocked_paste(&term, &term.conn, true);
}

#[test]
fn uncancelled_paste_still_sends_input() {
    let term = term();
    term_paste(
        term.clone(),
        "file-path".into(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
    .wait()
    .unwrap();
    assert_eq!(&*term.conn.lock().unwrap(), b"file-path");
}
