use super::*;

#[test]
fn one_shot_termination_has_no_pi_grace_and_cannot_be_revoked() {
    let control = Control::default();
    control.terminate_now();
    let first = control.0.lock().unwrap().terminal_deadline.unwrap();
    assert!(first <= Instant::now());
    assert!(control.is_cancelled());
    control.stop();
    assert_eq!(control.0.lock().unwrap().terminal_deadline, Some(first));
    assert!(control.begin_operation().is_none());
}

#[test]
fn an_old_ack_cannot_clear_a_new_operations_deadline() {
    let control = Control::default();
    let first = control.begin_operation().unwrap();
    assert!(control.complete_operation(&first));
    let second = control.begin_operation().unwrap();
    assert_ne!(first, second);
    assert!(control.cancel_operation(&second, Duration::from_millis(10)));
    assert!(!control.complete_operation(&first));
    std::thread::sleep(Duration::from_millis(15));
    assert!(!control.complete_operation(&second));
    assert!(control.0.lock().unwrap().terminating);
    assert!(control.begin_operation().is_none());
}

#[test]
fn a_stop_after_expiry_cannot_restart_the_operations_grace() {
    let control = Control::default();
    let operation = control.begin_operation().unwrap();
    assert!(control.cancel_operation(&operation, Duration::ZERO));
    assert!(!control.cancel_operation(&operation, Duration::from_secs(30)));
    assert!(!control.complete_operation(&operation));
    assert!(control.begin_operation().is_none());
}

#[test]
fn cooperative_completion_allows_the_next_operation() {
    let control = Control::default();
    let first = control.begin_operation().unwrap();
    assert!(control.cancel_operation(&first, Duration::from_secs(1)));
    assert!(control.complete_operation(&first));
    let second = control.begin_operation().unwrap();
    assert!(!control.cancel_operation(&first, Duration::ZERO));
    assert!(control.complete_operation(&second));
}

#[test]
fn an_ack_cannot_revoke_permanent_shutdown_or_natural_exit() {
    for natural in [false, true] {
        let control = Control::default();
        let operation = control.begin_operation().unwrap();
        if natural {
            control.natural_exit();
        } else {
            control.request_terminal(Duration::from_millis(10), false);
        }
        let deadline = control.0.lock().unwrap().terminal_deadline;
        let _ = control.complete_operation(&operation);
        assert_eq!(control.0.lock().unwrap().terminal_deadline, deadline);
        assert!(control.begin_operation().is_none());
    }
}

#[test]
fn an_ack_at_expiry_commits_termination_without_waiting_for_the_supervisor() {
    let control = Arc::new(Control::default());
    let operation = control.begin_operation().unwrap();
    assert!(control.cancel_operation(&operation, Duration::ZERO));
    let ack = {
        let control = control.clone();
        let operation = operation.clone();
        std::thread::spawn(move || control.complete_operation(&operation))
    };
    assert!(!ack.join().unwrap());
    assert!(control.0.lock().unwrap().terminating);
    assert!(!control.complete_operation(&operation));
}

#[test]
fn a_ticket_from_another_process_owner_cannot_ack_or_cancel_its_first_turn() {
    let old = Control::default();
    let new = Control::default();
    let old_ticket = old.begin_operation().unwrap();
    let new_ticket = new.begin_operation().unwrap();
    assert_ne!(old_ticket, new_ticket);
    assert!(!new.complete_operation(&old_ticket));
    assert!(!new.cancel_operation(&old_ticket, Duration::ZERO));
    assert!(new.complete_operation(&new_ticket));
}
