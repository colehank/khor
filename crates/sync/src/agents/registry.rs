//! The agents khor knows how to start without being told a command.
//!
//! # This is a snapshot, and it says so in every direction
//!
//! Taken from the ACP registry at `31e41d2f8068` on 2026-08-30
//! (<https://github.com/agentclientprotocol/registry>). It is data, not
//! a capability list: an entry here means *khor knows an argv that
//! starts this thing*, and **nothing more**. Whether it runs, whether
//! it needs a login first, and which half of the protocol it actually
//! implements are all answered by the handshake and by nothing here —
//! the registry's own daily probe on the day this was taken had 31 of
//! 32 agents reach `initialize`, 19 of them refusing to open a session
//! until somebody logged in, and exactly **one** implementing
//! `session/stop`.
//!
//! So: this table shortens typing. It does not promise anything, and a
//! face that treated an entry as a promise would be making the claim
//! this comment exists to refuse.
//!
//! # Why only the `npx` ones carry an argv
//!
//! Of the 40 agents in the registry, **22** distribute through
//! `npx`, which is a single argv and no installation — khor can hand
//! that straight to [`crate::agents::Spec`] and the thing starts. The
//! other 18 ship platform binaries or `uvx` packages: an archive
//! per platform, with a checksum, that something has to fetch, verify
//! and unpack. **khor does not download and run binaries**, so those
//! are listed by name only, and pointing khor at one is the same
//! `--` argv that has always worked.
//!
//! Listing them rather than hiding them is the point: "khor does not
//! install this for you" is a different sentence from "khor cannot talk
//! to this", and only the first one is true.
//!
//! # Keeping it fresh
//!
//! Versions here are pinned as the registry pinned them, so this table
//! ages. It ageing is visible (an `npx` pin that no longer resolves
//! fails loudly at spawn) rather than silent, which is the property
//! that made pinning preferable to `@latest`: `@latest` would drift
//! under a user who changed nothing and would make two machines on one
//! mesh run different agents while both reported the same name.

/// One agent the registry knows about.
pub struct Known {
    /// The registry's own id, which is what a person types.
    pub id: &'static str,
    /// The registry's display name.
    pub name: &'static str,
    /// One line, the registry's own words.
    pub about: &'static str,
    /// How to start it, or `None` when khor has no argv for it —
    /// **`None` means "you install it and point khor at it", never
    /// "khor cannot speak to it".**
    pub argv: Option<&'static [&'static str]>,
}

