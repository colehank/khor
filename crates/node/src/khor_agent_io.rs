use std::io::{self, BufRead, BufReader, Read};

const EVENT_LIMIT: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub(super) enum EventError {
    Read(io::Error),
    TooLarge,
}

pub(super) fn event_lines(reader: impl Read) -> impl Iterator<Item = Result<String, EventError>> {
    let mut reader = BufReader::new(reader);
    let mut ended = false;
    std::iter::from_fn(move || {
        if ended {
            return None;
        }
        let mut line = Vec::new();
        // Include CRLF and one excess byte without allocating an entire
        // unterminated event before the executor can reject it.
        match reader
            .by_ref()
            .take((EVENT_LIMIT + 3) as u64)
            .read_until(b'\n', &mut line)
        {
            Ok(0) => {
                ended = true;
                return None;
            }
            Err(error) => {
                ended = true;
                return Some(Err(EventError::Read(error)));
            }
            Ok(_) => {}
        }
        if line.last() == Some(&b'\n') {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }
        if line.len() > EVENT_LIMIT {
            ended = true;
            return Some(Err(EventError::TooLarge));
        }
        Some(String::from_utf8(line).map_err(|error| {
            ended = true;
            EventError::Read(io::Error::new(io::ErrorKind::InvalidData, error))
        }))
    })
}

pub(super) fn drain_stderr(mut reader: impl Read) {
    let mut bytes = [0; 8192];
    loop {
        match reader.read(&mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct Generated {
        left: usize,
        read: Rc<Cell<usize>>,
    }
    impl Read for Generated {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let size = out.len().min(self.left);
            out[..size].fill(b'x');
            self.left -= size;
            self.read.set(self.read.get() + size);
            Ok(size)
        }
    }

    #[test]
    fn event_lines_preserve_unicode_crlf_empty_and_final_lines() {
        let actual: Vec<_> = event_lines("one\r\n\n中文🔑\nlast\r".as_bytes())
            .map(Result::unwrap)
            .collect();
        assert_eq!(actual, ["one", "", "中文🔑", "last\r"]);
    }

    #[test]
    fn the_existing_event_limit_accepts_a_complete_crlf_line() {
        let input = format!("{}\r\nnext\n", "a".repeat(EVENT_LIMIT));
        let actual: Vec<_> = event_lines(input.as_bytes()).map(Result::unwrap).collect();
        assert_eq!(actual[0].len(), EVENT_LIMIT);
        assert_eq!(actual[1], "next");
    }

    #[test]
    fn an_unterminated_event_is_refused_before_reading_the_whole_pipe() {
        let read = Rc::new(Cell::new(0));
        let mut events = event_lines(Generated {
            left: EVENT_LIMIT * 4,
            read: read.clone(),
        });
        assert!(matches!(events.next(), Some(Err(EventError::TooLarge))));
        assert!(read.get() <= EVENT_LIMIT + 8192 + 3);
        assert!(events.next().is_none());
    }

    #[test]
    fn an_oversized_complete_event_is_not_delivered_to_the_executor() {
        let input = format!("{}\nnext\n", "a".repeat(EVENT_LIMIT + 1));
        let mut events = event_lines(input.as_bytes());
        assert!(matches!(events.next(), Some(Err(EventError::TooLarge))));
        assert!(events.next().is_none());
    }

    struct Broken {
        called: bool,
    }
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            assert!(
                !self.called,
                "a failed drain must not spin on the same reader"
            );
            self.called = true;
            Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "fixture read failure",
            ))
        }
    }

    #[test]
    fn a_stderr_read_error_stops_the_discard_worker() {
        drain_stderr(Broken { called: false });
    }

    #[test]
    fn invalid_utf8_is_an_error_and_does_not_revive_the_reader() {
        let mut events = event_lines(&b"\xff\nnext\n"[..]);
        assert!(matches!(events.next(), Some(Err(EventError::Read(_)))));
        assert!(events.next().is_none());
    }

    #[test]
    fn stderr_is_discarded_as_bytes_without_needing_lines_or_utf8() {
        let read = Rc::new(Cell::new(0));
        drain_stderr(Generated {
            left: EVENT_LIMIT * 4,
            read: read.clone(),
        });
        assert_eq!(read.get(), EVENT_LIMIT * 4);
        drain_stderr(&b"\xff\0\xfe"[..]);
    }
}
