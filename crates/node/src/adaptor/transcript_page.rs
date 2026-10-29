//! Event pages over JSONL windows. Record positions remain in cursors
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
    pending: Option<PendingRecord>,
    waiting_for_append: bool,
}

impl Cursor {
    pub fn is_for_source(&self, source: &str) -> bool {
        self.source == source
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
struct PendingRecord {
    start: u64,
    missing_start: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Position {
    pub start: u64,
    pub end: u64,
    pub item: usize,
}

pub struct Event<T = Utterance> {
    pub position: Position,
    pub payload: T,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub range: Range<u64>,
    pub missing_start: bool,
    pub missing_end: bool,
}

pub struct Page<T = Utterance> {
    pub events: Vec<Event<T>>,
    pub references: Vec<Reference>,
    pub older: Option<Cursor>,
    pub reset: bool,
    pub newer: Cursor,
    pub has_more: bool,
    origin: Cursor,
}

impl<T> Page<T> {
    pub fn cursor_before(&self, position: Position) -> Cursor {
        Cursor {
            position,
            ..self.origin.clone()
        }
    }
    pub fn cursor_after(&self, mut position: Position) -> Cursor {
        position.item += 1;
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
            len: self.len.max(position.end),
            modified: self.modified,
            position,
            pending: None,
            waiting_for_append: false,
        }
    }
}

fn check_cancelled(cancelled: &impl Fn() -> bool) -> Result<(), String> {
    if cancelled() {
        Err("history request cancelled".to_owned())
    } else {
        Ok(())
    }
}

/// Tail when cursor is None; otherwise the events before the supplied position.
/// Append preserves an older-page cursor. Replacement, shrink and same-length
/// rewrites restart at the tail and report reset. In-place rewrites that also
/// grow need a provider generation signal; length alone cannot detect them.
pub fn read_page<T>(
    file: &mut File,
    source: &str,
    cursor: Option<&Cursor>,
    parse: fn(&str) -> Vec<T>,
    cancelled: &impl Fn() -> bool,
) -> Result<Page<T>, String> {
    check_cancelled(cancelled)?;
    let observation = Observation::of(file).map_err(|e| e.to_string())?;
    if cursor.is_some_and(|cursor| cursor.source != source) {
        return Err("history cursor belongs to another session".to_owned());
    }
    if cursor.is_some_and(|cursor| {
        cursor.position.start > cursor.position.end || cursor.position.end > cursor.len
    }) {
        return Err("invalid history cursor range".to_owned());
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
    check_cancelled(cancelled)?;
    let observation = Observation::of(file).map_err(|error| error.to_string())?;
    let mut events = VecDeque::with_capacity(EVENT_LIMIT);
    let mut references = Vec::new();
    let mut leading_end = None;
    let mut newer = observation.cursor(
        source,
        Position {
            start: window.range.start,
            end: window.range.start,
            item: 0,
        },
    );
    for record in window.records() {
        check_cancelled(cancelled)?;
        match record {
            Record::Inline { range, bytes } => {
                newer.position = Position {
                    start: range.end,
                    end: range.end,
                    item: 0,
                };
                let text = String::from_utf8_lossy(bytes);
                for (item, utterance) in parse(&text).into_iter().enumerate() {
                    check_cancelled(cancelled)?;
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
                        payload: utterance,
                    });
                }
            }
            Record::Oversized { range } => {
                newer.position = Position {
                    start: range.end,
                    end: range.end,
                    item: 0,
                };
                references.push(Reference {
                    range,
                    missing_start: false,
                    missing_end: false,
                });
            }
            Record::Fragment {
                range,
                missing_start,
                missing_end,
            } => {
                if range.start == window.range.start && missing_start {
                    leading_end = Some(range.end);
                }
                if missing_end {
                    if missing_start
                        || range.end - range.start > transcript_window::INLINE_RECORD_BYTES as u64
                    {
                        newer.position = Position {
                            start: range.end,
                            end: range.end,
                            item: 0,
                        };
                        newer.pending = Some(PendingRecord {
                            start: range.start,
                            missing_start,
                        });
                    } else {
                        newer.position = Position {
                            start: range.start,
                            end: range.start,
                            item: 0,
                        };
                    }
                    newer.waiting_for_append = true;
                } else {
                    newer.position = Position {
                        start: range.end,
                        end: range.end,
                        item: 0,
                    };
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
    // An older page ends immediately before its input cursor, including an
    // intra-record boundary. Its newer cursor must not skip the rest of that record.
    let newer = cursor
        .map(|cursor| observation.cursor(source, cursor.position.clone()))
        .unwrap_or(newer);
    Ok(Page {
        events: events.into(),
        references,
        older,
        reset,
        newer,
        has_more: false,
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

/// Continue after a newer cursor without replaying the existing transcript.
/// Incomplete inline-sized records keep their start; oversized pending records
/// advance by byte windows and remain references until their newline arrives.
pub fn read_next<T>(
    file: &mut File,
    source: &str,
    cursor: &Cursor,
    parse: fn(&str) -> Vec<T>,
    cancelled: &impl Fn() -> bool,
) -> Result<Page<T>, String> {
    check_cancelled(cancelled)?;
    let observation = Observation::of(file).map_err(|error| error.to_string())?;
    if cursor.source != source {
        return Err("history cursor belongs to another session".to_owned());
    }
    if cursor.position.start > cursor.position.end || cursor.position.end > cursor.len {
        return Err("invalid history cursor range".to_owned());
    }
    if observation.changed(cursor) {
        let mut page = read_page(file, source, None, parse, cancelled)?;
        page.reset = true;
        return Ok(page);
    }
    if cursor.position.start > cursor.position.end
        || cursor.position.end > observation.len
        || cursor
            .pending
            .as_ref()
            .is_some_and(|pending| pending.start > cursor.position.start)
    {
        return Err("invalid history cursor range".to_owned());
    }
    let origin = observation.cursor(
        source,
        Position {
            start: 0,
            end: 0,
            item: 0,
        },
    );
    if cursor.waiting_for_append
        && observation.len == cursor.len
        && observation.modified == cursor.modified
    {
        return Ok(Page {
            events: vec![],
            references: vec![],
            older: None,
            newer: cursor.clone(),
            has_more: false,
            reset: false,
            origin,
        });
    }
    let window = transcript_window::read_forward_window(file, cursor.position.start)
        .map_err(|error| error.to_string())?;
    check_cancelled(cancelled)?;
    let observation = Observation::of(file).map_err(|error| error.to_string())?;
    if observation.changed(cursor) {
        let mut page = read_page(file, source, None, parse, cancelled)?;
        page.reset = true;
        return Ok(page);
    }
    let mut newer = observation.cursor(source, cursor.position.clone());
    let mut pending = cursor.pending.clone();
    let mut events = Vec::with_capacity(EVENT_LIMIT);
    let mut references = Vec::new();
    let mut has_more = false;
    'records: for record in window.records() {
        check_cancelled(cancelled)?;
        match record {
            Record::Inline { range, bytes } if pending.is_none() => {
                let skip = if range.start == cursor.position.start {
                    cursor.position.item
                } else {
                    0
                };
                for (item, utterance) in parse(&String::from_utf8_lossy(bytes))
                    .into_iter()
                    .enumerate()
                    .skip(skip)
                {
                    check_cancelled(cancelled)?;
                    if events.len() == EVENT_LIMIT {
                        newer = observation.cursor(
                            source,
                            Position {
                                start: range.start,
                                end: range.end,
                                item,
                            },
                        );
                        has_more = true;
                        break 'records;
                    }
                    events.push(Event {
                        position: Position {
                            start: range.start,
                            end: range.end,
                            item,
                        },
                        payload: utterance,
                    });
                }
                newer.position = Position {
                    start: range.end,
                    end: range.end,
                    item: 0,
                };
            }
            Record::Oversized { range } => {
                references.push(Reference {
                    range: range.clone(),
                    missing_start: false,
                    missing_end: false,
                });
                newer.position = Position {
                    start: range.end,
                    end: range.end,
                    item: 0,
                };
            }
            other => {
                let (range, missing_start, missing_end) = match other {
                    Record::Inline { range, .. } => (range, false, false),
                    Record::Fragment {
                        range,
                        missing_start,
                        missing_end,
                    } => (range, missing_start, missing_end),
                    Record::Oversized { .. } => unreachable!(),
                };
                let start = pending
                    .as_ref()
                    .map_or(range.start, |pending| pending.start);
                let missing_start = pending
                    .as_ref()
                    .map_or(missing_start, |pending| pending.missing_start);
                references.push(Reference {
                    range: start..range.end,
                    missing_start,
                    missing_end,
                });
                if missing_end
                    && pending.is_none()
                    && !missing_start
                    && range.end - range.start <= transcript_window::INLINE_RECORD_BYTES as u64
                {
                    newer.position = Position {
                        start,
                        end: start,
                        item: 0,
                    };
                } else {
                    newer.position = Position {
                        start: range.end,
                        end: range.end,
                        item: 0,
                    };
                    pending = missing_end.then_some(PendingRecord {
                        start,
                        missing_start,
                    });
                }
                newer.pending = pending.clone();
                newer.waiting_for_append = missing_end && range.end == observation.len;
            }
        }
    }
    has_more |= !newer.waiting_for_append && newer.position.start < observation.len;
    Ok(Page {
        events,
        references,
        older: None,
        newer,
        has_more,
        reset: false,
        origin,
    })
}

pub struct RecordChunk {
    pub offset: u64,
    pub bytes: Vec<u8>,
    pub next: Option<u64>,
}

/// Raw bytes are returned only on explicit demand; never parse or assemble an
/// oversized JSON object as a side effect of displaying its history reference.
pub fn read_record_chunk(
    file: &mut File,
    source: &str,
    cursor: &Cursor,
    from: u64,
    cancelled: &impl Fn() -> bool,
) -> Result<RecordChunk, String> {
    use std::io::{Read, Seek, SeekFrom};
    check_cancelled(cancelled)?;
    let observation = Observation::of(file).map_err(|error| error.to_string())?;
    if source != cursor.source || observation.changed(cursor) {
        return Err("history record changed; reload its page".to_owned());
    }
    if cursor.position.start > from
        || from > cursor.position.end
        || cursor.position.end > observation.len
    {
        return Err("invalid history record range".to_owned());
    }
    let count = (cursor.position.end - from).min(transcript_window::INLINE_RECORD_BYTES as u64);
    file.seek(SeekFrom::Start(from))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(count as usize);
    (&mut *file)
        .take(count)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    check_cancelled(cancelled)?;
    if bytes.len() as u64 != count
        || Observation::of(file)
            .map_err(|error| error.to_string())?
            .changed(cursor)
    {
        return Err("history record changed during reading".to_owned());
    }
    let end = from + count;
    Ok(RecordChunk {
        offset: from,
        bytes,
        next: (end < cursor.position.end).then_some(end),
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
            read_page(&mut file, "source", cursor, parse, &|| false).unwrap()
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
            .map(|event| match &event.payload {
                Utterance::Agent(text) => text.as_str(),
                _ => panic!("unexpected event"),
            })
            .collect()
    }

    fn next(fixture: &Fixture, cursor: &Cursor) -> Page {
        let mut file = File::open(&fixture.path).unwrap();
        read_next(&mut file, "source", cursor, parse, &|| false).unwrap()
    }
    fn append(fixture: &Fixture, bytes: &[u8]) {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&fixture.path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }

    #[test]
    fn newer_recovers_the_other_side_of_an_intra_record_older_cursor() {
        let text = (0..300)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
            + "\n";
        let fixture = Fixture::new("history-newer-record", text.as_bytes());
        let tail = fixture.page(None);
        let middle = fixture.page(tail.older.as_ref());
        assert_eq!(words(&middle).first(), Some(&"44"));
        let newer = next(&fixture, &middle.newer);
        assert_eq!(words(&newer).first(), Some(&"172"));
        assert_eq!(words(&newer).last(), Some(&"299"));
        assert!(!newer.has_more);
    }

    #[test]
    fn newer_waits_for_a_partial_line_then_parses_it_once() {
        let fixture = Fixture::new("history-newer-partial", b"one\n");
        let tail = fixture.page(None);
        append(&fixture, b"two");
        let partial = next(&fixture, &tail.newer);
        assert!(partial.events.is_empty());
        assert!(!partial.has_more);
        let unchanged = next(&fixture, &partial.newer);
        assert!(unchanged.events.is_empty());
        assert_eq!(unchanged.newer, partial.newer);
        append(&fixture, b"\nthree\n");
        let completed = next(&fixture, &partial.newer);
        assert_eq!(words(&completed), ["two", "three"]);
        assert!(next(&fixture, &completed.newer).events.is_empty());
    }

    #[test]
    fn giant_pending_record_advances_without_becoming_inline_text() {
        let fixture = Fixture::new("history-newer-giant", b"one\n");
        let tail = fixture.page(None);
        append(&fixture, &vec![b'x'; transcript_window::SCAN_BYTES + 10]);
        let first = next(&fixture, &tail.newer);
        assert!(first.events.is_empty());
        assert!(first.has_more);
        let pending = next(&fixture, &first.newer);
        assert!(pending.events.is_empty());
        assert!(!pending.has_more);
        append(&fixture, b"\nlast\n");
        let completed = next(&fixture, &pending.newer);
        assert_eq!(words(&completed), ["last"]);
        assert_eq!(completed.references[0].range.start, 4);
        assert!(!completed.references[0].missing_end);
    }

    #[test]
    fn explicit_record_chunks_are_bounded_and_keep_exact_original_bytes() {
        let mut text = vec![b'x'; transcript_window::INLINE_RECORD_BYTES * 3];
        text.push(b'\n');
        let fixture = Fixture::new("history-record-chunks", &text);
        let page = fixture.page(None);
        assert!(page.events.is_empty());
        let reference = &page.references[0];
        let cursor = page.cursor_before(Position {
            start: reference.range.start,
            end: reference.range.end,
            item: 0,
        });
        let mut file = File::open(&fixture.path).unwrap();
        let mut offset = reference.range.start;
        let mut collected = Vec::new();
        loop {
            let chunk = read_record_chunk(&mut file, "source", &cursor, offset, &|| false).unwrap();
            assert!(chunk.bytes.len() <= transcript_window::INLINE_RECORD_BYTES);
            collected.extend(chunk.bytes);
            match chunk.next {
                Some(next) => {
                    assert!(next > offset);
                    offset = next;
                }
                None => break,
            }
        }
        assert_eq!(collected, text);
        assert!(
            read_record_chunk(
                &mut file,
                "source",
                &cursor,
                reference.range.end + 1,
                &|| false
            )
            .is_err()
        );
        assert!(read_record_chunk(&mut file, "other", &cursor, 0, &|| false).is_err());
        std::fs::OpenOptions::new()
            .write(true)
            .open(&fixture.path)
            .unwrap()
            .set_len(1)
            .unwrap();
        assert!(read_record_chunk(&mut file, "source", &cursor, 0, &|| false).is_err());
    }

    #[test]
    fn worker_cancellation_stops_before_parsing_another_record() {
        let fixture = Fixture::new("history-cancel-parser", b"one\ntwo\nthree\n");
        let checks = std::cell::Cell::new(0);
        let cancelled = || {
            checks.set(checks.get() + 1);
            checks.get() >= 5
        };
        let mut file = File::open(&fixture.path).unwrap();
        assert!(read_page(&mut file, "source", None, parse, &cancelled).is_err());
        assert_eq!(checks.get(), 5);
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
        assert!(read_page(&mut file, "other", Some(&cursor), parse, &|| false).is_err());
        cursor.position.end = u64::MAX;
        assert!(read_page(&mut file, "source", Some(&cursor), parse, &|| false).is_err());
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
