//! The agent registry: which ACP agents this person has told khor
//! about, replicated everywhere — the open half of "whose session is
//! this" (批⑥, khor as a terminal IDE: any agent that speaks ACP plugs
//! into any client).
//!
//! # Why a replicated document and not a preference file
//!
//! khor has no user configuration file, on purpose: everything the user
//! *states* is a document that travels ([`crate::pins`],
//! [`crate::dirpins`], [`crate::webpins`]). Naming an agent is the same
//! class of fact — said once, true on every machine — and the network's
//! own shape is that joining anywhere joins everything.
//!
//! # The key carries no machine, unlike [`crate::dirpins`] and [`crate::webpins`]
//!
//! Those two key by machine because the *thing* is machine-relative: a
//! directory on one disk, a page through one exit. A registration is not
//! — it is the user saying "this is what I call that agent, and this is
//! how it starts". Whether the command exists on a given machine is a
//! separate question, and one only that machine can answer, at the
//! moment it tries. `claude` and `codex` already live this way: their
//! names are global, and a machine without the binary answers when it
//! is asked to run one, not before.
//!
//! So an agent registered on the laptop appears in the desk's wizard,
//! and opening it on a desk that lacks the binary fails **naming the
//! command** — which is the true answer, and a more useful one than
//! hiding the agent as though the user had never named it.
//!
//! # This document carries no secrets, and that is a boundary not an omission
//!
//! A [`Spec`] is a command and its arguments. It has no environment,
//! because this file replicates to every machine in the network: an API
//! key stored here would be copied to all of them, and khor would be
//! carrying a credential for the first time in its life — one machine
//! lost would be all of them lost. An agent that needs a variable set
//! is opened the ad-hoc way instead (`khor open --gui -- '{"command":
//! …, "env": {…}}'`, the launch JSON the protocol crate reads), where
//! the value lives only on the machine somebody typed it into.
//!
//! What that does not solve, and is on the ledger: a GUI started from
//! the desktop has no login shell's environment, so a variable exported
//! in a shell profile is not there for a session the app opens.
//!
//! # Removal is a deletion here, and a tombstone in [`crate::devices`]
//!
//! The device table cannot delete, because a removed machine keeps its
//! own copy of the document and keeps re-registering itself — the row
//! would walk back in on the next merge. A registration has no such
//! second author: it is written by a person and by nobody else, so the
//! delete is one more op that replicates by the same last-writer-wins
//! rule as the write it undoes.

use loro::{LoroDoc, LoroValue, VersionVector};
use serde::{Deserialize, Serialize};

use crate::glue;
use crate::store::Doc;

pub mod registry;

pub const REL_DIR: &str = ".khor/agents";

pub fn agents_dir(home: &std::path::Path) -> std::path::PathBuf {
    home.join(REL_DIR)
}

const AGENTS: &str = "agents";
/// Where an agent's own list of its conversations is kept, keyed by the
/// same name the registration is.
///
/// **A second map rather than a field on [`Spec`].** `Spec` is the
/// protocol crate's launch JSON and nothing else — the comment above it
/// says so, and it is serialized straight into what spawns the child.
/// A list of conversations riding inside it would be khor's invention
/// travelling in a field the protocol reads.
const SEEN: &str = "seen";

/// How one agent starts.
///
/// **The field names are the protocol crate's launch JSON, not khor's
/// invention** (`AcpAgentConfig`: `command`, `args`), because this is
/// serialized straight into what actually spawns the child. One
/// spelling end to end means there is no translation step to drift: a
/// registration that round-trips through here is the very string the
/// agent is started from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

impl Spec {
    /// From what a person typed after `--`: the first word is the
    /// binary, the rest are its arguments. Taken as argv rather than as
    /// one string on purpose — a path with a space in it survives this
    /// and does not survive re-splitting.
    pub fn from_argv(argv: &[String]) -> Option<Spec> {
        let (command, args) = argv.split_first()?;
        if command.trim().is_empty() {
            return None;
        }
        Some(Spec { command: command.clone(), args: args.to_vec() })
    }

