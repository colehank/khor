//! **One channel** (STORE-BRIEF phase 5): a message to the broadcast is
//! text, plus the machines it is pushed to, plus files — composed once,
//! resolved once, and never re-read against the table later.
//!
//! **The typed form.** Leading `@machine` or `@machine:path` names a
//! target (no path: that machine's receive location); after the targets,
//! `machine:path` names a file on that machine (this machine's own name
//! is a file here); the rest is the words. The same string from the
//! composer, the CLI's `tell` / `send` and a script: one parser, here.
//! A path with spaces is written in double quotes (`@turing:"~/my dir"`).
//!
//! **Resolved when composed** (`Node::post`): names become device ids, a
//! file on another machine is offered by that machine now (its digest
//! and size, `proto::Request::Offer`), every target is dialled now. A
//! target that does not answer refuses the whole message, named, and
//! nothing is written — no store-and-forward: the person decides
//! whether to wait or send elsewhere. Then one message is written to the
//! broadcast, and each target is **pushed to**: the broadcast reaches it
//! at once and it is asked to take the files (`Request::Act` `take`), so
//! the bytes land in the named directory with nobody pressing anything.
//! The target pulls each file from its holder — the sender, or the
//! machine the file is on — never through the sender, and writes how
//! that stands under its own key on the message (`ChatDoc::set_fate_of`).
//! Every other machine reads the message as an offer, as before.
//!
//! **Pushes in flight are counted** ([`pushes_in_flight`]) for the phone,
//! which keeps a background task open while any runs and, when the
//! system's grace ends first, tells each target its push was cut
//! ([`Node::cut_pushes`], the target's fate 「发送中断：手机已挂起」).

use std::path::PathBuf;
use std::sync::Mutex;

use khor_catalog::msg;
use khor_sync::chat::{FileRef, Target};
use khor_sync::devices::DeviceInfo;

use crate::proto::{Request, Response};
use crate::transfer::TransferKind;
use crate::wide::BROADCAST;
use crate::{Node, NodeEvent};

/// What a typed message says, read against the device table.
#[derive(Debug, Clone, Default)]
pub struct Draft {
    /// The machines it is pushed to, each with its directory there
    /// (empty: its receive location), in the order named, once each.
    pub to: Vec<(DeviceInfo, String)>,
    /// Files on other machines: the holder and the path on it.
    pub far: Vec<(DeviceInfo, String)>,
    /// Files typed as this machine's own (`<this machine>:/path`).
    pub here: Vec<String>,
    /// The words.
    pub text: String,
}

/// The first whitespace-separated token of `s` with its double quotes
/// taken off, and what follows it; `None` when only whitespace is left.
fn lead(s: &str) -> Option<(String, &str)> {
    let t = s.trim_start();
    if t.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut quoted = false;
    let mut end = t.len();
    for (i, c) in t.char_indices() {
        if c == '"' {
            quoted = !quoted;
            continue;
        }
        if c.is_whitespace() && !quoted {
            end = i;
            break;
        }
        out.push(c);
    }
    Some((out, &t[end..]))
}

/// The machine a person typed: its shown name (a rename), then its name.
pub fn machine_named<'a>(machines: &'a [DeviceInfo], typed: &str) -> Option<&'a DeviceInfo> {
    machines
        .iter()
        .find(|d| d.label.as_deref() == Some(typed))
        .or_else(|| machines.iter().find(|d| d.name == typed))
}

/// How a machine is written in the typed form: its shown name when that
/// is one token, its name otherwise — what the picker inserts.
pub fn typed_name(d: &DeviceInfo) -> &str {
    let shown = d.shown();
    if shown.is_empty() || shown.chars().any(|c| c.is_whitespace() || c == ':' || c == '"') {
        &d.name
    } else {
        shown
    }
}

/// The typed form of words for one machine — what `khor tell <machine>`
/// and the MCP `tell` mean now: the words, pushed to that machine. An
/// empty machine or the broadcast's own name is nobody in particular.
pub fn addressed(machine: &str, text: &str) -> String {
    let m = machine.trim();
    if m.is_empty() || m == BROADCAST {
        return text.to_owned();
    }
    if m.chars().any(char::is_whitespace) {
        format!("@\"{m}\" {text}")
    } else {
        format!("@{m} {text}")
    }
}

