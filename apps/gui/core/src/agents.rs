//! The two things about an ACP agent that only the command line could
//! reach: **the shipped registry snapshot**, and **what one agent said
//! it already has**.
//!
//! # The three answers, and why they are an enum on the wire
//!
//! Asking an agent what conversations it has produces four different
//! situations, and the person's next move differs in all four: nobody
//! ever asked (go ask), it answered that it cannot list (stop asking),
//! it answered with nothing (open a new one), it answered with a list
//! (pick one). The node keeps them apart already — `Option` around
//! [`khor_node::AgentAnswer`] — and the whole risk of this module is
//! flattening them on the way out, because **an empty list looks
//! exactly like a question never put**.
//!
//! So the wire shape is [`AgentSaid`], a tagged union whose `rows`
//! field **exists on one variant only**. That is deliberate and it is
//! the strongest guard available here: a face that reached for
//! `rows.length === 0` to mean "it has none" does not compile, because
//! there is no `rows` on the other two. A struct with a nullable list
//! would have satisfied every test in this file and lost the
//! distinction on the first face that read it.
//!
//! # Asking costs a process; remembering costs nothing
//!
//! [`agent_ask`] starts the agent, asks, and stops it — **measured 1.5s
//! against a real `@claude-acp` with its npm package already cached,
//! and 5.64s without it**. [`agent_seen`] reads what was recorded —
//! 0.017s, same agent, same answer. Two orders of magnitude at best,
//! and the gap is a whole process either way. So the
//! read is what a face does when a person picks an agent, and the ask
//! is a thing a person presses. A poll that asked would start an agent
//! process on a timer, which is the one thing this feature must never
//! do; `khor_node::refresh_agent_sessions` says the same in its own
//! words, and this module is the face-side half of that judgment.

use serde::Serialize;
use ts_rs::TS;

use khor_node::Node;

/// One entry of the shipped ACP registry snapshot
/// (`khor_sync::agents::registry`).
///
/// **`argv` is for reading, never for sending back.** Registering goes
/// by [`register_known`] with the id, and the command is looked up
/// again on this side — a face that posted the argv it was shown would
/// be a face that can ask khor to spawn anything at all. It is carried
/// at all because "khor 起得来" without the command is a claim a person
/// has to take on trust, which is the same reason `khor agents known`
/// prints it.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct KnownAgent {
    /// The registry's own id — what identifies this entry everywhere.
    pub id: String,
    pub name: String,
    pub about: String,
    /// How khor would start it, or `None` when it ships a binary.
    /// **`None` means "you install it and point khor at it", never
    /// "khor cannot speak to it"** — the two read differently on
    /// screen for that reason.
    pub argv: Option<String>,
}

/// One conversation an agent said it has.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct SeenSession {
    /// The agent's own id for it, which is what resumes it.
    pub id: String,
    /// Where it was opened. Carried to the screen because a resume
    /// lands there rather than in whatever the wizard's 目录 box says,
    /// and a person picking a conversation should see where it goes.
    pub cwd: String,
    /// Absent stays absent: a conversation nobody named must not come
    /// back as an empty string on screen.
    pub title: Option<String>,
    pub updated_at: Option<String>,
}

/// What khor knows about one agent's past conversations.
///
/// See the module head: the variants are the point, and `rows` living
/// on exactly one of them is what stops a face from collapsing
/// 「它说它没有」 into 「还没问过」.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(tag = "said", rename_all = "snake_case")]
pub enum AgentSaid {
    /// Nobody has ever put the question to this agent.
    Unasked,
    /// It was asked and answered that it cannot list — `session/list`
    /// is optional in the protocol (20 of 32 registry agents
    /// implemented it on 2026-08-30). **A recorded refusal, not a
    /// missing answer**: it is why the next person is not sent to
    /// start the same process for the same nothing.
    Cannot,
    /// It answered. Possibly with nothing, and here that really is
    /// nothing.
    Sessions { rows: Vec<SeenSession> },
}

