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
    let outcome = dispatch("console_catalog", reader, |_| {
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
            dispatch("console_catalog", &mut Cursor::new(body), |args| Ok(
                args.clone()
            ))
            .unwrap(),
            Value::Null
        );
    }
}
#[test]
fn valid_arguments_and_command_errors_are_preserved() {
    let body = br#"{"text":"hello","flag":false}"#;
    assert_eq!(
        dispatch("console_catalog", &mut Cursor::new(body), |args| Ok(
            args.clone()
        ))
        .unwrap(),
        serde_json::json!({"text":"hello","flag":false})
    );
    assert_eq!(
        dispatch::<()>("console_catalog", &mut Cursor::new(body), |_| Err(
            "command refused".into()
        ))
        .unwrap_err()
        .message,
        "command refused"
    );
}

fn tiny_limits() -> Limits {
    Limits {
        strings: 32,
        string: 16,
        encoded: 512,
        width: 4,
        nodes: 20,
        depth: 128,
    }
}

struct Chunks<'a> {
    bytes: &'a [u8],
    width: usize,
    read: usize,
}
impl Read for Chunks<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = self.width.min(out.len()).min(self.bytes.len());
        out[..count].copy_from_slice(&self.bytes[..count]);
        self.bytes = &self.bytes[count..];
        self.read += count;
        Ok(count)
    }
}

#[test]
fn escaped_strings_are_counted_across_every_read_boundary() {
    for source in [
        r#""a\\b\"c\n\t""#,
        r#""\u0000\u007f\u0080\u0800""#,
        r#""\ud83d\ude00\u0061""#,
        "\"汉😀字\"",
    ] {
        for width in 1..=source.len() {
            let expected: Value = serde_json::from_str(source).unwrap();
            let bytes = expected.as_str().unwrap().len();
            let limits = Limits {
                string: bytes,
                strings: bytes,
                ..tiny_limits()
            };
            let mut reader = Chunks {
                bytes: source.as_bytes(),
                width,
                read: 0,
            };
            assert_eq!(
                read(&mut reader, limits).unwrap(),
                expected,
                "{source} width {width}"
            );
            let mut reader = Chunks {
                bytes: source.as_bytes(),
                width,
                read: 0,
            };
            let error = read(
                &mut reader,
                Limits {
                    string: bytes - 1,
                    ..limits
                },
            )
            .unwrap_err();
            assert_eq!(error.code, 413, "{source} width {width}");
        }
    }
}

#[test]
fn syntax_remains_serdes_responsibility() {
    for source in [
        r#""\ud800""#,
        r#""\udc00""#,
        r#""\ud800\u0041""#,
        r#""\uxyz0""#,
        r#""\q""#,
        "[1,]",
        "{\"a\":}",
    ] {
        let error = read(
            &mut Cursor::new(source),
            Limits::for_command("console_catalog"),
        )
        .unwrap_err();
        assert_eq!(error.code, 400, "{source}");
    }
}

#[test]
fn an_oversize_ascii_string_stops_before_reading_the_rest() {
    let mut source = Chunks {
        bytes: br#""aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#,
        width: 1,
        read: 0,
    };
    let error = read(&mut source, tiny_limits()).unwrap_err();
    assert_eq!(error.code, 413);
    assert!(error.message.contains("single string"));
    assert_eq!(
        source.read, 18,
        "one quote and the first excessive decoded byte"
    );
}

#[test]
fn encoded_and_aggregate_decoded_budgets_are_independent() {
    let mut whitespace = io::repeat(b' ');
    assert_eq!(
        read(
            &mut whitespace,
            Limits {
                encoded: 8,
                ..tiny_limits()
            }
        )
        .unwrap_err()
        .code,
        413
    );
    let source = r#"["1234567890123456","1234567890123456","x"]"#;
    let error = read(&mut Cursor::new(source), tiny_limits()).unwrap_err();
    assert_eq!(error.code, 413);
    assert!(error.message.contains("decoded string"));
}

#[test]
fn container_width_is_rejected_before_an_extra_value_is_decoded() {
    // The excessive entry has a type error of its own. Width must win before
    // parsing that value; duplicate keys also consume entries.
    for source in [
        "[0,0,0,0,broken]",
        r#"{"a":0,"b":0,"c":0,"d":0,"extra":broken}"#,
        r#"{"a":0,"a":0,"a":0,"a":0,"a":broken}"#,
    ] {
        let error = read(&mut Cursor::new(source), tiny_limits()).unwrap_err();
        assert_eq!(error.code, 413, "{source}: {error:?}");
        assert!(error.message.contains("container entries"));
    }
}

#[test]
fn total_nodes_are_rejected_before_an_extra_value_is_decoded() {
    let limits = Limits {
        width: 10,
        nodes: 3,
        ..tiny_limits()
    };
    let error = read(&mut Cursor::new("[0,0,broken]"), limits).unwrap_err();
    assert_eq!(error.code, 413);
    assert!(error.message.contains("total values"));
}

