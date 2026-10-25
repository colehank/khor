use std::fmt;
use std::io::{self, BufRead, BufReader, Read};

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

const MIB: usize = 1024 * 1024;
const TEXT: usize = 64 * MIB;

#[derive(Clone, Copy)]
struct Limits {
    strings: usize,
    string: usize,
    encoded: usize,
    width: usize,
    nodes: usize,
    depth: usize,
}

impl Limits {
    fn for_command(command: &str) -> Self {
        let strings = if command == "merge_text" {
            3 * TEXT
        } else {
            TEXT
        } + MIB;
        Self {
            strings,
            string: TEXT,
            // A decoded byte can require six JSON bytes (\u0000). The extra
            // MiB covers punctuation/whitespace without retaining an envelope.
            encoded: strings * 6 + MIB,
            width: 65_536,
            nodes: 262_144,
            depth: 128,
        }
    }
}

#[derive(Debug)]
pub(super) struct Failure {
    pub code: u16,
    pub message: String,
}

impl Failure {
    fn bad(message: String) -> Self {
        Self { code: 400, message }
    }
    fn limit(reason: &str) -> Self {
        Self {
            code: 413,
            message: format!("JSON request exceeds {reason} limit"),
        }
    }
}

pub(super) fn dispatch<T>(
    command_name: &str,
    reader: &mut dyn Read,
    command: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<T, Failure> {
    let args = read(reader, Limits::for_command(command_name))?;
    command(&args).map_err(Failure::bad)
}

fn read(reader: &mut dyn Read, limits: Limits) -> Result<Value, Failure> {
    let mut input = BufReader::new(CountedRead::new(reader, limits));
    let mut tree = Tree {
        limits,
        nodes: 0,
        violation: None,
    };
    let parsed = (|| -> Result<Value, serde_json::Error> {
        // Empty/whitespace bodies have always represented absent arguments.
        loop {
            let bytes = input.fill_buf().map_err(serde_json::Error::io)?;
            if bytes.is_empty() {
                return Ok(Value::Null);
            }
            let spaces = bytes
                .iter()
                .take_while(|b| matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
                .count();
            if spaces == 0 {
                break;
            }
            input.consume(spaces);
        }
        let mut decoder = serde_json::Deserializer::from_reader(&mut input);
        let args = Seed {
            tree: &mut tree,
            depth: 0,
            allowed: true,
        }
        .deserialize(&mut decoder)?;
        decoder.end()?;
        Ok(args)
    })();
    if let Some(reason) = input.get_ref().violation.or(tree.violation) {
        return Err(Failure::limit(reason));
    }
    parsed.map_err(|why| {
        let kind = if why.is_io() {
            "Could not read JSON request"
        } else {
            "Invalid JSON request"
        };
        Failure::bad(format!("{kind}: {why}"))
    })
}

// This counts resources only. serde_json remains the sole syntax/UTF-8
// validator. Counting happens before these bytes reach its string scratch.
struct CountedRead<'a> {
    reader: &'a mut dyn Read,
    limits: Limits,
    encoded: usize,
    strings: usize,
    string: usize,
    state: StringState,
    high_surrogate: bool,
    nesting: usize,
    violation: Option<&'static str>,
}

#[derive(Clone, Copy)]
enum StringState {
    Outside,
    Text,
    Escape,
    Unicode { digits: u8, unit: u16 },
}

impl<'a> CountedRead<'a> {
    fn new(reader: &'a mut dyn Read, limits: Limits) -> Self {
        Self {
            reader,
            limits,
            encoded: 0,
            strings: 0,
            string: 0,
            state: StringState::Outside,
            high_surrogate: false,
            nesting: 0,
            violation: None,
        }
    }
    fn reject(&mut self, reason: &'static str) -> io::Error {
        self.violation = Some(reason);
        io::Error::other("JSON request resource limit")
    }
    fn charge(&mut self, bytes: usize) -> io::Result<()> {
        if bytes > self.limits.string.saturating_sub(self.string) {
            return Err(self.reject("single string bytes"));
        }
        if bytes > self.limits.strings.saturating_sub(self.strings) {
            return Err(self.reject("decoded string bytes"));
        }
        self.string += bytes;
        self.strings += bytes;
        Ok(())
    }
    fn count(&mut self, byte: u8) -> io::Result<()> {
        if self.encoded == self.limits.encoded {
            return Err(self.reject("encoded body bytes"));
        }
        self.encoded += 1;
        match self.state {
            StringState::Outside if byte == b'"' => {
                self.string = 0;
                self.high_surrogate = false;
                self.state = StringState::Text;
            }
            StringState::Outside => match byte {
                b'[' | b'{' => {
                    self.nesting += 1;
                    // serde's existing recursion budget refuses the 128th
                    // nested container. Report that resource refusal as 413.
                    if self.nesting >= self.limits.depth {
                        return Err(self.reject("nesting depth"));
                    }
                }
                b']' | b'}' => self.nesting = self.nesting.saturating_sub(1),
                _ => {}
            },
            StringState::Text => match byte {
                b'"' => self.state = StringState::Outside,
                b'\\' => self.state = StringState::Escape,
                _ => {
                    self.charge(1)?;
                    self.high_surrogate = false;
                }
            },
            StringState::Escape if byte == b'u' => {
                self.state = StringState::Unicode { digits: 0, unit: 0 }
            }
            StringState::Escape => {
                self.charge(1)?;
                self.high_surrogate = false;
                self.state = StringState::Text;
            }
            StringState::Unicode { digits, unit } => {
                // Invalid hex is left for serde to reject; it cannot gain an
                // allocation bypass because each escape still costs bytes.
                let nibble = (byte as char).to_digit(16).unwrap_or(0) as u16;
                let unit = (unit << 4) | nibble;
                if digits == 3 {
                    let bytes = match unit {
                        0xd800..=0xdbff => 3,
                        0xdc00..=0xdfff if self.high_surrogate => 1,
                        0..=0x7f => 1,
                        0x80..=0x7ff => 2,
                        _ => 3,
                    };
                    self.charge(bytes)?;
                    self.high_surrogate = (0xd800..=0xdbff).contains(&unit);
                    self.state = StringState::Text;
                } else {
                    self.state = StringState::Unicode {
                        digits: digits + 1,
                        unit,
                    };
                }
            }
        }
        Ok(())
    }
}

impl Read for CountedRead<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let count = self.reader.read(out)?;
        for byte in &out[..count] {
            self.count(*byte)?;
        }
        Ok(count)
    }
}