impl From<Option<khor_node::AgentAnswer>> for AgentSaid {
    fn from(a: Option<khor_node::AgentAnswer>) -> Self {
        match a {
            None => AgentSaid::Unasked,
            Some(khor_node::AgentAnswer::Cannot) => AgentSaid::Cannot,
            Some(khor_node::AgentAnswer::Sessions { rows }) => AgentSaid::Sessions {
                rows: rows
                    .into_iter()
                    .map(|r| SeenSession {
                        id: r.id,
                        cwd: r.cwd,
                        title: r.title,
                        updated_at: r.updated_at,
                    })
                    .collect(),
            },
        }
    }
}

/// The whole shipped snapshot, in the table's own order.
///
/// Everything, not just the startable ones. Listing the other 18 is
/// the point — "khor does not install this for you" and "khor cannot
/// talk to this" are different sentences and only the first is true
/// (`khor_sync::agents::registry`'s head).
pub fn agents_known() -> Vec<KnownAgent> {
    khor_node::agent_registry::KNOWN
        .iter()
        .map(|k| KnownAgent {
            id: k.id.to_owned(),
            name: k.name.to_owned(),
            about: k.about.to_owned(),
            argv: k.argv.map(|a| a.join(" ")),
        })
        .collect()
}

/// When the snapshot was taken, and how big it is: `(date, all,
/// startable)`.
///
/// **A table that ages has to say when it is from.** Without the date
/// on screen the list reads as "these are the agents there are", which
/// is a claim khor cannot make about a file pinned on one day.
pub fn agents_known_taken() -> (String, usize, usize) {
    (
        khor_node::agent_registry::TAKEN.to_owned(),
        khor_node::agent_registry::KNOWN.len(),
        khor_node::agent_registry::STARTABLE,
    )
}

/// Registers a snapshot entry under its own id as the name.
///
/// **The id is the name, and nobody is asked to invent one.** The name
/// becomes the row's `khor_core::Session::category`, so two machines
/// that each attach the same agent have to arrive at the same word —
/// and the registry id is the one word both of them already have. A
/// person who wants their own name for it still has `khor agents add`.
pub fn register_known(root: &std::path::Path, id: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.register_known_agent(id, id)
}

/// What was recorded, without starting anything. **0.017s measured.**
pub fn agent_seen(root: &std::path::Path, name: &str) -> Result<AgentSaid, String> {
    Ok(Node::open(root.to_path_buf())?.agent_seen(name)?.into())
}

/// Asks the agent itself, and records the answer. **Starts a real
/// process — measured 1.5s warm, 5.64s cold.** Only ever behind
/// something a person did.
pub fn agent_ask(root: &std::path::Path, name: &str) -> Result<AgentSaid, String> {
    Ok(Some(Node::open(root.to_path_buf())?.refresh_agent_sessions(name)?).into())
}

