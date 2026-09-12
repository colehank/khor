//! A workspace's VS Code, shaped for the screen (工作区批5).
//!
//! Every judgment — whether a machine has an editor, whether one is
//! already running, how the pipe in front of it is held — is made in
//! the node (`khor_node::code`, `Node::code_open`). This layer renames
//! fields for TS and decides nothing.
//!
//! **The consent is not here either, and that is deliberate.** Whether
//! a person has agreed to fetch 640 MB from Microsoft and accept their
//! server licence is a fact about that person, remembered by the face
//! that asked (`views/WorkspacePane`). A data layer that carried it
//! would be a second place for the answer to live, and the one that
//! forgot would be the one that launched.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// Where a machine's VS Code can be reached from **this** machine.
#[derive(Debug, Clone, Serialize, TS)]
pub struct CodeAt {
    /// A full URL, token and folder included. **Ready to point a
    /// webview at** — the parts are not handed out separately because
    /// there is nothing a caller could do with them that this has not
    /// already done, and two places building one URL is two places to
    /// get the escaping wrong.
    pub url: String,
}

/// `code --version`'s first line on that machine, or empty when there
/// is none (`khor_node::Node::code_here`).
///
/// **A refusal comes back as empty, not as an error.** A machine that
/// cannot answer and a machine with no editor lead to the same next
/// move — draw no button — and the node's own asker already folds the
/// two (`proto::Request::CodeHere`).
pub async fn here(root: &Path, machine: &str) -> Result<String, String> {
    Ok(Node::open(root.to_path_buf())?.code_here(machine).await.unwrap_or_default())
}

/// Starts (or reuses) that machine's web face and answers the URL to
/// point at it, with `dir` as the folder to open.
///
/// **The first call on a machine can take minutes**, because a machine
/// that has never run it fetches its server first. The face must have
/// said so before calling this; there is no way to ask from here.
pub async fn open(root: &Path, machine: &str, dir: &str) -> Result<CodeAt, String> {
    let (addr, token) = Node::open(root.to_path_buf())?.code_open(machine).await?;
    let folder = if dir.is_empty() { String::new() } else { format!("&folder={}", enc(dir)) };
    Ok(CodeAt { url: format!("http://{addr}/?tkn={token}{folder}") })
}

/// Stops that machine's web face and drops this side's pipe.
pub async fn stop(root: &Path, machine: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.code_stop(machine).await
}

/// Percent-encodes a path for a query string.
///
/// **Hand-rolled and tiny on purpose**: khor has no URL crate, the
/// input is one filesystem path, and the set that must be escaped in a
/// query value is small and fixed. Everything outside the unreserved
/// set goes as `%XX`, which is always correct and only ever uglier than
/// necessary.
fn enc(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_with_a_space_and_a_hash_survives_the_query_string() {
        // Both of these end a query value if they travel raw, and both
        // are ordinary in a directory name.
        assert_eq!(enc("/a b/c#d"), "/a%20b/c%23d");
    }

    #[test]
    fn a_chinese_directory_is_encoded_bytewise() {
        assert_eq!(enc("/工"), "/%E5%B7%A5");
    }

    #[test]
    fn an_ordinary_path_is_left_alone() {
        assert_eq!(enc("/Users/zgh/Desktop/working/khor"), "/Users/zgh/Desktop/working/khor");
    }
}