/// Every agent in the snapshot, `npx`-startable ones first, each group
/// by id. Sorted so the list a person reads never reorders itself
/// between builds.
pub const KNOWN: &[Known] = &[
    Known { id: "agoragentic-acp", name: "Agoragentic", about: "Agent marketplace with 174+ AI capabilities. Browse, invoke, and pay for agent services settled in USDC on Base L2.", argv: Some(&["npx", "-y", "agoragentic-mcp@1.3.0", "--acp"]) },
    Known { id: "auggie", name: "Auggie CLI", about: "Augment Code's powerful software agent, backed by industry-leading context engine", argv: Some(&["npx", "-y", "@augmentcode/auggie@0.36.0", "--acp"]) },
    Known { id: "autohand", name: "Autohand Code", about: "Autohand Code - AI coding agent powered by Autohand AI", argv: Some(&["npx", "-y", "@autohandai/autohand-acp@0.2.1"]) },
    Known { id: "claude-acp", name: "Claude Agent", about: "ACP wrapper for Anthropic's Claude", argv: Some(&["npx", "-y", "@agentclientprotocol/claude-agent-acp@0.70.0"]) },
    Known { id: "cline", name: "Cline", about: "Autonomous coding agent CLI - capable of creating/editing files, running commands, using the browser, and more", argv: Some(&["npx", "-y", "cline@3.0.60", "--acp"]) },
    Known { id: "codebuddy-code", name: "Codebuddy Code", about: "Tencent Cloud's official intelligent coding tool", argv: Some(&["npx", "-y", "@tencent-ai/codebuddy-code@2.142.0", "--acp"]) },
    Known { id: "codex-acp", name: "Codex", about: "ACP adapter for OpenAI's coding assistant", argv: Some(&["npx", "-y", "@agentclientprotocol/codex-acp@1.7.0"]) },
    Known { id: "deepagents", name: "DeepAgents", about: "Batteries-included AI coding and general purpose agent powered by LangChain.", argv: Some(&["npx", "-y", "deepagents-acp@0.1.7"]) },
    Known { id: "dimcode", name: "DimCode", about: "A coding agent that puts leading models at your command.", argv: Some(&["npx", "-y", "dimcode@0.3.22", "acp"]) },
    Known { id: "dirac", name: "Dirac", about: "Reduces API costs by more than 50%, produces better and faster work. Uses Hash anchored parallel edits, AST manipulation and a whole lot of neat optimizations. Fully Open Source.", argv: Some(&["npx", "-y", "dirac-cli@0.5.1", "--acp"]) },
    Known { id: "factory-droid", name: "Factory Droid", about: "Factory Droid - AI coding agent powered by Factory AI", argv: Some(&["npx", "-y", "droid@0.208.1", "exec", "--output-format", "acp-daemon"]) },
    Known { id: "gemini", name: "Gemini CLI", about: "Google's official CLI for Gemini", argv: Some(&["npx", "-y", "@google/gemini-cli@0.57.0", "--acp"]) },
    Known { id: "github-copilot", name: "GitHub Copilot", about: "GitHub's AI pair programmer", argv: Some(&["npx", "-y", "@github/copilot-language-server@1.537.0", "--acp"]) },
    Known { id: "github-copilot-cli", name: "GitHub Copilot", about: "GitHub's AI pair programmer", argv: Some(&["npx", "-y", "@github/copilot@1.0.82", "--acp"]) },
    Known { id: "glm-acp-agent", name: "GLM Agent", about: "ACP agent powered by Zhipu AI's GLM Coding Plan models (glm-5.1, glm-5-turbo, glm-4.7, glm-4.5-air). Supports streaming, tool calls, mid-session model switching, image input via Z.AI Coding Plan Vision MCP, and session load/fork/resume with on-disk persistence.", argv: Some(&["npx", "-y", "glm-acp-agent@1.7.0"]) },
    Known { id: "grok-build", name: "Grok Build", about: "xAI's coding agent and CLI", argv: Some(&["npx", "-y", "@xai-official/grok@1.0.13", "agent", "stdio"]) },
    Known { id: "kilo", name: "Kilo", about: "The open source coding agent", argv: Some(&["npx", "-y", "@kilocode/cli@7.5.6", "acp"]) },
    Known { id: "nova", name: "Nova", about: "Nova by Compass AI - a fully-fledged software engineer at your command", argv: Some(&["npx", "-y", "@compass-ai/nova@1.1.37", "acp"]) },
    Known { id: "pi-acp", name: "pi ACP", about: "ACP adapter for pi coding agent", argv: Some(&["npx", "-y", "pi-acp@0.0.33"]) },
    Known { id: "qoder", name: "Qoder CLI", about: "AI coding assistant with agentic capabilities", argv: Some(&["npx", "-y", "@qoder-ai/qodercli@0.2.14", "--acp"]) },
    Known { id: "qwen-code", name: "Qwen Code", about: "Alibaba's Qwen coding assistant", argv: Some(&["npx", "-y", "@qwen-code/qwen-code@0.22.3", "--acp", "--experimental-skills"]) },
    Known { id: "sigit", name: "siGit Code", about: "Local-first coding agent. Runs entirely on your machine with optional on-device LLM inference via Onde.", argv: Some(&["npx", "-y", "@smbcloud/sigit@1.5.2"]) },
    // Binary and `uvx` distributions: named, not startable from here.
    Known { id: "amp-acp", name: "Amp", about: "ACP wrapper for Amp - the frontier coding agent", argv: None },
    Known { id: "antigravity-acp", name: "Google Antigravity", about: "Google’s AI coding agent", argv: None },
    Known { id: "cortex-code", name: "Cortex Code", about: "Snowflake's Cortex Code coding agent", argv: None },
    Known { id: "corust-agent", name: "Corust Agent", about: "Co-building with a seasoned Rust partner.", argv: None },
    Known { id: "crow-cli", name: "crow-cli", about: "Minimal ACP Native Coding Agent", argv: None },
    Known { id: "cursor", name: "Cursor", about: "Cursor's coding agent", argv: None },
    Known { id: "devin", name: "Devin", about: "Devin CLI coding agent by Cognition", argv: None },
    Known { id: "fast-agent", name: "fast-agent", about: "Code and build agents with comprehensive multi-provider support", argv: None },
    Known { id: "goose", name: "goose", about: "A local, extensible, open source AI agent that automates engineering tasks", argv: None },
    Known { id: "harn", name: "Harn", about: "Harn runs .harn agent pipelines as a native ACP coding agent over stdio.", argv: None },
    Known { id: "junie", name: "Junie", about: "AI Coding Agent by JetBrains", argv: None },
    Known { id: "kimi", name: "Kimi CLI", about: "Moonshot AI's coding assistant", argv: None },
    Known { id: "minion-code", name: "Minion Code", about: "An enhanced AI code assistant built on the Minion framework with rich development tools", argv: None },
    Known { id: "mistral-vibe", name: "Mistral Vibe", about: "Mistral's open-source coding assistant", argv: None },
    Known { id: "opencode", name: "OpenCode", about: "The open source coding agent", argv: None },
    Known { id: "poolside", name: "Poolside", about: "Poolside's coding agent", argv: None },
    Known { id: "stakpak", name: "Stakpak", about: "Open-source DevOps agent in Rust with enterprise-grade security", argv: None },
    Known { id: "vtcode", name: "VT Code", about: "An open-source coding agent with LLM-native code understanding and robust shell safety. Supports multiple LLM providers with automatic failover and efficient context management.", argv: None },
];

