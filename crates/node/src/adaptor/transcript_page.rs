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
    pub(crate) fn record_range(&self) -> Range<u64> {
        self.position.start..self.position.end
    }
    /// Join contiguous giant-record fragments without exposing cursor internals
    /// to the face. This selects bytes, not a claim of valid record syntax.
    pub fn at_record_range(&self, source: &str, start: u64, end: u64) -> Result<Self, String> {
        check_cursor(source, Some(self))?;
        if start >= end || start > self.position.start || end < self.position.end || end > self.len
        {
            return Err("invalid joined history record range".into());
        }
        Ok(Self {
            position: Position {
                start,
                end,
                item: 0,
            },
            pending: None,
            waiting_for_append: false,
            ..self.clone()
        })
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileVersion {
    file: String,
    len: u64,
    modified: (u64, u32),
}

/// Capture only an already-published complete EOF. The writer and the
/// independently opened read path must still identify the same file.
pub(crate) fn capture_eof(
    writer: &File,
    published: &mut File,
    source: &str,
) -> Result<Cursor, String> {
    use std::io::{Read, Seek, SeekFrom};
    if source.is_empty() || source.len() > 256 {
        return Err("invalid capture source identity".into());
    }
    if !writer.metadata().map_err(|error| error.to_string())?.is_file()
        || !published.metadata().map_err(|error| error.to_string())?.is_file()
    {
        return Err("capture source is not a regular file".into());
    }
    let before = FileVersion::of(writer).map_err(|error| error.to_string())?;
    if FileVersion::of(published).map_err(|error| error.to_string())? != before {
        return Err("published capture file identity changed".into());
    }
    if before.len > 0 {
        published.seek(SeekFrom::End(-1)).map_err(|error| error.to_string())?;
        let mut last = [0];
        published.read_exact(&mut last).map_err(|error| error.to_string())?;
        if last != [b'\n'] {
            return Err("capture ends inside an incomplete record".into());
        }
    }
    if FileVersion::of(writer).map_err(|error| error.to_string())? != before
        || FileVersion::of(published).map_err(|error| error.to_string())? != before
    {
        return Err("capture changed while observing its EOF".into());
    }
    Ok(before.cursor(source, Position { start: before.len, end: before.len, item: 0 }))
}

impl FileVersion {
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

pub const WINDOW_WIRE_BYTES: usize = transcript_window::SCAN_BYTES + 16 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRead {
    observation: FileVersion,
    window: Option<transcript_window::TranscriptWindow>,
    forward: bool,
    reset: bool,
}
impl PreparedRead {
    pub fn validate(&self) -> Result<(), String> {
        if self.observation.file.is_empty()
            || self.observation.file.len() > 256
            || self.observation.modified.1 >= 1_000_000_000
        {
            return Err("invalid transcript file version".into());
        }
        if let Some(window) = &self.window {
            window.validate()?;
            if window.observed_len > self.observation.len {
                return Err("transcript changed while reading its window".into());
            }
        }
        if crate::proto::encode(self)?.len() > WINDOW_WIRE_BYTES {
            return Err("serialized transcript window exceeds its wire budget".into());
        }
        Ok(())
    }
    pub(crate) fn validate_request(
        &self,
        cursor: Option<&Cursor>,
        newer: bool,
    ) -> Result<(), String> {
        self.validate()?;
        if self.forward != (newer && !self.reset) {
            return Err("transcript window has another read direction".into());
        }
        if let Some(cursor) = cursor.filter(|_| !self.reset) {
            if self.observation.changed(cursor) {
                return Err("transcript window has another file version".into());
            }
            if self
                .window
                .as_ref()
                .is_some_and(|window| window.observed_len < cursor.len)
            {
                return Err("transcript window predates its cursor".into());
            }
        }
        let cursor = cursor.filter(|_| !self.reset);
        if let Some(window) = &self.window {
            let (start, end) = if self.forward {
                let cursor = cursor.ok_or("forward window has no cursor")?;
                let start = cursor.position.start;
                (
                    start,
                    start
                        .saturating_add((transcript_window::SCAN_BYTES - 1) as u64)
                        .min(window.observed_len),
                )
            } else {
                let end = cursor
                    .map(|cursor| {
                        if cursor.position.item == 0 {
                            cursor.position.start
                        } else {
                            cursor.position.end
                        }
                    })
                    .unwrap_or(window.observed_len);
                (
                    end.saturating_sub((transcript_window::SCAN_BYTES - 1) as u64),
                    end,
                )
            };
            if window.range != (start..end) {
                return Err("transcript window does not match its requested range".into());
            }
        }
        Ok(())
    }
}

fn check_cursor(source: &str, cursor: Option<&Cursor>) -> Result<(), String> {
    if let Some(cursor) = cursor {
        if cursor.source != source {
            return Err("history cursor belongs to another session".into());
        }
        if cursor.position.start > cursor.position.end
            || cursor.position.end > cursor.len
            || cursor
                .pending
                .as_ref()
                .is_some_and(|pending| pending.start > cursor.position.start)
        {
            return Err("invalid history cursor range".into());
        }
    }
    Ok(())
}

/// Only IO and file identity validation happen here. Local and remote readers
/// pass the same bounded result to `parse_prepared`.
pub fn prepare_read(
    file: &mut File,
    source: &str,
    cursor: Option<&Cursor>,
    newer: bool,
    cancelled: &impl Fn() -> bool,
) -> Result<PreparedRead, String> {
    check_cancelled(cancelled)?;
    check_cursor(source, cursor)?;
    if newer && cursor.is_none() {
        return Err("newer history requires a cursor".into());
    }
    let observation = FileVersion::of(file).map_err(|e| e.to_string())?;
    let reset = cursor.is_some_and(|cursor| observation.changed(cursor));
    let cursor = cursor.filter(|_| !reset);
    if let Some(cursor) = cursor {
        if cursor.position.end > observation.len {
            return Err("invalid history cursor range".into());
        }
    }
    let forward = newer && !reset;
    if let Some(cursor) = cursor.filter(|_| forward) {
        if cursor.waiting_for_append
            && observation.len == cursor.len
            && observation.modified == cursor.modified
        {
            return Ok(PreparedRead {
                observation,
                window: None,
                forward,
                reset,
            });
        }
    }
    let window = if forward {
        transcript_window::read_forward_window(
            file,
            cursor
                .expect("forward requires a validated cursor")
                .position
                .start,
        )
    } else {
        let before = cursor.map(|cursor| {
            if cursor.position.item == 0 {
                cursor.position.start
            } else {
                cursor.position.end
            }
        });
        transcript_window::read_window(file, before)
    }
    .map_err(|e| e.to_string())?;
    check_cancelled(cancelled)?;
    let observation = FileVersion::of(file).map_err(|e| e.to_string())?;
    if forward && cursor.is_some_and(|cursor| observation.changed(cursor)) {
        let mut prepared = prepare_read(file, source, None, false, cancelled)?;
        prepared.reset = true;
        return Ok(prepared);
    }
    if observation.len < window.observed_len {
        return Err("transcript changed while reading its window".into());
    }
    let prepared = PreparedRead {
        observation,
        window: Some(window),
        forward,
        reset,
    };
    prepared.validate_request(cursor, newer)?;
    Ok(prepared)
}

pub fn parse_prepared<T>(
    prepared: PreparedRead,
    source: &str,
    cursor: Option<&Cursor>,
    newer: bool,
    parse: fn(&str) -> Vec<T>,
    cancelled: &impl Fn() -> bool,
) -> Result<Page<T>, String> {
    check_cancelled(cancelled)?;
    check_cursor(source, cursor)?;
    prepared.validate_request(cursor, newer)?;
    let cursor = cursor.filter(|_| !prepared.reset);
    let PreparedRead {
        observation,
        window,
        forward,
        reset,
    } = prepared;
    match (window, forward) {
        (Some(window), false) => {
            parse_older(window, observation, source, cursor, reset, parse, cancelled)
        }
        (Some(window), true) => parse_forward(
            window,
            observation,
            source,
            cursor.ok_or("forward window has no cursor")?,
            parse,
            cancelled,
        ),
        (None, true) => {
            let cursor = cursor.ok_or("unchanged window has no cursor")?;
            if !cursor.waiting_for_append
                || observation.changed(cursor)
                || observation.len != cursor.len
                || observation.modified != cursor.modified
            {
                return Err("invalid unchanged transcript window".into());
            }
            Ok(Page {
                events: vec![],
                references: vec![],
                older: None,
                newer: cursor.clone(),
                has_more: false,
                reset: false,
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
        (None, false) => Err("tail window has no bytes".into()),
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
    let prepared = prepare_read(file, source, cursor, false, cancelled)?;
    parse_prepared(prepared, source, cursor, false, parse, cancelled)
}

fn parse_older<T>(
    window: transcript_window::TranscriptWindow,
    observation: FileVersion,
    source: &str,
    cursor: Option<&Cursor>,
    reset: bool,
    parse: fn(&str) -> Vec<T>,
    cancelled: &impl Fn() -> bool,
) -> Result<Page<T>, String> {
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
    let prepared = prepare_read(file, source, Some(cursor), true, cancelled)?;
    parse_prepared(prepared, source, Some(cursor), true, parse, cancelled)
}

fn parse_forward<T>(
    window: transcript_window::TranscriptWindow,
    observation: FileVersion,
    source: &str,
    cursor: &Cursor,
    parse: fn(&str) -> Vec<T>,
    cancelled: &impl Fn() -> bool,
) -> Result<Page<T>, String> {
    let origin = observation.cursor(
        source,
        Position {
            start: 0,
            end: 0,
            item: 0,
        },
    );
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordChunk {
    pub offset: u64,
    #[serde(with = "serde_bytes")]
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
    let observation = FileVersion::of(file).map_err(|error| error.to_string())?;
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
        || FileVersion::of(file)
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
    fn joined_record_range_keeps_source_snapshot_and_actual_read_validation() {
        let fixture = Fixture::new("joined-record", b"0123456789\n");
        let page = fixture.page(None);
        let fragment = page.cursor_before(Position { start: 3, end: 5, item: 0 });
        assert!(fragment.at_record_range("other-source", 0, 10).is_err());
        assert!(fragment.at_record_range("source", 4, 10).is_err());
        assert!(fragment.at_record_range("source", 0, 12).is_err());
        let joined = fragment.at_record_range("source", 0, 10).unwrap();
        let read = |cursor: &Cursor| read_record_chunk(&mut File::open(&fixture.path).unwrap(),
            "source", cursor, 0, &|| false);
        assert_eq!(read(&joined).unwrap().bytes, b"0123456789");
        append(&fixture, b"appended\n");
        assert_eq!(read(&joined).unwrap().bytes, b"0123456789");
        let replacement = fixture.path.with_extension("replacement");
        std::fs::write(&replacement, b"changed-file\n").unwrap();
        std::fs::rename(replacement, &fixture.path).unwrap();
        assert!(read(&joined).unwrap_err().contains("changed"));
    }

    #[test]
    fn capture_eof_separates_identical_records_without_scanning_history() {
        let fixture = Fixture::new("capture-eof", b"same\n");
        let writer = File::open(&fixture.path).unwrap();
        let mut published = File::open(&fixture.path).unwrap();
        let cursor = capture_eof(&writer, &mut published, "source").unwrap();
        assert_eq!(cursor.record_range(), 5..5);
        append(&fixture, b"same\n");
        let before = fixture.page(Some(&cursor));
        let after = next(&fixture, &cursor);
        assert_eq!(words(&before), ["same"]);
        assert_eq!(words(&after), ["same"]);
        assert_eq!(before.events[0].position.start, 0);
        assert_eq!(after.events[0].position.start, 5);
        let empty = Fixture::new("capture-empty", b"");
        let writer = File::open(&empty.path).unwrap();
        let mut published = File::open(&empty.path).unwrap();
        let cursor = capture_eof(&writer, &mut published, "source").unwrap();
        assert_eq!(cursor.record_range(), 0..0);
        assert!(empty.page(Some(&cursor)).events.is_empty());
    }

    #[test]
    fn capture_eof_rejects_an_incomplete_or_replaced_published_file() {
        let partial = Fixture::new("capture-partial", b"partial");
        let writer = File::open(&partial.path).unwrap();
        let mut published = File::open(&partial.path).unwrap();
        assert!(capture_eof(&writer, &mut published, "source").unwrap_err().contains("incomplete"));
        let fixture = Fixture::new("capture-replaced", b"same\n");
        let writer = File::open(&fixture.path).unwrap();
        std::fs::rename(&fixture.path, fixture.path.with_extension("retired")).unwrap();
        std::fs::write(&fixture.path, b"same\n").unwrap();
        let mut published = File::open(&fixture.path).unwrap();
        assert!(capture_eof(&writer, &mut published, "source").unwrap_err().contains("identity changed"));
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
        let cursor = FileVersion::of(&File::open(&fixture.path).unwrap())
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
        let mut cursor = FileVersion::of(&File::open(&fixture.path).unwrap())
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

    #[test]
    fn prepared_wire_round_trip_keeps_record_item_continuations() {
        let text = (0..300)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",")
            + "\n";
        let fixture = Fixture::new("prepared-wire-items", text.as_bytes());
        let mut file = File::open(&fixture.path).unwrap();
        let prepared = prepare_read(&mut file, "source", None, false, &|| false).unwrap();
        let wire = crate::proto::encode(&prepared).unwrap();
        assert!(wire.len() <= WINDOW_WIRE_BYTES);
        let prepared: PreparedRead = crate::proto::decode(&wire).unwrap();
        let tail = parse_prepared(prepared, "source", None, false, parse, &|| false).unwrap();
        assert_eq!(words(&tail), words(&fixture.page(None)));
        let cursor = tail.older.clone().unwrap();
        let prepared = prepare_read(&mut file, "source", Some(&cursor), false, &|| false).unwrap();
        let prepared = crate::proto::decode(&crate::proto::encode(&prepared).unwrap()).unwrap();
        let older =
            parse_prepared(prepared, "source", Some(&cursor), false, parse, &|| false).unwrap();
        let prepared =
            prepare_read(&mut file, "source", Some(&older.newer), true, &|| false).unwrap();
        let prepared = crate::proto::decode(&crate::proto::encode(&prepared).unwrap()).unwrap();
        let forward = parse_prepared(prepared, "source", Some(&older.newer), true, parse, &|| {
            false
        })
        .unwrap();
        assert_eq!(words(&forward), words(&tail));
    }

    #[test]
    fn serialized_window_budget_includes_binary_payload_and_invalid_metadata_is_rejected() {
        fn forbidden(_: &str) -> Vec<Utterance> {
            panic!("giant raw record parsed inline");
        }
        let fixture = Fixture::new(
            "prepared-wire-budget",
            &vec![255; transcript_window::SCAN_BYTES * 2],
        );
        let mut file = File::open(&fixture.path).unwrap();
        let prepared = prepare_read(&mut file, "source", None, false, &|| false).unwrap();
        let bytes = crate::proto::encode(&prepared).unwrap();
        assert!(
            bytes.len() <= WINDOW_WIRE_BYTES,
            "binary bytes must use the existing bytes codec"
        );
        let prepared = crate::proto::decode(&bytes).unwrap();
        let page = parse_prepared(prepared, "source", None, false, forbidden, &|| false).unwrap();
        assert!(page.events.is_empty());
        assert!(!page.references.is_empty());
        let mut prepared = prepare_read(&mut file, "source", None, false, &|| false).unwrap();
        prepared.observation.file = "x".repeat(257);
        assert!(parse_prepared(prepared, "source", None, false, forbidden, &|| false).is_err());
    }

    #[test]
    fn prepared_io_does_not_parse_after_cancellation_and_replacement_still_resets() {
        let fixture = Fixture::new("prepared-cancel-reset", b"first\n");
        let before = fixture.page(None);
        let mut file = File::open(&fixture.path).unwrap();
        let prepared = prepare_read(&mut file, "source", None, false, &|| false).unwrap();
        assert!(
            parse_prepared(prepared, "source", None, false, parse, &|| true)
                .err()
                .unwrap()
                .contains("cancelled")
        );
        drop(file);
        let old = fixture.path.with_extension("old");
        std::fs::rename(&fixture.path, &old).unwrap();
        std::fs::write(&fixture.path, b"second\n").unwrap();
        let mut file = File::open(&fixture.path).unwrap();
        let prepared =
            prepare_read(&mut file, "source", Some(&before.newer), true, &|| false).unwrap();
        let prepared = crate::proto::decode(&crate::proto::encode(&prepared).unwrap()).unwrap();
        let reset = parse_prepared(
            prepared,
            "source",
            Some(&before.newer),
            true,
            parse,
            &|| false,
        )
        .unwrap();
        assert!(reset.reset);
        assert_eq!(words(&reset), ["second"]);
    }
    #[test]
    fn prepared_wire_is_bound_to_request_direction_range_and_file_version() {
        fn forbidden(_: &str) -> Vec<Utterance> {
            panic!("inconsistent wire response reached its parser");
        }
        let fixture = Fixture::new("prepared-request-scope", b"first\n");
        let cursor = fixture.page(None).newer;
        let fresh = || {
            prepare_read(
                &mut File::open(&fixture.path).unwrap(),
                "source",
                Some(&cursor),
                true,
                &|| false,
            )
            .unwrap()
        };
        let mut invalid_cursor = cursor.clone();
        invalid_cursor.pending = Some(PendingRecord {
            start: cursor.position.start + 1,
            missing_start: false,
        });
        assert!(
            parse_prepared(
                fresh(),
                "source",
                Some(&invalid_cursor),
                true,
                forbidden,
                &|| false
            )
            .err()
            .unwrap()
            .contains("cursor range")
        );
        let mut response = fresh();
        response.observation.file = "another-file".into();
        assert!(
            parse_prepared(response, "source", Some(&cursor), true, forbidden, &|| {
                false
            })
            .err()
            .unwrap()
            .contains("version")
        );
        let mut response = fresh();
        let window = response.window.as_mut().unwrap();
        window.range = 0..0;
        window.before = None;
        assert!(
            parse_prepared(response, "source", Some(&cursor), true, forbidden, &|| {
                false
            })
            .err()
            .unwrap()
            .contains("range")
        );
        assert!(
            parse_prepared(fresh(), "source", Some(&cursor), false, forbidden, &|| {
                false
            })
            .err()
            .unwrap()
            .contains("direction")
        );
        let mut response = fresh();
        response.window.as_mut().unwrap().observed_len += 1;
        assert!(
            parse_prepared(response, "source", Some(&cursor), true, forbidden, &|| {
                false
            })
            .err()
            .unwrap()
            .contains("changed")
        );
        let mut response = fresh();
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&fixture.path)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
        response.observation = FileVersion::of(&File::open(&fixture.path).unwrap()).unwrap();
        let page = parse_prepared(response, "source", Some(&cursor), true, forbidden, &|| {
            false
        })
        .unwrap();
        assert!(
            page.events.is_empty(),
            "a concurrent append is read on the next window"
        );
        assert_eq!(words(&next(&fixture, &page.newer)), ["second"]);
    }
}
