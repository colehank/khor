//! **A failure said in parts** (NOTICE-BRIEF, the owner 2026-10-08): what
//! went wrong, the machine it went wrong with, and the words the network
//! or that machine came back with — never one sentence with English
//! inside it. The face says the kind in its own words, on the session's
//! pill (「图片未传到 Mac-mini」, 「重新连接 Mac-mini」); the raw words
//! are shown only under 详情.
//!
//! **Read back from the sentence, not carried beside it.** A refusal
//! crosses from the resident and from far machines as a catalog sentence
//! (`Reply::Refused { why }`), and the catalog reads one back into its key
//! and arguments (`khor_catalog::read`, the same matching that says a far
//! refusal again in this language). So the kind is the key that said it,
//! and the raw words are the argument that is not ours: the dial's
//! `{1}` in 「无法连接 {0}：{1}」. A sentence no entry says is `other`,
//! whole, under 详情.

use serde::Serialize;
use ts_rs::TS;

/// What kind of failure. The pill shows two families: a transfer that
/// did not get there (all but `host-lost`), and a session that cannot go
/// on for now (`unreached`, `host-lost`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum MissKind {
    /// The machine could not be dialled (timed out, refused, no road).
    Unreached,
    /// The machine answered, but khor's host for this session there did
    /// not (a far terminal's pipe was bound and its handshake failed).
    HostLost,
    /// This machine's own khor resident stopped answering midway.
    ServeGone,
    /// The far khor is too old to take what was sent.
    Predates,
    /// Over the size a drop takes.
    TooBig,
    /// Somebody cancelled it: not a failure to show.
    Cancelled,
    /// Anything else; the words say what.
    Other,
}

/// One failure, in parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Miss {
    pub kind: MissKind,
    /// The machine involved, when the failure names one.
    pub machine: Option<String>,
    /// The words it came back with, for 详情 only; empty when there are
    /// none yet (a pipe that ended, its first try not answered).
    pub raw: String,
}

impl Miss {
    pub fn new(kind: MissKind, machine: Option<&str>, raw: impl Into<String>) -> Miss {
        Miss { kind, machine: machine.map(str::to_owned), raw: raw.into() }
    }
}

/// How deep a sentence is looked into for one we know: a refusal inside a
/// frame inside a frame is as far as khor nests them.
const DEPTH: usize = 3;

/// `said` in parts. `machine` is the machine the caller was talking to,
/// for a sentence that does not name one itself.
pub fn read(said: &str, machine: Option<&str>) -> Miss {
    known(said, DEPTH).unwrap_or_else(|| Miss::new(MissKind::Other, machine, said)).or_machine(machine)
}

impl Miss {
    fn or_machine(mut self, machine: Option<&str>) -> Miss {
        // This machine's own resident is not a far machine.
        if self.machine.is_none() && self.kind != MissKind::ServeGone {
            self.machine = machine.map(str::to_owned);
        }
        self
    }
}

fn known(said: &str, depth: usize) -> Option<Miss> {
    let (key, args) = khor_catalog::read(said)?;
    let arg = |i: usize| args.get(i).map(String::as_str);
    let miss = match key {
        "msg.cant-reach-named" => Miss::new(MissKind::Unreached, arg(0), arg(1).unwrap_or_default()),
        "msg.cant-reach-plain" => Miss::new(MissKind::Unreached, None, arg(0).unwrap_or_default()),
        "msg.machine-unreachable-timeout" => Miss::new(MissKind::Unreached, arg(0), said),
        "msg.peer-backing-off" => Miss::new(MissKind::Unreached, None, said),
        "msg.drop-serve-gone" | "msg.reach-needs-serve" => Miss::new(MissKind::ServeGone, None, said),
        "msg.drop-predates" | "msg.landed-unsaid" => Miss::new(MissKind::Predates, None, said),
        "msg.drop-too-big" => Miss::new(MissKind::TooBig, None, said),
        "msg.drop-cancelled" => Miss::new(MissKind::Cancelled, None, said),
        // A frame around a sentence: the one inside may be ours.
        _ if depth > 1 => return args.iter().find_map(|a| known(a, depth - 1)),
        _ => return None,
    };
    Some(miss)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IMG_5413: 「无法连接 localhost：aborted by peer…」 on the pill came
    /// back as one sentence. In parts, the English is only the raw words.
    #[test]
    fn a_dial_failure_comes_apart() {
        let raw = "aborted by peer: the server refused to accept a new connection";
        let m = read(&khor_catalog::msg::cant_reach_named("Mac-mini", raw), Some("turing"));
        assert_eq!(m, Miss::new(MissKind::Unreached, Some("Mac-mini"), raw));
    }

    #[test]
    fn the_kinds_a_drop_ends_in() {
        let kind = |s: String| read(&s, Some("turing")).kind;
        assert_eq!(kind(khor_catalog::msg::drop_serve_gone().to_owned()), MissKind::ServeGone);
        assert_eq!(kind(khor_catalog::msg::drop_predates().to_owned()), MissKind::Predates);
        assert_eq!(kind(khor_catalog::msg::drop_cancelled().to_owned()), MissKind::Cancelled);
        assert_eq!(read(khor_catalog::msg::drop_serve_gone(), Some("turing")).machine, None, "this machine's own resident");
        let other = read("Permission denied (os error 13)", Some("turing"));
        assert_eq!(other, Miss::new(MissKind::Other, Some("turing"), "Permission denied (os error 13)"));
    }
}