    /// The launch JSON, ready for the protocol crate to read.
    pub fn launch(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// What a person reads back: the command as they would type it.
    /// For display only — never re-parsed, since [`Spec::launch`] is
    /// what starts anything.
    pub fn typed(&self) -> String {
        let mut out = self.command.clone();
        for a in &self.args {
            out.push(' ');
            out.push_str(a);
        }
        out
    }
}

/// One conversation an agent said it has, as khor remembers it.
///
/// **Remembered because asking costs a process.** khor finds claude's
/// and codex's sessions by reading files, which is a millisecond and
/// can ride the list poll; asking a registry agent means starting the
/// agent. So the answer is taken once — at registration — and kept
/// here, where it syncs like everything else: **asked on one machine,
/// known on all of them**, and nobody else has to start that process.
///
/// It therefore goes stale, and that is the trade. What it is used for
/// is offering a conversation to continue, and an id that has since
/// been deleted refuses at load — loudly, by the agent, which is a
/// better failure than never offering anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Seen {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    /// The agent's own name for it. Absent stays absent: a conversation
    /// nobody named must not come back as an empty string on screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// RFC 3339 as the agent wrote it, kept as text — khor sorts and
    /// prints it and never does arithmetic on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// What an agent answered when it was asked for its conversations.
///
/// **The refusal is stored too, and that is the point of the enum.** An
/// agent that cannot list is a fact worth keeping: without it, "we
/// asked and it cannot tell us" comes back out of the store as "nobody
/// has asked", the screen says so, and the person puts the same
/// question again — starting the agent again — to get the same
/// nothing. Found exactly that way, on this feature's first
/// end-to-end run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Answer {
    /// It listed. Possibly nothing — and here that really is nothing.
    Sessions { rows: Vec<Seen> },
    /// It does not implement `session/list`, which the protocol allows
    /// (20 of 32 registry agents did on 2026-08-30).
    Cannot,
}

pub struct AgentDoc {
    inner: LoroDoc,
}

