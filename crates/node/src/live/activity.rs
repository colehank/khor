use khor_core::activity::{ActivityEvent, ActivityKind, RETAINED_EVENTS};

use super::{LiveState, Meta};

pub(super) fn session_events(
    meta: &Meta,
    state: &LiveState,
    saved: Option<crate::commands::Saved>,
) -> Vec<ActivityEvent> {
    let mut events = meta.activity.events.clone();
    if state.exit.is_some() {
        for event in &mut events {
            if event.kind == ActivityKind::NeedsInput {
                event.resolved = true;
            }
        }
    }
    if let Some(saved) = saved {
        events.extend(command_events(saved));
    }
    if events.len() > RETAINED_EVENTS {
        // Keep request resolutions when a busy shell's command tail would otherwise evict them.
        events.sort_by_key(|event| (event.kind == ActivityKind::NeedsInput, event.at_ms));
        events.drain(..events.len() - RETAINED_EVENTS);
    }
    events.sort_by_key(|event| event.at_ms);
    events
}

fn command_events(saved: crate::commands::Saved) -> Vec<ActivityEvent> {
    let mut events: Vec<_> = saved
        .recent
        .into_iter()
        .filter_map(|command| {
            let ended = command.ended_ms?;
            Some(ActivityEvent {
                id: format!(
                    "command:{}:{}:{}",
                    command.started_ms,
                    command.n,
                    command.pane.as_deref().unwrap_or("")
                ),
                kind: ActivityKind::CommandFinished,
                at_ms: ended,
                resolved: false,
                text: command.text.map(|text| text.chars().take(160).collect()),
                duration_ms: Some(ended.saturating_sub(command.started_ms)),
                exit: command.exit,
            })
        })
        .collect();
    if let Some(text) = saved.doing.notice.filter(|_| saved.doing.notice_ms > 0) {
        events.push(ActivityEvent {
            id: format!("notice:{}", saved.doing.notice_ms),
            kind: ActivityKind::Notice,
            at_ms: saved.doing.notice_ms,
            resolved: false,
            text: Some(text.chars().take(160).collect()),
            duration_ms: None,
            exit: None,
        });
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_replays_finished_commands_without_observing_them_running() {
        let command = |n, ended_ms| crate::commands::Command {
            n,
            text: Some("x".repeat(300)),
            cwd: None,
            started_ms: n * 100,
            ended_ms,
            exit: Some(0),
            at: 0,
            end_at: None,
            pane: None,
            last_line: None,
        };
        let saved = crate::commands::Saved {
            recent: vec![
                command(1, Some(1000)),
                command(2, Some(1100)),
                command(3, None),
            ],
            ..Default::default()
        };
        let events = command_events(saved.clone());
        assert_eq!(events, command_events(saved));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].duration_ms, Some(900));
        assert_eq!(events[0].text.as_ref().unwrap().len(), 160);
        assert_ne!(events[0].id, events[1].id);
    }
}