/// How many of [`KNOWN`] khor can start on its own. Written out rather
/// than counted at a call site so that a shrinking table is a diff
/// somebody reads, not a number that quietly gets smaller.
pub const STARTABLE: usize = 22;

impl Known {
    /// The entry with this id, if the snapshot has one.
    pub fn find(id: &str) -> Option<&'static Known> {
        KNOWN.iter().find(|k| k.id == id)
    }

    /// This entry as a launch spec, when khor has an argv for it.
    pub fn spec(&self) -> Option<crate::agents::Spec> {
        let argv: Vec<String> = self.argv?.iter().map(|s| (*s).to_owned()).collect();
        crate::agents::Spec::from_argv(&argv)
    }
}

#[cfg(test)]
mod tests {
    use super::{Known, KNOWN, STARTABLE};

    /// **Named entries, not「the table has entries」.** The weaker claim
    /// holds for a table of one, and the failure this is written
    /// against is a regeneration that silently drops most of the
    /// registry — which reads as "khor supports fewer agents now" and
    /// as nothing at all in a test suite.
    ///
    /// Three chosen for what each one is: `gemini` is the plainest
    /// `npx` case, `qwen-code` carries two flags rather than one (an
    /// argv split on spaces would lose the second), and `goose` is the
    /// binary case whose whole point is that it is listed *without* a
    /// command.
    #[test]
    fn the_snapshot_still_holds_the_agents_it_was_taken_for() {
        let gemini = Known::find("gemini").expect("gemini");
        assert_eq!(
            gemini.argv,
            Some(&["npx", "-y", "@google/gemini-cli@0.57.0", "--acp"][..])
        );
        let qwen = Known::find("qwen-code").expect("qwen-code");
        assert_eq!(
            qwen.argv,
            Some(&["npx", "-y", "@qwen-code/qwen-code@0.22.3", "--acp", "--experimental-skills"][..])
        );
        let goose = Known::find("goose").expect("goose");
        assert!(
            goose.argv.is_none(),
            "goose ships a binary; an argv here would be one khor invented"
        );
        assert!(Known::find("no-such-agent").is_none());
    }

    /// The count is a written number, so a table that shrinks has to
    /// shrink this too — in a diff somebody reads.
    #[test]
    fn the_startable_count_is_the_startable_entries() {
        let counted = KNOWN.iter().filter(|k| k.argv.is_some()).count();
        assert_eq!(counted, STARTABLE);
        assert_eq!(STARTABLE, 22);
        assert_eq!(KNOWN.len(), 40);
    }

    /// **Every startable entry has to survive the trip into a `Spec`**,
    /// because that is the only thing this table is for. An entry whose
    /// argv is empty, or whose first word is blank, would sit in the
    /// list wearing「khor 起得来」and fail at spawn — which is the one
    /// way this data can lie.
    #[test]
    fn every_startable_entry_really_produces_a_launch() {
        for k in KNOWN {
            match k.argv {
                Some(argv) => {
                    let spec = k.spec().unwrap_or_else(|| panic!("{} has no spec", k.id));
                    assert_eq!(spec.command, argv[0], "{}", k.id);
                    assert_eq!(spec.args.len(), argv.len() - 1, "{}", k.id);
                    // The launch JSON is what actually spawns the child.
                    assert!(spec.launch().contains("\"command\""), "{}", k.id);
                }
                None => assert!(k.spec().is_none(), "{}", k.id),
            }
            assert!(!k.id.is_empty() && !k.name.is_empty(), "{}", k.id);
        }
    }

    /// Ids are what a person types after `@`, so two entries sharing one
    /// would make `find` answer with whichever came first — silently.
    #[test]
    fn the_ids_are_unique() {
        let mut ids: Vec<&str> = KNOWN.iter().map(|k| k.id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }
}
