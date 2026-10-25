use serde_json::Value;
use std::io::Read;

pub(super) fn dispatch<T>(
    reader: &mut dyn Read,
    command: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<T, String> {
    let mut body = String::new();
    reader
        .read_to_string(&mut body)
        .map_err(|why| format!("Could not read JSON request: {why}"))?;
    let args = if body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&body).map_err(|why| format!("Invalid JSON request: {why}"))?
    };
    command(&args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::io::{self, Cursor};

    struct InterruptedBody {
        prefix: Cursor<Vec<u8>>,
    }
    impl Read for InterruptedBody {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let count = self.prefix.read(out)?;
            if count > 0 {
                Ok(count)
            } else {
                Err(io::Error::other("fixture body failure"))
            }
        }
    }

    fn refused_without_dispatch(reader: &mut dyn Read) {
        let called = Cell::new(false);
        let outcome = dispatch(reader, |_| {
            called.set(true);
            Ok(())
        });
        assert!(outcome.is_err(), "invalid input was accepted");
        assert!(!called.get(), "invalid input reached command dispatch");
    }

    #[test]
    fn read_failure_before_body_never_dispatches() {
        refused_without_dispatch(&mut InterruptedBody {
            prefix: Cursor::new(vec![]),
        });
    }
    #[test]
    fn read_failure_after_valid_json_never_dispatches() {
        refused_without_dispatch(&mut InterruptedBody {
            prefix: Cursor::new(b"{}".to_vec()),
        });
    }
    #[test]
    fn invalid_json_never_dispatches() {
        for body in ["{", "oops", "{} {}", "null trailing"] {
            refused_without_dispatch(&mut Cursor::new(body.as_bytes()));
        }
    }
    #[test]
    fn invalid_utf8_never_dispatches() {
        refused_without_dispatch(&mut Cursor::new([0xff]));
    }
    #[test]
    fn empty_body_and_explicit_null_keep_no_argument_semantics() {
        for body in ["", " \r\n\t", "null"] {
            assert_eq!(
                dispatch(&mut Cursor::new(body), |args| Ok(args.clone())).unwrap(),
                Value::Null
            );
        }
    }
    #[test]
    fn valid_arguments_and_command_errors_are_preserved() {
        let body = br#"{"text":"hello","flag":false}"#;
        assert_eq!(
            dispatch(&mut Cursor::new(body), |args| Ok(args.clone())).unwrap(),
            serde_json::json!({"text":"hello","flag":false})
        );
        assert_eq!(
            dispatch::<()>(&mut Cursor::new(body), |_| Err("command refused".into())).unwrap_err(),
            "command refused"
        );
    }
}