impl AgentDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    /// One registration, or `None` for a name nobody registered — and
    /// also for one stored in a spelling this version cannot read, which
    /// is skipped rather than guessed at ([`crate::webpins::split`]'s
    /// rule for the same situation).
    pub fn get(&self, name: &str) -> Option<Spec> {
        let value = self.inner.get_map(AGENTS).get(name)?.into_value().ok()?;
        let LoroValue::String(json) = value else { return None };
        serde_json::from_str(&json).ok()
    }

    /// Registers, or replaces a registration under the same name.
    ///
    /// Unchanged means unwritten — rewriting the same value inflates the
    /// version vector for nothing (`DeviceDoc::upsert`'s rule).
    pub fn set(&self, name: &str, spec: &Spec) -> Result<(), String> {
        if self.get(name).as_ref() == Some(spec) {
            return Ok(());
        }
        self.inner
            .get_map(AGENTS)
            .insert(name, spec.launch())
            .map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Forgets one. Removing a name nobody registered is not an error:
    /// the caller asked for a world without it, and that is the world.
    pub fn remove(&self, name: &str) -> Result<(), String> {
        if self.get(name).is_none() {
            return Ok(());
        }
        self.inner.get_map(AGENTS).delete(name).map_err(|e| e.to_string())?;
        // And what it told us about itself. A registration that comes
        // back under the same name is a different agent as far as
        // anyone here knows, and inheriting the old one's conversations
        // would offer ids that belong to something else.
        if self.seen(name).is_some() {
            self.inner.get_map(SEEN).delete(name).map_err(|e| e.to_string())?;
        }
        self.inner.commit();
        Ok(())
    }

    /// What this agent last said it has, or `None` if nobody has asked
    /// it yet.
    ///
    /// **`None` and `Some(vec![])` are different answers** and the
    /// difference is carried on purpose: never asked, versus asked and
    /// it has none. A caller that saw an empty list for both would tell
    /// a person "this agent has no past conversations" about an agent
    /// nobody had ever put the question to.
    pub fn seen(&self, name: &str) -> Option<Answer> {
        let value = self.inner.get_map(SEEN).get(name)?.into_value().ok()?;
        let LoroValue::String(json) = value else { return None };
        serde_json::from_str(&json).ok()
    }

    /// Records what an agent answered. Replaces: the agent's answer is
    /// the whole truth about itself at that moment, and merging an old
    /// list into a new one would resurrect conversations it has deleted.
    pub fn set_seen(&self, name: &str, answer: &Answer) -> Result<(), String> {
        if self.seen(name).as_ref() == Some(answer) {
            return Ok(());
        }
        let json = serde_json::to_string(answer).map_err(|e| e.to_string())?;
        self.inner.get_map(SEEN).insert(name, json).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every registration, by name, sorted — so two machines list them
    /// in the same order without either of them deciding one.
    pub fn all(&self) -> Vec<(String, Spec)> {
        let map = self.inner.get_map(AGENTS);
        let mut out: Vec<(String, Spec)> =
            map.keys().filter_map(|k| self.get(&k).map(|s| (k.to_string(), s))).collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

impl Doc for AgentDoc {
    fn open(peer: u64) -> Result<Self, String> {
        AgentDoc::new(peer)
    }

    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }

    fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }

    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        glue::changes_since(&self.inner, theirs)
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        glue::snapshot(&self.inner)
    }

    fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        glue::merge(&self.inner, bytes)
    }

    fn items(&self) -> usize {
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(command: &str, args: &[&str]) -> Spec {
        Spec {
            command: command.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    /// **The stored form is the launch form.** If these two ever came
    /// apart, a registration would list correctly and start something
    /// else — and the listing is the only place a person would look.
    #[test]
    fn the_stored_spelling_is_the_one_the_agent_is_started_from() {
        let s = spec("npx", &["-y", "some-acp-agent"]);
        let json: serde_json::Value = serde_json::from_str(&s.launch()).unwrap();
        assert_eq!(json["command"], "npx");
        assert_eq!(json["args"][0], "-y");
        assert_eq!(json["args"][1], "some-acp-agent");
        assert_eq!(json.get("env"), None, "this document carries no environment (module head)");
    }

    /// A path with a space in it survives, because argv is never
    /// re-split. The test that would go red if somebody stored
    /// `typed()` and parsed it back.
    #[test]
    fn a_command_with_a_space_in_its_path_survives_the_round_trip() {
        let argv = vec!["/opt/my agents/run-it".to_owned(), "--acp".to_owned()];
        let s = Spec::from_argv(&argv).expect("a command and one flag");
        let doc = AgentDoc::new(1).unwrap();
        doc.set("spacey", &s).unwrap();
        let back = doc.get("spacey").expect("it is registered");
        assert_eq!(back.command, "/opt/my agents/run-it");
        assert_eq!(back.args, vec!["--acp".to_owned()]);
    }

    /// Register on the phone, registered on the desk — the reason this
    /// is a document and not a preference file. And the removal travels
    /// back, which is the half a tombstone-free delete has to earn
    /// (module head).
    #[test]
    fn a_registration_travels_and_forgetting_it_travels_back() {
        let phone = AgentDoc::new(1).unwrap();
        let desk = AgentDoc::new(2).unwrap();
        phone.set("gemini", &spec("gemini", &["--acp"])).unwrap();
        desk.merge(&phone.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(
            desk.get("gemini").map(|s| s.typed()),
            Some("gemini --acp".to_owned()),
            "a registration made on the phone must reach the desk"
        );
        desk.remove("gemini").unwrap();
        phone.merge(&desk.changes_since(&Default::default()).unwrap()).unwrap();
        assert_eq!(phone.get("gemini"), None, "forgetting it must travel back");
        assert_eq!(desk.items(), 0, "a forgotten agent is not an item");
    }

    /// Listing is enumerated, not merely non-empty: "`all()` returns
    /// something" is satisfied by a list that drops half the registry,
    /// and dropping one is exactly how a wizard would quietly stop
    /// offering an agent somebody registered.
    #[test]
    fn every_registration_is_listed_under_its_own_name() {
        let doc = AgentDoc::new(1).unwrap();
        doc.set("zed", &spec("claude-code-acp", &[])).unwrap();
        doc.set("gemini", &spec("gemini", &["--acp"])).unwrap();
        doc.set("mine", &spec("/usr/local/bin/my-agent", &[])).unwrap();
        let listed: Vec<(String, String)> =
            doc.all().into_iter().map(|(n, s)| (n, s.typed())).collect();
        assert_eq!(
            listed,
            vec![
                ("gemini".to_owned(), "gemini --acp".to_owned()),
                ("mine".to_owned(), "/usr/local/bin/my-agent".to_owned()),
                ("zed".to_owned(), "claude-code-acp".to_owned()),
            ],
            "all three, by name, sorted"
        );
    }

    /// An empty argv names nothing, and a blank first word names
    /// nothing either — the second is the one that matters, because an
    /// empty category collides with the group key that means "nobody
    /// could tell whose this is" (`khor_node::list`'s `cat:` prefix).
    #[test]
    fn a_command_that_is_only_whitespace_is_not_a_command() {
        assert_eq!(Spec::from_argv(&[]), None);
        assert_eq!(Spec::from_argv(&["   ".to_owned()]), None);
        assert!(Spec::from_argv(&["x".to_owned()]).is_some(), "control: a real one is accepted");
    }

    /// **Four answers to「what has this agent done before」, and each is
    /// a different sentence to a person.**
    ///
    /// Never asked / asked and it cannot / it has none / here they are.
    /// The first two were **one** answer on this feature's first
    /// end-to-end run: a refusal was not recorded, so the store said
    /// "nobody has asked" about an agent that had just been asked — and
    /// the screen told the person to go start that agent again for the
    /// same nothing.
    ///
    /// Written as one test over the four so that the *differences* are
    /// the claim; four separate tests each pass while two of them
    /// return the same value.
    #[test]
    fn what_an_agent_has_done_before_has_four_answers() {
        let doc = AgentDoc::new(7).unwrap();

        // 1) Nobody has asked.
        assert_eq!(doc.seen("gem"), None);

        // 2) Asked, and it cannot list. **Not the same as (1)** — that
        //    is the whole assertion.
        doc.set_seen("gem", &Answer::Cannot).unwrap();
        assert_eq!(doc.seen("gem"), Some(Answer::Cannot));
        assert_ne!(doc.seen("gem"), None, "asked-and-cannot is not never-asked");

        // 3) It listed, and has none. Not the same as (2) either.
        doc.set_seen("empty", &Answer::Sessions { rows: vec![] }).unwrap();
        assert_eq!(doc.seen("empty"), Some(Answer::Sessions { rows: vec![] }));
        assert_ne!(doc.seen("empty"), doc.seen("gem"));

        // 4) It listed, and has some — through the doc, so the
        //    serialization is what is being asserted and not a struct
        //    this test built and read back in memory.
        let rows = vec![
            Seen {
                id: "sess-a".into(),
                cwd: "/w/one".into(),
                title: Some("先看看那个文件".into()),
                updated_at: Some("2026-08-31T04:24:14.876Z".into()),
            },
            // No title, no time: an agent may give neither, and neither
            // may come back as an empty string on screen.
            Seen { id: "sess-b".into(), cwd: String::new(), title: None, updated_at: None },
        ];
        doc.set_seen("cacp", &Answer::Sessions { rows: rows.clone() }).unwrap();
        let Some(Answer::Sessions { rows: back }) = doc.seen("cacp") else {
            panic!("wanted sessions")
        };
        assert_eq!(back, rows);
        assert_eq!(back[1].title, None);
        assert_eq!(back[1].updated_at, None);
    }

    /// **Forgetting an agent forgets what it told us.** A registration
    /// that comes back under the same name is, as far as anything here
    /// knows, a different agent — and offering it the old one's
    /// conversation ids would be offering ids that belong to something
    /// else.
    #[test]
    fn forgetting_an_agent_forgets_its_conversations() {
        let doc = AgentDoc::new(7).unwrap();
        doc.set("gem", &Spec { command: "gemini".into(), args: vec!["--acp".into()] }).unwrap();
        doc.set_seen("gem", &Answer::Sessions {
            rows: vec![Seen {
                id: "old".into(),
                cwd: String::new(),
                title: None,
                updated_at: None,
            }],
        })
        .unwrap();
        doc.remove("gem").unwrap();
        assert_eq!(doc.get("gem"), None);
        assert_eq!(doc.seen("gem"), None, "the conversations went with it");
    }
}
