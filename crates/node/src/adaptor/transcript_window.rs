//! Bounded byte windows over an append-only JSONL transcript.
//!
//! A window is not a parsed conversation. Edge fragments remain explicit so
//! callers cannot silently discard a large record or parse an unfinished write.
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Range;

pub const SCAN_BYTES: usize = 256 * 1024;
pub const INLINE_RECORD_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub struct TranscriptWindow {
    pub range: Range<u64>,
    pub before: Option<u64>,
    pub observed_len: u64,
    bytes: Vec<u8>,
    begins_record: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Record<'a> {
    Inline {
        range: Range<u64>,
        bytes: &'a [u8],
    },
    Oversized {
        range: Range<u64>,
    },
    Fragment {
        range: Range<u64>,
        missing_start: bool,
        missing_end: bool,
    },
}

impl TranscriptWindow {
    /// Ranges include their newline. Only Inline records may be parsed here;
    /// oversized records and fragments require a separate bounded fetch path.
    pub fn records(&self) -> impl Iterator<Item = Record<'_>> {
        let mut offset = 0;
        self.bytes
            .split_inclusive(|b| *b == b'\n')
            .map(move |line| {
                let first = offset == 0;
                let range = self.range.start + offset as u64
                    ..self.range.start + (offset + line.len()) as u64;
                offset += line.len();
                let missing_start = first && !self.begins_record;
                let missing_end = line.last() != Some(&b'\n');
                if missing_start || missing_end {
                    Record::Fragment {
                        range,
                        missing_start,
                        missing_end,
                    }
                } else if line.len() > INLINE_RECORD_BYTES {
                    Record::Oversized { range }
                } else {
                    Record::Inline { range, bytes: line }
                }
            })
    }
}

/// `before` is exclusive; None starts at the observed EOF. One byte of the
/// read budget identifies whether the left edge begins a record. Byte offsets
/// deliberately survive UTF-8 fragments; only complete lines reach parsing.
pub fn read_window(
    reader: &mut (impl Read + Seek),
    before: Option<u64>,
) -> io::Result<TranscriptWindow> {
    let observed_len = reader.seek(SeekFrom::End(0))?;
    let end = before.unwrap_or(observed_len).min(observed_len);
    let start = end.saturating_sub((SCAN_BYTES - 1) as u64);
    read_range(reader, observed_len, start, end)
}

pub fn read_forward_window(
    reader: &mut (impl Read + Seek),
    from: u64,
) -> io::Result<TranscriptWindow> {
    let observed_len = reader.seek(SeekFrom::End(0))?;
    let start = from.min(observed_len);
    let end = start
        .saturating_add((SCAN_BYTES - 1) as u64)
        .min(observed_len);
    read_range(reader, observed_len, start, end)
}