struct Tree {
    limits: Limits,
    nodes: usize,
    violation: Option<&'static str>,
}
impl Tree {
    fn reject<E: de::Error>(&mut self, reason: &'static str) -> E {
        self.violation = Some(reason);
        E::custom("JSON request resource limit")
    }
}
struct Seed<'a> {
    tree: &'a mut Tree,
    depth: usize,
    allowed: bool,
}
impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;
    fn deserialize<D: de::Deserializer<'de>>(self, decoder: D) -> Result<Value, D::Error> {
        if !self.allowed {
            return Err(self.tree.reject("container entries"));
        }
        if self.tree.nodes == self.tree.limits.nodes {
            return Err(self.tree.reject("total values"));
        }
        if self.depth > self.tree.limits.depth {
            return Err(self.tree.reject("nesting depth"));
        }
        self.tree.nodes += 1;
        decoder.deserialize_any(ValueVisitor {
            tree: self.tree,
            depth: self.depth,
        })
    }
}
struct ValueVisitor<'a> {
    tree: &'a mut Tree,
    depth: usize,
}
impl<'de> Visitor<'de> for ValueVisitor<'_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.to_owned()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = access.next_element_seed(Seed {
            allowed: values.len() < self.tree.limits.width,
            tree: self.tree,
            depth: self.depth + 1,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        let mut entries = 0;
        while let Some(key) = access.next_key_seed(Seed {
            allowed: entries < self.tree.limits.width,
            tree: self.tree,
            depth: self.depth + 1,
        })? {
            let Value::String(key) = key else {
                return Err(de::Error::custom("JSON object key must be a string"));
            };
            let value = access.next_value_seed(Seed {
                allowed: true,
                tree: self.tree,
                depth: self.depth + 1,
            })?;
            values.insert(key, value);
            entries += 1;
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
#[path = "json_request_tests.rs"]
mod tests;
