use std::io::Read;
use std::io::Result as IoResult;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};

/// A `Reader` that reads exactly the number of bytes from a sub-reader.
///
/// If the limit is reached, it returns EOF. If the limit is not reached
/// when the destructor is called, the remaining bytes will be read and
/// thrown away.
pub struct EqualReader<R>
where
    R: Read,
{
    reader: R,
    size: usize,
    last_read_signal: Sender<IoResult<()>>,
}

impl<R> EqualReader<R>
where
    R: Read,
{
    pub fn new(reader: R, size: usize) -> (EqualReader<R>, Receiver<IoResult<()>>) {
        let (tx, rx) = channel();

        let r = EqualReader {
            reader,
            size,
            last_read_signal: tx,
        };

        (r, rx)
    }
}

impl<R> Read for EqualReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if self.size == 0 {
            return Ok(0);
        }

        let buf = if buf.len() < self.size {
            buf
        } else {
            &mut buf[..self.size]
        };

        match self.reader.read(buf) {
            Ok(len) => {
                self.size -= len;
                Ok(len)
            }
            err @ Err(_) => err,
        }
    }
}

impl<R> Drop for EqualReader<R>
where
    R: Read,
{
    fn drop(&mut self) {
        let mut remaining_to_read = self.size;

        let mut buf = [0; 8192];
        while remaining_to_read > 0 {
            let wanted = remaining_to_read.min(buf.len());
            match self.reader.read(&mut buf[..wanted]) {
                Err(e) => {
                    self.last_read_signal.send(Err(e)).ok();
                    break;
                }
                Ok(0) => {
                    self.last_read_signal.send(Ok(())).ok();
                    break;
                }
                Ok(other) => {
                    remaining_to_read -= other;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EqualReader;
    use std::io::Read;

    #[test]
    fn test_limit() {
        use std::io::Cursor;

        let mut org_reader = Cursor::new("hello world".to_string().into_bytes());

        {
            let (mut equal_reader, _) = EqualReader::new(org_reader.by_ref(), 5);

            let mut string = String::new();
            equal_reader.read_to_string(&mut string).unwrap();
            assert_eq!(string, "hello");
        }

        let mut string = String::new();
        org_reader.read_to_string(&mut string).unwrap();
        assert_eq!(string, " world");
    }

    #[test]
    fn test_not_enough() {
        use std::io::Cursor;

        let mut org_reader = Cursor::new("hello world".to_string().into_bytes());

        {
            let (mut equal_reader, _) = EqualReader::new(org_reader.by_ref(), 5);

            let mut vec = [0];
            equal_reader.read_exact(&mut vec).unwrap();
            assert_eq!(vec[0], b'h');
        }

        let mut string = String::new();
        org_reader.read_to_string(&mut string).unwrap();
        assert_eq!(string, " world");
    }
}

#[cfg(test)]
mod bounded_cleanup_tests {
    use super::EqualReader;
    use std::io::{self, Read};
    use std::sync::{Arc, Mutex};

    struct Fails(Arc<Mutex<Vec<usize>>>);
    impl Read for Fails {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.0.lock().unwrap().push(buffer.len());
            Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "fixture disconnected body",
            ))
        }
    }

    #[test]
    fn an_arbitrary_declared_remainder_uses_a_fixed_cleanup_buffer() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (body, done) = EqualReader::new(Fails(seen.clone()), usize::MAX);
        drop(body);
        assert_eq!(*seen.lock().unwrap(), [8192]);
        assert_eq!(
            done.recv().unwrap().unwrap_err().kind(),
            io::ErrorKind::ConnectionReset
        );
    }

    #[test]
    fn cleanup_drains_only_the_unread_declared_bytes() {
        let bytes = vec![7; 25_000];
        let mut source = io::Cursor::new(bytes);
        let (mut body, done) = EqualReader::new(&mut source, 20_000);
        let mut prefix = [0; 3];
        body.read_exact(&mut prefix).unwrap();
        drop(body);
        assert_eq!(source.position(), 20_000);
        assert!(
            done.recv().is_err(),
            "complete drain preserves sender-disconnect semantics"
        );
    }

    #[test]
    fn early_eof_keeps_the_original_success_notification() {
        let (body, done) = EqualReader::new(io::Cursor::new([1, 2]), 20_000);
        drop(body);
        assert!(done.recv().unwrap().is_ok());
    }
}
