//! Bounded session events recorded from host reports, never inferred from silence.
use crate::State;
use serde::{Deserialize, Serialize};

pub const RETAINED_EVENTS: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    NeedsInput,
    TurnCompleted,
    Failed,
    CommandFinished,
    Notice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct ActivityEvent {
    pub id: String,
    pub kind: ActivityKind,
    #[ts(type = "number")]
    pub at_ms: u64,
    pub resolved: bool,
    pub text: Option<String>,
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    pub exit: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityLog {
    sequence: u64,
    last: Option<(String, State, Option<i32>)>,
    pub events: Vec<ActivityEvent>,
}

impl ActivityLog {
    pub fn report(&mut self, run: &str, word: State, exit: Option<i32>, at_ms: u64) {
        let current = (run.to_owned(), word, exit);
        if self.last.as_ref() == Some(&current) {
            return;
        }
        let same_pending = self
            .last
            .as_ref()
            .is_some_and(|(old_run, old_word, old_exit)| {
                old_run == run && *old_word == State::Blocked && old_exit.is_none()
            });
        let same_failure = self
            .last
            .as_ref()
            .is_some_and(|(old_run, old_word, old_exit)| {
                old_run == run
                    && (*old_word == State::Errored
                        || old_exit.is_some_and(|code| !matches!(code, 0 | 130 | 141)))
            });
        self.last = Some(current);
        if word != State::Blocked || exit.is_some() || !same_pending {
            for event in &mut self.events {
                if event.kind == ActivityKind::NeedsInput {
                    event.resolved = true;
                }
            }
        }
        let kind = if let Some(code) = exit {
            if matches!(code, 0 | 130 | 141) {
                None
            } else {
                Some(ActivityKind::Failed)
            }
        } else {
            match word {
                State::Blocked if !same_pending => Some(ActivityKind::NeedsInput),
                State::Done => Some(ActivityKind::TurnCompleted),
                State::Errored => Some(ActivityKind::Failed),
                _ => None,
            }
        };
        let Some(kind) = kind else { return };
        if kind == ActivityKind::Failed && same_failure {
            if let Some(event) = self
                .events
                .last_mut()
                .filter(|event| event.kind == ActivityKind::Failed)
            {
                event.exit = exit.or(event.exit);
                return;
            }
        }
        self.sequence = self.sequence.saturating_add(1);
        self.events.push(ActivityEvent {
            id: format!("{run}:{}", self.sequence),
            kind,
            at_ms,
            resolved: false,
            text: None,
            duration_ms: None,
            exit,
        });
        if self.events.len() > RETAINED_EVENTS {
            self.events.drain(..self.events.len() - RETAINED_EVENTS);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_completion_is_distinct_from_idle_and_duplicate_reports() {
        let mut log = ActivityLog::default();
        log.report("run", State::Busy, None, 1);
        log.report("run", State::Idle, None, 2);
        assert!(log.events.is_empty());
        log.report("run", State::Done, None, 3);
        log.report("run", State::Done, None, 4);
        assert_eq!(log.events.len(), 1);
        assert_eq!(log.events[0].at_ms, 3);
    }

    #[test]
    fn pending_is_resolved_by_the_host_and_survives_serialization() {
        let mut log = ActivityLog::default();
        log.report("old", State::Blocked, None, 1);
        log.report("old", State::Blocked, None, 2);
        let mut restored: ActivityLog =
            serde_json::from_slice(&serde_json::to_vec(&log).unwrap()).unwrap();
        restored.report("new", State::Blocked, None, 3);
        assert!(restored.events[0].resolved);
        assert!(!restored.events[1].resolved);
        assert_ne!(restored.events[0].id, restored.events[1].id);
        restored.report("new", State::Busy, None, 4);
        assert!(restored.events.iter().all(|event| event.resolved));
    }

    #[test]
    fn retains_a_bounded_tail_and_ignores_cancelled_exits() {
        let mut log = ActivityLog::default();
        for n in 0..100 {
            log.report("run", State::Busy, None, n * 2);
            log.report("run", State::Done, None, n * 2 + 1);
        }
        assert_eq!(log.events.len(), RETAINED_EVENTS);
        let last = log.events.last().unwrap().id.clone();
        log.report("run", State::Idle, Some(130), 300);
        assert_eq!(log.events.last().unwrap().id, last);
        log.report("run", State::Idle, Some(1), 301);
        assert_eq!(log.events.last().unwrap().kind, ActivityKind::Failed);
    }

    #[test]
    fn a_failed_turn_and_its_exit_are_one_incident() {
        let mut log = ActivityLog::default();
        log.report("run", State::Errored, None, 1);
        log.report("run", State::Idle, Some(1), 2);
        assert_eq!(log.events.len(), 1);
        assert_eq!(log.events[0].exit, Some(1));
        log.report("other", State::Errored, None, 3);
        assert_eq!(log.events.len(), 2);
    }
}