fn read_range(
    reader: &mut (impl Read + Seek),
    observed_len: u64,
    start: u64,
    end: u64,
) -> io::Result<TranscriptWindow> {
    let begins_record = if start == 0 {
        true
    } else {
        reader.seek(SeekFrom::Start(start - 1))?;
        let mut previous = [0];
        reader.read_exact(&mut previous)?;
        previous[0] == b'\n'
    };
    reader.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity((end - start) as usize);
    reader.take(end - start).read_to_end(&mut bytes)?;
    let actual_end = start + bytes.len() as u64;
    Ok(TranscriptWindow {
        range: start..actual_end,
        before: (start > 0).then_some(start),
        observed_len,
        bytes,
        begins_record,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct Counted {
        inner: Cursor<Vec<u8>>,
        read: usize,
        short: usize,
    }
    impl Counted {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                inner: Cursor::new(bytes),
                read: 0,
                short: usize::MAX,
            }
        }
    }
    impl Read for Counted {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(self.short);
            let n = self.inner.read(&mut out[..n])?;
            self.read += n;
            Ok(n)
        }
    }
    impl Seek for Counted {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.inner.seek(to)
        }
    }

    #[test]
    fn first_window_reads_only_the_tail_with_a_fixed_budget() {
        let mut input = Counted::new(b"{}\n".repeat(SCAN_BYTES * 4));
        let page = read_window(&mut input, None).unwrap();
        assert!(input.read <= SCAN_BYTES, "read {} bytes", input.read);
        assert!(page.range.start > 0);
        assert_eq!(page.range.end, input.inner.get_ref().len() as u64);
        assert!(page.records().any(|r| matches!(r, Record::Inline { .. })));
    }

    #[test]
    fn empty_and_beginning_have_no_previous_page() {
        for bytes in [vec![], b"{}\n".to_vec()] {
            let mut input = Cursor::new(bytes);
            let page = read_window(&mut input, None).unwrap();
            assert_eq!(page.before, None);
            assert_eq!(read_window(&mut input, Some(0)).unwrap().range, 0..0);
        }
    }

    #[test]
    fn exclusive_cursor_clamps_to_eof_and_short_reads_are_completed() {
        let mut input = Counted::new(b"first\nsecond\n".to_vec());
        input.short = 2;
        let page = read_window(&mut input, Some(6)).unwrap();
        assert_eq!(page.range, 0..6);
        assert_eq!(
            page.records().collect::<Vec<_>>(),
            vec![Record::Inline {
                range: 0..6,
                bytes: b"first\n"
            }]
        );
        assert_eq!(
            read_window(&mut input, Some(u64::MAX)).unwrap().range,
            0..13
        );
    }

    #[test]
    fn huge_complete_records_are_references_and_fragments_do_not_stall_paging() {
        let mut bytes = vec![b'x'; SCAN_BYTES * 3];
        bytes.push(b'\n');
        let mut input = Counted::new(bytes);
        let mut before = None;
        let mut last = input.inner.get_ref().len() as u64;
        let mut pages = 0;
        loop {
            input.read = 0;
            let page = read_window(&mut input, before).unwrap();
            assert!(input.read <= SCAN_BYTES);
            assert_eq!(page.range.end, last);
            assert_eq!(page.records().count(), 1);
            assert!(matches!(
                page.records().next(),
                Some(Record::Fragment { .. })
            ));
            pages += 1;
            match page.before {
                Some(next) => {
                    assert!(next < last);
                    last = next;
                    before = Some(next);
                }
                None => break,
            }
        }
        assert_eq!(pages, 4);
        let mut complete = Cursor::new([vec![b'x'; INLINE_RECORD_BYTES + 1], vec![b'\n']].concat());
        let page = read_window(&mut complete, None).unwrap();
        assert_eq!(
            page.records().collect::<Vec<_>>(),
            vec![Record::Oversized {
                range: 0..(INLINE_RECORD_BYTES + 2) as u64
            }]
        );
    }

    #[test]
    fn incomplete_tail_remains_a_fragment_until_the_writer_finishes() {
        let mut input = Cursor::new(b"{}\npartial".to_vec());
        let page = read_window(&mut input, None).unwrap();
        assert_eq!(
            page.records().collect::<Vec<_>>(),
            vec![
                Record::Inline {
                    range: 0..3,
                    bytes: b"{}\n"
                },
                Record::Fragment {
                    range: 3..10,
                    missing_start: false,
                    missing_end: true
                },
            ]
        );
        input.get_mut().push(b'\n');
        assert!(
            read_window(&mut input, None)
                .unwrap()
                .records()
                .all(|r| matches!(r, Record::Inline { .. }))
        );
    }

    struct ChangedDuringRead {
        inner: Cursor<Vec<u8>>,
        replace: Option<Vec<u8>>,
    }
    impl Read for ChangedDuringRead {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if let Some(next) = self.replace.take() {
                *self.inner.get_mut() = next;
            }
            self.inner.read(out)
        }
    }
    impl Seek for ChangedDuringRead {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.inner.seek(to)
        }
    }

    #[test]
    fn concurrent_truncation_reports_only_the_bytes_actually_read() {
        let mut input = ChangedDuringRead {
            inner: Cursor::new(b"first\nsecond\n".to_vec()),
            replace: Some(b"{}\n".to_vec()),
        };
        let page = read_window(&mut input, None).unwrap();
        assert_eq!(page.observed_len, 13);
        assert_eq!(page.range, 0..3);
        assert_eq!(page.before, None);
        assert_eq!(page.records().count(), 1);
    }

    #[test]
    fn concurrent_append_is_left_for_the_next_observation() {
        let mut input = ChangedDuringRead {
            inner: Cursor::new(b"{}\n".to_vec()),
            replace: Some(b"{}\nnext\n".to_vec()),
        };
        let page = read_window(&mut input, None).unwrap();
        assert_eq!(page.observed_len, 3);
        assert_eq!(page.range, 0..3);
        assert_eq!(page.records().count(), 1);
        assert_eq!(read_window(&mut input, None).unwrap().range, 0..8);
    }

    #[test]
    fn utf8_cut_at_the_left_edge_is_not_offered_to_the_parser() {
        let mut bytes = "中".repeat(SCAN_BYTES).into_bytes();
        bytes.extend_from_slice(b"\n{}\n");
        let mut input = Cursor::new(bytes);
        let page = read_window(&mut input, None).unwrap();
        assert!(matches!(
            page.records().next(),
            Some(Record::Fragment {
                missing_start: true,
                ..
            })
        ));
        assert_eq!(
            page.records().last(),
            Some(Record::Inline {
                range: page.range.end - 3..page.range.end,
                bytes: b"{}\n"
            })
        );
    }
}
