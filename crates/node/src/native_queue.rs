//! Putting a message into a conversation the vendor keeps.
//!
//! `khor send`/`khor tell` drive **khor's own** sessions. This is the other
//! case: a conversation another tool owns — Codex or DSH — and a person
//! wants to say one thing to it without taking it over.
//!
//! # The two vendors, and why these channels
//!
//! **Codex** ships `codex queue --thread <id> --message <text>`: the write
//! goes through Codex's own daemon, so there is no second writer and no
//! writer lock to fight — the reason a takeover of a *loaded* thread is
//! refused elsewhere in this crate.
//!
//! **DSH** ships a headless run that adopts a persisted session:
//! `dsh --profile headless --json --session-id <id> <task>`. It answers one
//! turn (the session continues afterwards) and the `--json` events carry
//! the answer in a final `{"type":"final"}` frame, or the refusal in its
//! `reason`. One turn per invocation is the shape DSH offers; khor does not
//! pretend it is a long-lived conversation it owns.
//!
//! # Why an argv and not a line
//!
//! The text is a person's sentence: quotes, newlines, `%`, `!` are all
//! ordinary. It is handed to `Command` as one argument, never to a shell,
//! so nothing in it can become syntax. A `.cmd`/`.bat` program is the one
//! case Windows cannot start directly — it is wrapped in an explicit
//! command interpreter (`cmd.exe /c`), the same road the ACP sidecar
//! registration already takes.
//!
//! # What "supported" means here
//!
//! Only vendors whose argv this tree has verified. An unknown agent is
//! refused by name rather than guessed at: a wrong flag would look like a
//! message that was sent.

use khor_catalog::msg;
use std::process::Stdio;

/// The longest message this will carry. Codex takes more; a person
/// pasting a file by accident should be told, not have it injected.
const MAX_TEXT: usize = 16 * 1024;

/// One program and its arguments, before the program is looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spawn {
    pub program: String,
    pub args: Vec<String>,
}

/// Sends `text` into `native_id` through the vendor that owns it.
///
/// `cwd` is where the vendor expects to stand. **DSH requires it**: a
/// session adopted by `--session-id` must be resumed from the directory it
/// was recorded in, and DSH refuses otherwise with
/// `session "…" was recorded in "…", not "…"`. Codex's queue does not care,
/// and an empty `cwd` leaves the process where it is.
pub fn send(agent: &str, native_id: &str, text: &str, cwd: Option<&str>) -> Result<String, String> {
    let spawn = command_for(agent, native_id, text)?;
    let path = crate::Node::user_path();
    let found = crate::find_program(&spawn.program, path)
        .ok_or_else(|| msg::native_send_no_program(&spawn.program))?;
    let (program, args) = invocation(&found, spawn.args);
    let mut command = crate::console::hidden(&program);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd.map(str::trim).filter(|dir| !dir.is_empty()) {
        command.current_dir(dir);
    }
    let out = command
        .output()
        .map_err(|error| msg::wont_start(&program, error))?;
    let said = String::from_utf8_lossy(&out.stdout);
    let trouble = String::from_utf8_lossy(&out.stderr);
    match agent {
        // The events carry the answer and the refusal in their own frames,
        // so a failed run is read from stdout before its status is trusted:
        // DSH exits 0 on a turn that errored inside the model call.
        "dsh" => dsh_answer(
            &said,
            &trouble,
            out.status.success(),
            out.status.code().unwrap_or(-1),
        ),
        _ => {
            if out.status.success() {
                let line = said
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
                    .trim();
                return Ok(if line.is_empty() {
                    msg::native_send_accepted(&spawn.program)
                } else {
                    line.to_owned()
                });
            }
            let why = if trouble.trim().is_empty() {
                said.trim().to_owned()
            } else {
                trouble.trim().to_owned()
            };
            Err(if why.is_empty() {
                msg::native_send_refused(&spawn.program, out.status)
            } else {
                why
            })
        }
    }
}

/// The argv for one vendor, or the sentence saying why there is none.
///
/// Split out so the argument shape is testable without starting a process
/// — a wrong flag would be refused by the vendor with a sentence about a
/// missing argument, which reads like the id being wrong.
pub(crate) fn command_for(agent: &str, native_id: &str, text: &str) -> Result<Spawn, String> {
    if text.trim().is_empty() {
        return Err(msg::native_send_empty().into());
    }
    if text.len() > MAX_TEXT {
        return Err(msg::native_send_long(text.len(), MAX_TEXT));
    }
    if text.contains('\0') || native_id.contains('\0') || native_id.is_empty() {
        return Err(msg::native_send_nul().into());
    }
    let args = match agent {
        "codex" => vec![
            "queue".to_owned(),
            "--thread".to_owned(),
            native_id.to_owned(),
            "--message".to_owned(),
            text.to_owned(),
        ],
        "dsh" => vec![
            "--profile".to_owned(),
            "headless".to_owned(),
            // Machine-readable events: the answer and the refusal are told
            // apart by their frames, not by guessing at prose.
            "--json".to_owned(),
            "--session-id".to_owned(),
            native_id.to_owned(),
            text.to_owned(),
        ],
        other => return Err(msg::native_send_unknown(other)),
    };
    Ok(Spawn {
        program: agent.to_owned(),
        args,
    })
}