/// Reads the typed form (module head) against `machines`; `me` is this
/// machine's device id. An `@` naming no machine refuses with the names
/// there are; a `name:path` naming no machine is words.
pub fn parse(typed: &str, machines: &[DeviceInfo], me: &str) -> Result<Draft, String> {
    let mut d = Draft::default();
    let mut rest = typed;
    while let Some((tok, after)) = lead(rest) {
        let Some(t) = tok.strip_prefix('@').filter(|t| !t.is_empty()) else {
            break;
        };
        let (name, path) = t.split_once(':').unwrap_or((t, ""));
        let m = machine_named(machines, name).ok_or_else(|| {
            let names: Vec<&str> = machines.iter().map(|d| d.shown()).collect();
            msg::no_such_machine(name, names.join(khor_catalog::cli::name_separator()))
        })?;
        let path = crate::road::tidy_setting(path)?;
        if !d.to.iter().any(|(t, _)| t.id == m.id) {
            d.to.push((m.clone(), path));
        }
        rest = after;
    }
    while let Some((tok, after)) = lead(rest) {
        let Some((name, path)) = tok.split_once(':').filter(|(n, p)| !n.is_empty() && !p.is_empty()) else {
            break;
        };
        let Some(m) = machine_named(machines, name) else {
            break;
        };
        if m.id == me {
            d.here.push(path.to_owned());
        } else {
            d.far.push((m.clone(), path.to_owned()));
        }
        rest = after;
    }
    d.text = rest.trim().to_owned();
    Ok(d)
}

// ── pushes in flight ───────────────────────────────────────

/// One push running: the message and the target, by id.
#[derive(Clone)]
struct InFlight {
    seq: u64,
    msg: String,
    to: DeviceInfo,
}

fn flights() -> &'static Mutex<Vec<InFlight>> {
    static ALL: Mutex<Vec<InFlight>> = Mutex::new(Vec::new());
    &ALL
}

/// Held while a push runs; its drop takes it off the count.
struct Flying(u64);

impl Flying {
    fn begin(msg: &str, to: &DeviceInfo) -> Flying {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        flights().lock().unwrap_or_else(|p| p.into_inner()).push(InFlight {
            seq,
            msg: msg.to_owned(),
            to: to.clone(),
        });
        Flying(seq)
    }
}

impl Drop for Flying {
    fn drop(&mut self) {
        flights().lock().unwrap_or_else(|p| p.into_inner()).retain(|f| f.seq != self.0);
    }
}

/// How many pushes this process is running now — what the phone keeps a
/// background task open for (module head).
pub fn pushes_in_flight() -> usize {
    flights().lock().unwrap_or_else(|p| p.into_inner()).len()
}

