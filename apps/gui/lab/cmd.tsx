// Lab only (lane cm-d): one file in the editor per screen, for the gates'
// probes and the report's shots. `#rs` … picks the screen, `?dark` /
// `?light` the theme. The view is `window.__view`.
//
//   #rs    a 4-space Rust file — Tab at the caret inserts four (item 2)
//   #ts    a TypeScript file, caret on a word that recurs — the active
//          line and the selection's other matches (item 3)
//   #toml  a TOML file (a legacy mode, no tree) — folded by indent (item 4)
//   #py #go #md  one file each, for 「符号」 (item 5) beside #ts and #rs
import { foldable, foldCode, foldService } from "@codemirror/language";
import type { EditorView } from "@codemirror/view";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { CodeEditor } from "@/components/CodeEditor";

const RS = `use std::path::Path;

/// One file's place in the tree.
pub struct Entry {
    pub path: String,
    pub size: u64,
}

impl Entry {
    pub fn new(path: &Path, size: u64) -> Self {
        let path = path.to_string_lossy().into_owned();
        Entry { path, size }
    }
}
`;

const TS = `import { fetchFileText } from "@/api";

/** The draft a file view holds while it is being edited. */
export type Draft = { text: string; was: string };

export async function reread(machine: string, path: string, draft: Draft): Promise<Draft> {
  const now = await fetchFileText(machine, path);
  if (now.digest === draft.was) return draft;
  return { text: now.text, was: now.digest };
}

export function same(a: Draft, b: Draft): boolean {
  return a.text === b.text && a.was === b.was;
}
`;

const TOML = `[package]
name = "khor-node"
version = "0.1.14"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[workspace.metadata.release]
    shared-version = true
    tag-name = "v{{version}}"
    pre-release-hook = [
        "cargo",
        "test",
    ]

[profile.release]
lto = true
`;

const PY = `import os


class Store:
    """Files under one root."""

    def __init__(self, root):
        self.root = root

    def read(self, name):
        with open(os.path.join(self.root, name)) as f:
            return f.read()


def main():
    print(Store(".").read("a"))
`;

const GO = `package store

type Store struct {
	root string
}

func New(root string) *Store {
	return &Store{root: root}
}

func (s *Store) Read(name string) ([]byte, error) {
	return os.ReadFile(filepath.Join(s.root, name))
}
`;

const MD = `# khor

A mesh of your machines.

## Install

Run the script.

### On a Mac

Drag it in.

## Use

Open it.
`;

const screens: Record<string, { path: string; text: string }> = {
  py: { path: "store.py", text: PY },
  go: { path: "store.go", text: GO },
  md: { path: "README.md", text: MD },
  rs: { path: "entry.rs", text: RS },
  ts: { path: "draft.ts", text: TS },
  toml: { path: "Cargo.toml", text: TOML },
};
const s = screens[location.hash.slice(1)] ?? screens.rs;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <div className="flex h-dvh flex-col bg-card text-foreground">
    <CodeEditor path={s.path} text={s.text} editable />
  </div>,
);

// What the driver needs from CodeMirror itself, which a page script
// cannot import.
(window as unknown as { __fold: unknown }).__fold = { foldable, foldCode, foldService };

// The view, once the editor has put it on its box.
const wait = () => {
  const v = (document.querySelector("[data-code-view]") as (HTMLElement & { __view?: EditorView }) | null)?.__view;
  if (v) (window as unknown as { __view: EditorView }).__view = v;
  else requestAnimationFrame(wait);
};
wait();