/// Opens a conversation the agent already has, by the id it gave.
///
/// **A different act from `open_session`, and it takes different
/// answers.** The wizard's four questions are for a conversation that
/// does not exist yet; this one exists, so its directory and its name
/// are already decided — by it. Taking the 目录 box instead would hand
/// the agent another conversation's address, and `session/load` names a
/// cwd that a real adaptor is entitled to refuse
/// (`khor_acp::Handle`).
///
/// **The id must be one khor recorded.** An id that was never listed
/// has no address to open at, so it is refused by name rather than
/// opened at some default — the failure mode being avoided is a fresh
/// conversation that gets called the old one.
pub fn resume_session(
    root: &std::path::Path,
    agent: &str,
    session: &str,
) -> Result<String, String> {
    let n = Node::open(root.to_path_buf())?;
    // The same refusal the CLI and the wizard give (`agent_or_refuse`),
    // for the same reason: one judgment, or it grows two.
    let spec = n.agent_or_refuse(agent)?;
    let seen = n
        .agent_seen_row(agent, session)?
        .ok_or_else(|| khor_catalog::msg::resume_unknown_session(session, agent))?;
    // Its own name, or where it lives — an id is not something a
    // person recognises in a list of rows (`khor agents sessions`
    // prints the same two, in the same order, for the same reason).
    let title = seen.title.clone().unwrap_or_else(|| {
        std::path::Path::new(&seen.cwd)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| agent.to_owned())
    });
    let id = n.reopen_gui_as(
        std::path::Path::new(&seen.cwd),
        &title,
        &[Node::agent_launch(&spec)?],
        agent,
        session,
    )?;
    Ok(id.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch tree that **takes itself away however the test leaves**
    /// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
    /// cleanup written at the end of a test is skipped by the assertion
    /// that fails, so a red run leaks a directory every time it is red).
    /// A copy per file, which is this repo's convention for test helpers —
    /// it costs no dev-dependency.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn root(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-guiagents-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// **The four answers stay four on the way out.** This is the
    /// flattening this module exists to prevent, and the assertion is
    /// on the *serialised* shape rather than on the enum, because the
    /// enum is not what a face reads.
    ///
    /// Asserted as JSON containing `"said"` per case and — the load
    /// bearing half — that `rows` appears **only** on the listing one.
    /// A shape that always carried `rows` would satisfy every
    /// per-variant check here and lose the distinction at the first
    /// face that read it.
    #[test]
    fn an_empty_listing_and_a_question_never_asked_do_not_serialise_alike() {
        let unasked = serde_json::to_string(&AgentSaid::from(None)).unwrap();
        let cannot =
            serde_json::to_string(&AgentSaid::from(Some(khor_node::AgentAnswer::Cannot))).unwrap();
        let none =
            serde_json::to_string(&AgentSaid::from(Some(khor_node::AgentAnswer::Sessions {
                rows: vec![],
            })))
            .unwrap();
        let some =
            serde_json::to_string(&AgentSaid::from(Some(khor_node::AgentAnswer::Sessions {
                rows: vec![khor_node::AgentSeen {
                    id: "sess-1".into(),
                    cwd: "/w/khor".into(),
                    title: Some("接上注册表".into()),
                    updated_at: Some("2026-08-30T10:00:00Z".into()),
                }],
            })))
            .unwrap();

        assert!(unasked.contains(r#""said":"unasked""#), "{unasked}");
        assert!(cannot.contains(r#""said":"cannot""#), "{cannot}");
        assert!(none.contains(r#""said":"sessions""#), "{none}");
        assert!(some.contains(r#""said":"sessions""#), "{some}");

        // The three that are not a listing must be four distinct
        // sentences on the wire, or the face has nothing to tell them
        // apart by.
        assert_ne!(unasked, cannot);
        assert_ne!(unasked, none);
        assert_ne!(cannot, none);
        assert_ne!(none, some);

        // **`rows` exists on the listing and nowhere else.** This is
        // the whole design: a face cannot reach for a list on an
        // answer that is not one.
        assert!(
            !unasked.contains("rows"),
            "unasked carried a list: {unasked}"
        );
        assert!(!cannot.contains("rows"), "cannot carried a list: {cannot}");
        assert!(none.contains(r#""rows":[]"#), "{none}");
        assert!(
            some.contains("sess-1") && some.contains("/w/khor"),
            "{some}"
        );
    }

    /// **A conversation's cwd survives the trip to the face.** It is
    /// what a resume is opened at, and it is printed on the row so a
    /// person can see where the click lands — dropping it here would
    /// leave both halves silently guessing.
    #[test]
    fn a_recorded_conversation_carries_where_it_lives() {
        let said = AgentSaid::from(Some(khor_node::AgentAnswer::Sessions {
            rows: vec![khor_node::AgentSeen {
                id: "s".into(),
                cwd: "/w/khor".into(),
                title: None,
                updated_at: None,
            }],
        }));
        let AgentSaid::Sessions { rows } = said else {
            panic!("a listing")
        };
        assert_eq!(rows[0].cwd, "/w/khor");
        // A conversation nobody named stays unnamed rather than
        // becoming an empty string on screen.
        assert_eq!(rows[0].title, None);
    }

    /// The snapshot reaches the face whole, both halves of it — and the
    /// counts are the table's own written ones, so a snapshot that
    /// shrinks fails here rather than reading as "khor supports fewer
    /// agents now".
    ///
    /// **The 18 without a command are asserted present**, because
    /// listing them is the point: a face that showed only the startable
    /// ones would be telling a person that goose cannot be reached.
    #[test]
    fn the_whole_snapshot_reaches_the_face_including_the_ones_it_cannot_start() {
        let rows = agents_known();
        let (taken, all, startable) = agents_known_taken();
        assert_eq!(rows.len(), all);
        assert_eq!(rows.iter().filter(|r| r.argv.is_some()).count(), startable);
        assert_eq!(rows.len() - startable, 18);
        assert!(
            taken.starts_with("2026-"),
            "the snapshot says when: {taken}"
        );

        let gemini = rows.iter().find(|r| r.id == "gemini").expect("gemini");
        assert_eq!(
            gemini.argv.as_deref(),
            Some("npx -y @google/gemini-cli@0.57.0 --acp"),
            "the command is spelled out, or「khor 起得来」is taken on trust"
        );
        let goose = rows.iter().find(|r| r.id == "goose").expect("goose");
        assert_eq!(
            goose.argv, None,
            "goose ships a binary; an argv here would be invented"
        );
    }

    /// **Registering by id spawns nothing and names the agent by its
    /// id**, which is the word two machines will both arrive at.
    #[test]
    fn attaching_a_snapshot_entry_registers_it_under_its_own_id() {
        let r = root("attach");
        register_known(&r, "gemini").unwrap();
        let n = Node::open(r.to_path_buf()).unwrap();
        let spec = n.agent("gemini").expect("readable").expect("registered");
        assert!(spec.typed().contains("gemini-cli"), "{}", spec.typed());
        // And nothing was asked: attaching is not asking, and the
        // difference is a process.
        assert!(
            matches!(agent_seen(&r, "gemini").unwrap(), AgentSaid::Unasked),
            "registering must not have put the question"
        );
    }

    /// **An id khor never recorded is refused, not opened.** Nothing
    /// is spawned here, which is the point: the alternative to this
    /// refusal is starting the agent at some default directory and
    /// calling whatever comes back the conversation that was asked
    /// for.
    ///
    /// The two refusals a resume can hit are asserted to differ — an
    /// unregistered agent and an unrecorded conversation are different
    /// situations with different next steps (登记它 / 问它一次), and
    /// `is_err()` cannot see the difference.
    #[test]
    fn resuming_a_conversation_nobody_recorded_is_refused_by_name() {
        let r = root("resume");
        let n = Node::open(r.to_path_buf()).unwrap();
        n.register_agent("zed", &["some-acp-agent".to_owned()])
            .unwrap();

        let unrecorded =
            resume_session(&r, "zed", "sess-nobody-listed").expect_err("never recorded");
        assert!(unrecorded.contains("sess-nobody-listed"), "{unrecorded}");
        assert!(
            unrecorded.contains("zed"),
            "and it says who to ask: {unrecorded}"
        );

        let unregistered =
            resume_session(&r, "zedd", "sess-nobody-listed").expect_err("no such registration");
        assert_eq!(
            unregistered,
            n.agent_or_refuse("zedd").unwrap_err(),
            "one judgment for「没登记过这个」, shared with the CLI"
        );
        assert_ne!(
            unrecorded, unregistered,
            "「没登记过这个 agent」and「没问到过这段对话」are different situations"
        );
    }

    /// The two refusals `@id` can hit are different sentences — and
    /// **the second is the one a copy gets wrong**, because "khor has
    /// no command for this real agent" reads as "no such agent" unless
    /// somebody kept them apart.
    #[test]
    fn the_two_ways_a_snapshot_id_can_fail_say_different_things() {
        let r = root("refuse");
        let missing = register_known(&r, "no-such-agent").expect_err("not in the snapshot");
        let binary = register_known(&r, "goose").expect_err("ships a binary");
        assert_ne!(missing, binary);
        assert!(missing.contains("no-such-agent"), "{missing}");
        assert!(binary.contains("goose"), "{binary}");
    }
}