impl Node {
    /// Console prose does not parse addressing or command syntax. CLI
    /// tell/send retain their explicit argument grammar through post.
    pub async fn note(&self, text: &str, files: &[PathBuf]) -> Result<String, String> {
        if let Some(reply) = self.via_serve(crate::ipc::Op::Note {
            text: text.to_owned(), files: files.iter().map(|p| p.display().to_string()).collect(),
        }).await {
            return match reply? {
                crate::ipc::Reply::Posted { id } => Ok(id),
                crate::ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        self.note_here(text, files)
    }

    pub(crate) fn note_here(&self, text: &str, files: &[PathBuf]) -> Result<String, String> {
        if text.trim().is_empty() && files.is_empty() { return Err(msg::post_nothing().to_owned()); }
        let refs = files.iter().map(|p| self.offer_file(p)).collect::<Result<Vec<_>, _>>()?;
        let told = self.chat.post(BROADCAST, self.device(), text, &[], &refs)?;
        if !text.is_empty() { let _ = self.seen_snapshot(&told.row.id, &[crate::SeenMessage { id: told.msg_id.clone(), at_ms: told.at }]); }
        self.emit(NodeEvent::Event(told.event));
        let _ = self.emit_row_of(&told.row.id);
        Ok(told.msg_id)
    }

    /// **Composes a message into the broadcast** (module head): `typed`
    /// is the typed form, `files` this machine's files to go with it.
    /// Through the resident serve when one holds the key — it holds the
    /// endpoint the targets pull from, and it pushes on its own time
    /// while this returns; otherwise on an endpoint of its own, waiting
    /// for every push to end. Answers the message id; refused before
    /// anything is written when a target or a holder does not answer.
    pub async fn post(&self, typed: &str, files: &[PathBuf]) -> Result<String, String> {
        let files: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
        if let Some(reply) = self
            .via_serve(crate::ipc::Op::Post {
                typed: typed.to_owned(),
                files: files.clone(),
            })
            .await
        {
            return match reply? {
                crate::ipc::Reply::Posted { id } => Ok(id),
                crate::ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = khor_net::endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
        let out = self.post_with(&ep, typed, &files, true).await;
        ep.close().await;
        out
    }

    /// [`Self::post`] on an endpoint the caller holds. `wait`: push to
    /// every target before answering (a one-shot caller whose endpoint
    /// ends with it); otherwise the pushes run on the resident's own
    /// tasks.
    pub(crate) async fn post_with(
        &self,
        ep: &iroh::Endpoint,
        typed: &str,
        files: &[PathBuf],
        wait: bool,
    ) -> Result<String, String> {
        let me = self.device_str().to_owned();
        let machines = self.devices()?;
        let draft = parse(typed, &machines, &me)?;
        let mut here: Vec<PathBuf> = files.to_vec();
        here.extend(draft.here.iter().map(|p| crate::road::expand(self.root(), p)));
        if draft.text.is_empty() && here.is_empty() && draft.far.is_empty() {
            return Err(msg::post_nothing().to_owned());
        }
        for (d, _) in &draft.to {
            if d.id == me {
                return Err(msg::post_target_self(d.shown()));
            }
            if d.sponsor.is_some() {
                return Err(msg::post_target_no_khor(d.shown()));
            }
        }
        // **Every target answers now, or nothing is sent** (module head).
        // A round, not a dial: a wire held from before answers a dial
        // for a machine that has since gone, and only an exchange finds
        // out — and the round brings the target up to date besides.
        let dials = draft
            .to
            .iter()
            .map(|(d, _)| async move { (d, self.round_with(ep, d, true).await.is_ok()) })
            .collect();
        let down: Vec<&str> = crate::link::join_all(dials)
            .await
            .into_iter()
            .filter(|(_, up)| !up)
            .map(|(d, _)| d.shown())
            .collect();
        if !down.is_empty() {
            return Err(msg::post_target_down(down.join(khor_catalog::cli::name_separator())));
        }
        // A file on another machine is offered by it now: what the
        // message stores is its digest, and it is pulled from there.
        let mut refs: Vec<FileRef> = Vec::new();
        for (h, path) in &draft.far {
            let asked = self.ask_on(ep, h, &Request::Offer { path: path.clone() }).await;
            match asked {
                Ok(Response::Offered { name, size, digest }) => refs.push(FileRef {
                    name,
                    size: size as i64,
                    digest,
                    holder: h.id.clone(),
                }),
                Ok(Response::Refused { why }) => return Err(msg::post_holder_refused(h.shown(), path, why)),
                Ok(other) => return Err(msg::peer_non_answer(format_args!("{other:?}"))),
                Err(_) => return Err(msg::post_holder_down(h.shown(), path)),
            }
        }
        for p in &here {
            refs.push(self.offer_file(p)?);
        }
        let to: Vec<Target> = draft
            .to
            .iter()
            .map(|(d, path)| Target {
                device: d.id.clone(),
                path: path.clone(),
            })
            .collect();
        let told = self.chat.post(BROADCAST, self.device(), &draft.text, &to, &refs)?;
        // Words typed here were read here (`seen`'s rule: own words are
        // unread nowhere). A file is not: it has arrived nowhere yet.
        if !draft.text.is_empty() {
            self.mark_seen(&told.row.id, told.at)?;
        }
        self.emit(NodeEvent::Event(told.event));
        let _ = self.emit_row_of(&told.row.id);
        let id = told.msg_id;
        let carries = !refs.is_empty();
        // Counted from here, before any task runs: a phone that leaves
        // the screen the moment this answers still sees them.
        let flying: Vec<(DeviceInfo, Flying)> =
            draft.to.iter().map(|(d, _)| (d.clone(), Flying::begin(&id, d))).collect();
        if wait {
            let id = id.as_str();
            let pushes = flying.into_iter().map(|(d, f)| async move { self.push_to(ep, &d, id, carries, f).await }).collect();
            crate::link::join_all(pushes).await;
        } else if let Some(n) = self.me() {
            for (d, f) in flying {
                let (n, ep, id) = (n.clone(), ep.clone(), id.clone());
                tokio::spawn(async move { n.push_to(&ep, &d, &id, carries, f).await });
            }
        }
        // Everyone else reads it at once, not at the next beat.
        self.nudge(ep, true);
        Ok(id)
    }

    /// **One push** (module head): the broadcast reaches the target now,
    /// then it is asked to take the files; its own record of the move
    /// comes back with a last round. Counted while it runs.
    async fn push_to(&self, ep: &iroh::Endpoint, d: &DeviceInfo, msg_id: &str, files: bool, _flying: Flying) {
        let _ = self.round_with(ep, d, true).await;
        if !files {
            return;
        }
        let take = Request::Act {
            session: TransferKind::session_id(msg_id).0,
            action: "take".to_owned(),
        };
        match self.ask_on(ep, d, &take).await {
            Ok(Response::Acted { .. }) => {}
            Ok(Response::Refused { why }) => eprintln!("{why}"),
            Ok(other) => eprintln!("{}", msg::peer_non_answer(format_args!("{other:?}"))),
            Err(why) => eprintln!("{why}"),
        }
        let _ = self.round_with(ep, d, true).await;
    }

    /// **The phone's grace is over** (module head): every target of a
    /// push still running here is told it was cut, and ends its pull with
    /// that sentence as its fate. Bounded: the system is about to stop
    /// this process.
    pub(crate) async fn cut_pushes_on(&self, ep: &iroh::Endpoint) {
        let flying: Vec<InFlight> = flights().lock().unwrap_or_else(|p| p.into_inner()).clone();
        let cuts = flying
            .iter()
            .map(|f| async move {
                let cut = Request::Act {
                    session: TransferKind::session_id(&f.msg).0,
                    action: "cut".to_owned(),
                };
                let _ = tokio::time::timeout(std::time::Duration::from_secs(3), self.ask_on(ep, &f.to, &cut)).await;
            })
            .collect();
        crate::link::join_all(cuts).await;
    }

    /// [`Request::Offer`] on this machine: the file digested and its
    /// offer record kept, so whoever is told its digest pulls it from here.
    pub(crate) fn offer_here(&self, path: &str) -> Result<Response, String> {
        let p = crate::road::expand(self.root(), path);
        if !p.is_file() {
            return Err(msg::not_a_file_path(p.display()));
        }
        let f = self.offer_file(&p)?;
        Ok(Response::Offered {
            name: f.name,
            size: f.size.max(0) as u64,
            digest: f.digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(id: &str, name: &str, label: Option<&str>) -> DeviceInfo {
        DeviceInfo {
            id: id.into(),
            name: name.into(),
            addrs: Vec::new(),
            style: None,
            pinned: false,
            sponsor: None,
            ssh_host: None,
            ssh_jump: None,
            ssh_port: None,
            ssh_password: false,
            ssh_identity: None,
            ssh_passphrase: false,
            sat_tmux: None,
            carrier: None,
            watch: Vec::new(),
            label: label.map(str::to_owned),
            hosts: true,
            no_relay: false,
            roads: Vec::new(),
            receive_dir: None,
            receive_at: None,
            receive_for: None,
            receive_why: None,
        }
    }

    fn table() -> Vec<DeviceInfo> {
        vec![
            machine("me", "mac", None),
            machine("t", "turing", None),
            machine("h", "hinton", Some("北京")),
        ]
    }

    /// **The typed form** (module head): targets first, each with its own
    /// path; then files on machines; then words. A renamed machine is
    /// typed by its new name; this machine's own `name:path` is a file
    /// here; a `word:` that names no machine is words.
    #[test]
    fn the_typed_form_reads_targets_then_files_then_words() {
        let t = table();
        let d = parse("@turing:~/data @北京 hinton:/data/x.bin mac:~/y look: at these", &t, "me").unwrap();
        let to: Vec<(&str, &str)> = d.to.iter().map(|(m, p)| (m.id.as_str(), p.as_str())).collect();
        assert_eq!(to, [("t", "~/data"), ("h", "")]);
        let far: Vec<(&str, &str)> = d.far.iter().map(|(m, p)| (m.id.as_str(), p.as_str())).collect();
        assert_eq!(far, [("h", "/data/x.bin")]);
        assert_eq!(d.here, ["~/y"]);
        assert_eq!(d.text, "look: at these");

        let d = parse("@turing:\"~/my dir\"   hello\nthere", &t, "me").unwrap();
        assert_eq!((d.to[0].1.as_str(), d.text.as_str()), ("~/my dir", "hello\nthere"), "quotes hold a space; words keep their lines");
        let d = parse("@turing @turing hi", &t, "me").unwrap();
        assert_eq!(d.to.len(), 1, "a target named twice is pushed to once");
        let d = parse("mail me@turing tomorrow", &t, "me").unwrap();
        assert!(d.to.is_empty() && d.text == "mail me@turing tomorrow", "an @ inside the words is words");
        let d = parse(&addressed("北京", "hi there"), &t, "me").unwrap();
        assert_eq!((d.to[0].0.id.as_str(), d.text.as_str()), ("h", "hi there"), "`khor tell` is a push");
        assert_eq!(addressed("wide/all", "hi"), "hi");
        let d = parse("@ alone", &t, "me").unwrap();
        assert_eq!((d.to.len(), d.text.as_str()), (0, "@ alone"));
    }

    #[test]
    fn an_unknown_target_or_a_relative_path_refuses_by_name() {
        let t = table();
        let e = parse("@lecun hi", &t, "me").unwrap_err();
        assert!(e.contains("lecun") && e.contains("turing"), "{e}");
        assert!(parse("@turing:data hi", &t, "me").is_err(), "a path is absolute or ~-led");
        assert!(parse("@turing:~/../x hi", &t, "me").is_err());
    }

    #[test]
    fn the_picker_writes_the_shown_name_when_it_is_one_token() {
        assert_eq!(typed_name(&machine("h", "hinton", Some("北京"))), "北京");
        assert_eq!(typed_name(&machine("h", "hinton", Some("北京 机房"))), "hinton");
        assert_eq!(typed_name(&machine("t", "turing", None)), "turing");
    }
}
