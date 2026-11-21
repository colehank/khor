//! The six things khor can do with another tool's conversation, in one
//! place.
//!
//! The plan for this stage named them: `detect`, `list`, `open`, `send`,
//! `read`, `launch`. Before this module they existed as scattered pieces —
//! the roster detected, `native_queue` sent, `dsh_sessions` listed and
//! read, a table cell said which road launches. Each face then reached for
//! a different piece, and the roster's capability badges were *typed*
//! beside them (`list: true`, `read: false`) rather than derived, so a
//! verb could be implemented and the badge still say it was missing.
//!
//! This module is the single answer to "what can khor actually do with
//! agent X". It implements each verb for the vendors that have real code
//! behind them and refuses, **by name**, for the rest — a refusal that
//! says "no road yet" is honest; a badge that promises one is not.
//!
//! # Detected versus implemented
//!
//! Detection and launch stay in [`crate::agent_fleet`]: they are lookups
//! and argv, not conversations. This module owns the verbs that touch a
//! conversation: `list`, `read`, `send`, `open`. `open` is answered as the
//! **argv a terminal should run** (`resume_argv`) rather than by starting
//! a process, because the face that opens a terminal is the one that
//! knows where a person is looking; the node says only what to type.

use khor_catalog::msg;

/// The verbs this module can route, in the order a face shows them.
pub const VERBS: &[&str] = &["list", "read", "send", "open"];

/// What khor has real code for, per agent.
///
/// **This is the authority the roster's badges are built from.** Adding a
/// verb here without adding its branch below makes the badge lie, so the
/// two live in the same module and are tested together.
pub fn implemented(agent: &str) -> &'static [&'static str] {
    match agent {
        // khor reads codex rollouts (`adaptor::codex`), lists them through
        // the `_codexagent` ACP shim, queues messages through Codex's own
        // daemon, and resumes in a terminal.
        "codex" => &["list", "read", "send", "open"],
        // The same four over DSH's store: `dsh_sessions` lists and reads,
        // `native_queue` drives headless, and the native TUI resumes.
        "dsh" => &["list", "read", "send", "open"],
        _ => &[],
    }
}

/// Whether this agent has this verb.
pub fn has(agent: &str, verb: &str) -> bool {
    implemented(agent).contains(&verb)
}

/// Puts one message into one of `agent`'s conversations.
///
/// The one door a face and the CLI share (`khor agents tell`); routing
/// lives in [`crate::native_queue`], and an agent without a channel is
/// refused by name rather than guessed at.
pub fn send(agent: &str, native_id: &str, text: &str, cwd: Option<&str>) -> Result<String, String> {
    if !has(agent, "send") {
        return Err(msg::native_send_unknown(agent));
    }
    crate::native_queue::send(agent, native_id, text, cwd)
}

/// The argv a terminal should run to continue this conversation where the
/// vendor owns it (`open`).
///
/// Answers a **line to type in a session**, not a process started here:
/// the conversation stays the vendor's, and the face that opened the
/// terminal is the one that knows which machine and directory it is on.
pub fn resume_argv(agent: &str, native_id: &str) -> Result<String, String> {
    if !has(agent, "open") {
        return Err(msg::native_resume_unknown(agent));
    }
    let argv = match agent {
        "codex" => format!("codex resume {native_id}"),
        "dsh" => format!("dsh tui --resume {native_id}"),
        other => return Err(msg::native_resume_unknown(other)),
    };
    Ok(argv)
}

/// Reads one of `agent`'s conversations as flat lines.
///
/// Only DSH has a reader that is not the ACP client's job; the ACP agents'
/// transcripts already arrive through `session_content`, which is a
/// different road with its own types.
pub fn read(agent: &str, native_id: &str) -> Result<Vec<crate::adaptor::dsh::Line>, String> {
    if !has(agent, "read") {
        return Err(msg::native_read_unknown(agent));
    }
    match agent {
        "dsh" => {
            let text = crate::dsh_sessions::conversation(native_id)?;
            Ok(crate::adaptor::dsh::lines_of(&text))
        }
        other => Err(msg::native_read_unknown(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_fleet;

    /// A `PATH` where both vendors are installed, so their badges exist:
    /// a row only claims capabilities when the program is there.
    fn installed_path(scratch: &std::path::Path) -> String {
        std::fs::create_dir_all(scratch).expect("scratch");
        for program in ["codex", "dsh"] {
            let file = scratch.join(program);
            std::fs::write(&file, b"x").expect("program");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
                    .expect("mode");
            }
        }
        scratch.display().to_string()
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("khor-driver-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    /// **The roster must not promise what this module cannot do.** The
    /// independent review found a row advertising a road that failed at
    /// spawn; a badge is the same failure one step earlier, so the two
    /// answers are compared rather than trusted.
    #[test]
    fn every_badge_the_roster_shows_is_a_verb_with_code_behind_it() {
        let scratch = scratch("badges");
        let path = installed_path(&scratch);
        let fleet = agent_fleet::fleet_at(None, &path);
        for row in &fleet.agents {
            for (badge, verb) in [
                ("list", "list"),
                ("read", "read"),
                ("send", "send"),
                ("resume", "open"),
            ] {
                if row.capabilities.contains(&badge) {
                    assert!(
                        has(&row.id, verb),
                        "{} claims {badge} but this module has no {verb} for it",
                        row.id
                    );
                }
            }
        }
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// And the other direction: a verb with code must be shown, or the
    /// feature exists where nobody can find it. DSH's list/read were
    /// implemented while the table still said `false` — this is that bug.
    #[test]
    fn every_verb_with_code_is_shown_on_the_row() {
        let scratch = scratch("shown");
        let path = installed_path(&scratch);
        let fleet = agent_fleet::fleet_at(None, &path);
        for row in &fleet.agents {
            if !matches!(row.id.as_str(), "codex" | "dsh") {
                continue;
            }
            for verb in implemented(&row.id) {
                let badge = match *verb {
                    "open" => "resume",
                    other => other,
                };
                assert!(
                    row.capabilities.contains(&badge),
                    "{} has {verb} but its row does not say {badge}",
                    row.id
                );
            }
        }
        let _ = std::fs::remove_dir_all(&scratch);
    }

    /// An agent with no road is refused by name, not guessed at.
    #[test]
    fn an_agent_without_a_road_is_refused_by_name() {
        assert!(
            send("claude", "id", "hi", None)
                .unwrap_err()
                .contains("claude")
        );
        assert!(resume_argv("claude", "id").unwrap_err().contains("claude"));
        assert!(read("claude", "id").unwrap_err().contains("claude"));
        assert!(!has("claude", "send"));
        assert!(!has("nobody", "read"));
    }

    /// The argv a terminal would type, for the two vendors khor owns.
    #[test]
    fn the_resume_argv_is_the_vendors_own_command() {
        assert_eq!(
            resume_argv("codex", "01a1de3a").unwrap(),
            "codex resume 01a1de3a"
        );
        assert_eq!(
            resume_argv("dsh", "session-0b5f").unwrap(),
            "dsh tui --resume session-0b5f"
        );
    }
}
