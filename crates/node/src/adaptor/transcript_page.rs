//! Event pages over vendor JSONL windows. Record positions remain in cursors
//! because one record can expand to more than one response page.
use std::{collections::VecDeque, fs::File, io, ops::Range, time::UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{
    claude::Utterance,
    transcript_window::{self, Record},
};

pub const EVENT_LIMIT: usize = 128;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Cursor {
    source: String,
    file: String,
    len: u64,
    modified: (u64, u32),
    position: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Position {
    pub start: u64,
    pub end: u64,
    pub item: usize,
}

pub struct Event {
    pub position: Position,
    pub utterance: Utterance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub range: Range<u64>,
    pub missing_start: bool,
    pub missing_end: bool,
}

pub struct Page {
    pub events: Vec<Event>,
    pub references: Vec<Reference>,
    pub older: Option<Cursor>,
    pub reset: bool,
    origin: Cursor,
}

impl Page {
    pub fn cursor_before(&self, position: Position) -> Cursor {
        Cursor {
            position,
            ..self.origin.clone()
        }
    }
}

struct Observation {
    file: String,
    len: u64,
    modified: (u64, u32),
}

impl Observation {
    fn of(file: &File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        let modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            format!("{}:{}", metadata.dev(), metadata.ino())
        };
        #[cfg(not(unix))]
        let identity = format!("{:?}", metadata.created()?);
        Ok(Self {
            file: identity,
            len: metadata.len(),
            modified: (modified.as_secs(), modified.subsec_nanos()),
        })
    }

    fn changed(&self, cursor: &Cursor) -> bool {
        self.file != cursor.file
            || self.len < cursor.len
            || (self.len == cursor.len && self.modified != cursor.modified)
    }

    fn cursor(&self, source: &str, position: Position) -> Cursor {
        Cursor {
            source: source.to_owned(),
            file: self.file.clone(),
            len: self.len,
            modified: self.modified,
            position,
        }
    }
}

/// Tail when cursor is None; otherwise the events before the supplied position.
/// Append preserves an older-page cursor. Replacement, shrink and same-length
/// rewrites restart at the tail and report reset. In-place rewrites that also
/// grow need a provider generation signal; length alone cannot detect them.
pub fn read_page(
    file: &mut File,
    source: &str,
    cursor: Option<&Cursor>,
    parse: fn(&str) -> Vec<Utterance>,
) -> Result<Page, String> {
    let observation = Observation::of(file).map_err(|e| e.to_string())?;
    if cursor.is_some_and(|cursor| cursor.source != source) {
        return Err("history cursor belongs to another session".to_owned());
    }
    let reset = cursor.is_some_and(|cursor| observation.changed(cursor));
    let cursor = cursor.filter(|_| !reset);
    if let Some(cursor) = cursor {
        if cursor.position.start > cursor.position.end || cursor.position.end > observation.len {
            return Err("invalid history cursor range".to_owned());
        }
    }
    let before = cursor.map(|cursor| {
        if cursor.position.item == 0 {
            cursor.position.start
        } else {
            cursor.position.end
        }
    });
    let window = transcript_window::read_window(file, before).map_err(|e| e.to_string())?;
    let mut events = VecDeque::with_capacity(EVENT_LIMIT);
    let mut references = Vec::new();
    let mut leading_end = None;
    for record in window.records() {
        match record {
            Record::Inline { range, bytes } => {
                let text = String::from_utf8_lossy(bytes);
                for (item, utterance) in parse(&text).into_iter().enumerate() {
                    if cursor.is_some_and(|cursor| {
                        range.start == cursor.position.start && item >= cursor.position.item
                    }) {
                        break;
                    }
                    if events.len() == EVENT_LIMIT {
                        events.pop_front();
                    }
                    events.push_back(Event {
                        position: Position {
                            start: range.start,
                            end: range.end,
                            item,
                        },
                        utterance,
                    });
                }
            }
            Record::Oversized { range } => references.push(Reference {
                range,
                missing_start: false,
                missing_end: false,
            }),
            Record::Fragment {
                range,
                missing_start,
                missing_end,
            } => {
                if range.start == window.range.start && missing_start {
                    leading_end = Some(range.end);
                }
                references.push(Reference {
                    range,
                    missing_start,
                    missing_end,
                });
            }
        }
    }
    let older_position = events
        .front()
        .map(|event| event.position.clone())
        .or_else(|| {
            // Revisit a small boundary record whole on the older page. A window
            // containing only a giant fragment must instead move strictly left.
            window.before.map(|start| {
                let end = leading_end
                    .filter(|end| *end < window.range.end)
                    .unwrap_or(start);
                Position {
                    start: end,
                    end,
                    item: 0,
                }
            })
        });
    let older = older_position
        .filter(|position| position.start > 0 || position.item > 0)
        .map(|position| observation.cursor(source, position));
    if let Some(first) = events.front() {
        let start = first.position.start;
        references.retain(|reference| reference.range.end > start);
    }
    Ok(Page {
        events: events.into(),
        references,
        older,
        reset,
        origin: observation.cursor(
            source,
            Position {
                start: 0,
                end: 0,
                item: 0,
            },
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct Fixture {
        tree: crate::testdir::TempTree,
        path: std::path::PathBuf,
    }
    impl Fixture {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let tree = crate::testdir::TempTree::at(name);
            std::fs::create_dir_all(tree.path()).unwrap();
            let path = tree.path().join("transcript.jsonl");
            std::fs::write(&path, bytes).unwrap();
            Self { tree, path }
        }
        fn page(&self, cursor: Option<&Cursor>) -> Page {
            let mut file = File::open(&self.path).unwrap();
            read_page(&mut file, "source", cursor, parse).unwrap()
        }
    }
    fn parse(line: &str) -> Vec<Utterance> {
        line.trim()
            .split(',')
            .filter(|s| *s != "_")
            .map(|s| Utterance::Agent(s.to_owned()))
            .collect()
    }
    fn words(page: &Page) -> Vec<&str> {
        page.events
            .iter()
            .map(|event| match &event.utterance {
                Utterance::Agent(text) => text.as_str(),
                _ => panic!("unexpected event"),
            })
            .collect()
    }

    #[test]
    fn one_record_continues_before_event_128_without_loss_or_duplication() {
        let text = (0..300)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
            + "\n";
        let fixture = Fixture::new("history-record-events", text.as_bytes());
        let tail = fixture.page(None);
        assert_eq!(words(&tail).first(), Some(&"172"));
        assert_eq!(words(&tail).last(), Some(&"299"));
        let middle = fixture.page(tail.older.as_ref());
        assert_eq!(words(&middle).first(), Some(&"44"));
        assert_eq!(words(&middle).last(), Some(&"171"));
        let first = fixture.page(middle.older.as_ref());
        assert_eq!(words(&first).first(), Some(&"0"));
        assert_eq!(words(&first).last(), Some(&"43"));
        assert!(first.older.is_none());
        assert_eq!(
            tail.events.len() + middle.events.len() + first.events.len(),
            300
        );
    }

    #[test]
    fn small_line_crossing_window_boundary_is_recovered_on_older_page() {
        let prefix = b"_\n".repeat(transcript_window::SCAN_BYTES / 2);
        let text = [
            prefix,
            b"boundary-line\n".to_vec(),
            b"_\n".repeat((transcript_window::SCAN_BYTES - 8) / 2),
        ]
        .concat();
        let fixture = Fixture::new("history-small-boundary", &text);
        let mut cursor = None;
        let mut boundary = 0;
        let mut pages = 0;
        loop {
            let page = fixture.page(cursor.as_ref());
            boundary += words(&page)
                .iter()
                .filter(|word| **word == "boundary-line")
                .count();
            pages += 1;
            cursor = page.older;
            if cursor.is_none() {
                break;
            }
            assert!(pages < 2000);
        }
        assert_eq!(boundary, 1);
    }

    #[test]
    fn append_preserves_cursor_while_replacement_and_shrink_reset() {
        let fixture = Fixture::new("history-file-generation", b"one\ntwo\n");
        let cursor = Observation::of(&File::open(&fixture.path).unwrap())
            .unwrap()
            .cursor(
                "source",
                Position {
                    start: 4,
                    end: 8,
                    item: 0,
                },
            );
        let cursor = &cursor;
        let mut append = std::fs::OpenOptions::new()
            .append(true)
            .open(&fixture.path)
            .unwrap();
        writeln!(append, "three").unwrap();
        let older = fixture.page(Some(cursor));
        assert!(!older.reset);
        assert_eq!(words(&older), vec!["one"]);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&fixture.path)
            .unwrap()
            .set_len(2)
            .unwrap();
        assert!(fixture.page(Some(cursor)).reset);
        let replacement = fixture.tree.path().join("replacement");
        std::fs::write(&replacement, b"new\n").unwrap();
        std::fs::rename(replacement, &fixture.path).unwrap();
        assert!(fixture.page(Some(cursor)).reset);
    }

    #[test]
    fn cursor_is_scoped_to_source_and_corrupt_ranges_are_rejected() {
        let fixture = Fixture::new("history-cursor-scope", b"one\ntwo\n");
        let mut cursor = Observation::of(&File::open(&fixture.path).unwrap())
            .unwrap()
            .cursor(
                "source",
                Position {
                    start: 4,
                    end: 8,
                    item: 0,
                },
            );
        let mut file = File::open(&fixture.path).unwrap();
        assert!(read_page(&mut file, "other", Some(&cursor), parse).is_err());
        cursor.position.end = u64::MAX;
        assert!(read_page(&mut file, "source", Some(&cursor), parse).is_err());
    }

    #[test]
    fn huge_line_pages_make_progress_and_expose_references() {
        let mut text = vec![b'x'; transcript_window::SCAN_BYTES * 3];
        text.push(b'\n');
        let fixture = Fixture::new("history-large-reference", &text);
        let mut page = fixture.page(None);
        let mut pages = 1;
        while let Some(cursor) = page.older {
            assert!(!page.references.is_empty());
            assert!(page.events.is_empty());
            let next = fixture.page(Some(&cursor));
            if let Some(before) = &next.older {
                assert!(before.position.start < cursor.position.start);
            }
            page = next;
            pages += 1;
            assert!(pages < 10);
        }
        assert_eq!(pages, 4);
    }
}