#[test]
fn valid_json_values_keep_serdes_shapes_and_duplicate_key_behavior() {
    for source in [
        "null",
        "true",
        "-123",
        "18446744073709551615",
        "1.25e2",
        r#"{"a":1,"a":2}"#,
        "[0,false,{},[]]",
    ] {
        let value = read(
            &mut Cursor::new(source),
            Limits::for_command("console_catalog"),
        )
        .unwrap();
        assert_eq!(value, serde_json::from_str::<Value>(source).unwrap());
    }
}

struct Pieces(std::collections::VecDeque<Box<dyn Read>>);
impl Read for Pieces {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        while let Some(front) = self.0.front_mut() {
            let count = front.read(out)?;
            if count != 0 {
                return Ok(count);
            }
            self.0.pop_front();
        }
        Ok(0)
    }
}
struct Pattern {
    bytes: &'static [u8],
    left: usize,
    offset: usize,
}
impl Read for Pattern {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = out.len().min(self.left);
        if self.bytes.len() == 1 {
            out[..count].fill(self.bytes[0]);
        } else {
            for (i, byte) in out[..count].iter_mut().enumerate() {
                *byte = self.bytes[(self.offset + i) % self.bytes.len()];
            }
        }
        self.left -= count;
        self.offset = (self.offset + count) % self.bytes.len();
        Ok(count)
    }
}
fn text_piece(bytes: &'static [u8], repeats: usize) -> Box<dyn Read> {
    Box::new(Pattern {
        bytes,
        left: bytes.len() * repeats,
        offset: 0,
    })
}
fn literal(bytes: &'static [u8]) -> Box<dyn Read> {
    Box::new(Cursor::new(bytes))
}

#[test]
fn full_size_write_and_sixfold_json_escaping_remain_legal() {
    for pattern in [b"a".as_slice(), br"\u0001".as_slice()] {
        let mut source = Pieces(
            [
                literal(br#"{"machine":"local","path":"scratch.txt","was":"","text":""#),
                text_piece(pattern, TEXT),
                literal(br#""}"#),
            ]
            .into(),
        );
        let value = read(&mut source, Limits::for_command("write_text")).unwrap();
        let text = value["text"].as_str().unwrap();
        assert_eq!(text.len(), TEXT);
        assert_eq!(
            text.as_bytes()[0],
            if pattern.len() == 1 { b'a' } else { 1 }
        );
    }
}

#[test]
fn all_three_full_size_merge_inputs_remain_legal() {
    let mut source = Pieces(
        [
            literal(br#"{"base":""#),
            text_piece(b"a", TEXT),
            literal(br#"","mine":""#),
            text_piece(b"b", TEXT),
            literal(br#"","theirs":""#),
            text_piece(b"c", TEXT),
            literal(br#""}"#),
        ]
        .into(),
    );
    let value = read(&mut source, Limits::for_command("merge_text")).unwrap();
    for name in ["base", "mine", "theirs"] {
        assert_eq!(value[name].as_str().unwrap().len(), TEXT);
    }
}

#[test]
fn a_legal_twenty_thousand_item_drop_keeps_all_items() {
    let mut body = String::from(r#"{"id":"isolated","items":["#);
    for i in 0..20_000 {
        if i != 0 {
            body.push(',');
        }
        body.push_str(&format!(
            r#"{{"path":"folder/{i}.txt","from":null,"keep":true}}"#
        ));
    }
    body.push_str("]}");
    let value = read(
        &mut Cursor::new(body),
        Limits::for_command("term_drop_items"),
    )
    .unwrap();
    assert_eq!(value["items"].as_array().unwrap().len(), 20_000);
}

#[test]
fn a_full_size_base64_chat_image_keeps_the_existing_wire_budget() {
    // Base64 is already the image representation on the 64MiB msgpack road.
    // Leave room for the message/frame fields, rather than reducing it to a
    // small HTTP envelope cap or base64ing it a second time.
    let image = TEXT - 1024;
    let mut source = Pieces(
        [
            literal(br#"{"id":"isolated","text":"hello","images":[["image/png",""#),
            text_piece(b"A", image),
            literal(br#""]],"files":[]}"#),
        ]
        .into(),
    );
    let value = read(&mut source, Limits::for_command("chat_say")).unwrap();
    assert_eq!(value["images"][0][1].as_str().unwrap().len(), image);
}

#[test]
fn the_existing_serde_nesting_boundary_is_preserved_and_reported_as_a_limit() {
    for depth in [126, 127, 128, 129] {
        let source = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        let original = serde_json::from_str::<Value>(&source);
        let bounded = read(
            &mut Cursor::new(&source),
            Limits::for_command("console_catalog"),
        );
        assert_eq!(bounded.is_ok(), original.is_ok(), "depth {depth}");
        if let Err(error) = bounded {
            assert_eq!(error.code, 413, "depth {depth}");
        }
    }
}
