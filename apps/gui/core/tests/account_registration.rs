use khor_node::Node;
use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn one_account_save_waits_for_short_root_contention_and_registers_once() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "khor-account-save-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    )));
    drop(Node::open(fixture.0.clone()).unwrap());
    let gate = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.0.join(".khor/trust-mutation/gate"))
        .unwrap();
    gate.lock().unwrap();
    let owner = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        drop(gate);
    });
    let saved = khor_gui_core::register_account(
        &fixture.0,
        "one save",
        "deepseek",
        "bill",
        None,
        "fixture-only-key",
        None,
    );
    owner.join().unwrap();
    let id = saved.expect("one foreground save should outwait short background contention");
    let node = Node::open(fixture.0.clone()).unwrap();
    let accounts = node.accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, id);
    assert_eq!(
        fs::read_dir(fixture.0.join(".khor/keys")).unwrap().count(),
        1
    );
}