/// The program and arguments to actually start.
///
/// A `.cmd`/`.bat` is not a program Windows can start by itself, and khor
/// never goes through a shell to reach one: it names the interpreter
/// explicitly, the same way an ACP sidecar registered as a batch file is
/// started (the owned process adapter refuses a bare batch for this
/// reason).
fn invocation(found: &str, args: Vec<String>) -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        let lower = found.to_ascii_lowercase();
        if lower.ends_with(".cmd") || lower.ends_with(".bat") {
            let mut wrapped = Vec::with_capacity(args.len() + 2);
            wrapped.push("/c".to_owned());
            wrapped.push(found.to_owned());
            wrapped.extend(args);
            return ("cmd.exe".to_owned(), wrapped);
        }
    }
    (found.to_owned(), args)
}

/// Readings of DSH's `--json` event stream.
///
/// One frame per line. The final one carries `text` (the answer) and, when
/// the turn failed, a `reason` whose `kind` is `error` with the vendor's
/// own message in it. Anything else — a run that died before its final
/// frame — falls back to its stderr.
fn dsh_answer(said: &str, trouble: &str, ok: bool, code: i32) -> Result<String, String> {
    for line in said.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if frame.get("type").and_then(|t| t.as_str()) != Some("final") {
            continue;
        }
        let failed = frame
            .pointer("/reason/kind")
            .and_then(|k| k.as_str())
            .is_some_and(|kind| kind == "error");
        if failed {
            let why = frame
                .pointer("/reason/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or("")
                .trim();
            return Err(if why.is_empty() {
                msg::native_send_refused("dsh", code)
            } else {
                why.to_owned()
            });
        }
        let answer = frame
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim();
        return Ok(if answer.is_empty() {
            msg::native_send_accepted("dsh")
        } else {
            answer.to_owned()
        });
    }
    let why = trouble.trim();
    if why.is_empty() && ok {
        // The run said nothing and failed nothing: one turn happened and
        // had no answer to give. Claiming more than that would be inventing
        // the vendor's words.
        return Ok(msg::native_send_accepted("dsh"));
    }
    Err(if why.is_empty() {
        msg::native_send_refused("dsh", code)
    } else {
        why.to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_gets_one_argument_for_the_message() {
        let spawn = command_for(
            "codex",
            "01a1de3a-93d7-7791-93da-5d5a31cfc817",
            "hi \"there\"\n!",
        )
        .expect("codex is known");
        assert_eq!(spawn.program, "codex");
        assert_eq!(
            spawn.args,
            vec![
                "queue",
                "--thread",
                "01a1de3a-93d7-7791-93da-5d5a31cfc817",
                "--message",
                "hi \"there\"\n!",
            ]
        );
    }

    #[test]
    fn dsh_adopts_the_session_and_asks_for_event_frames() {
        let spawn = command_for("dsh", "session-0b5f9684", "run the tests").expect("dsh is known");
        assert_eq!(spawn.program, "dsh");
        assert_eq!(
            spawn.args,
            vec![
                "--profile",
                "headless",
                "--json",
                "--session-id",
                "session-0b5f9684",
                "run the tests",
            ]
        );
    }

    #[test]
    fn an_unknown_vendor_is_refused_by_name_and_nothing_is_invented() {
        let why = command_for("claude", "x", "hi").expect_err("no claude path yet");
        assert!(why.contains("claude"), "{why}");
    }

    #[test]
    fn empty_and_oversized_messages_are_refused_before_anything_starts() {
        assert!(command_for("codex", "id", "   ").is_err());
        assert!(command_for("dsh", "id", &"x".repeat(MAX_TEXT + 1)).is_err());
        assert!(command_for("codex", "", "hi").is_err());
    }

    /// The answer comes out of the event frame, not out of stdout prose.
    #[test]
    fn a_final_frame_carries_the_answer() {
        let said = r#"{"type":"session","sessionId":"s"}
{"type":"final","sessionId":"s","reason":{"kind":"completed"},"text":"all green"}"#;
        assert_eq!(dsh_answer(said, "", true, 0).unwrap(), "all green");
    }

    /// A turn that failed inside the model call still exits 0 and still
    /// says why: the refusal is the vendor's message from its own frame.
    #[test]
    fn a_failed_final_frame_is_a_refusal_with_the_vendors_words() {
        let said = r#"{"type":"final","sessionId":"s","reason":{"kind":"error","error":{"message":"no API key","code":"MISSING_CREDENTIAL"}},"text":""}"#;
        let why = dsh_answer(said, "", true, 0).expect_err("failed turn is not success");
        assert_eq!(why, "no API key");
    }

    /// No final frame at all — the run died earlier — falls back to stderr,
    /// refuses when the run failed with nothing to say, and accepts only a
    /// quiet success.
    #[test]
    fn a_stream_without_a_final_frame_falls_back_rather_than_inventing_an_answer() {
        assert_eq!(dsh_answer("", "boom", false, 1).unwrap_err(), "boom");
        assert!(
            dsh_answer("", "", false, 1).is_err(),
            "a failed run that said nothing is a refusal, not success"
        );
        assert!(
            dsh_answer("", "", true, 0).is_ok(),
            "a quiet success is still a success"
        );
    }
}
