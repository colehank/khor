//! Credential-safe text at the agent's display/receipt boundaries.
use khor_catalog::msg;
use std::{borrow::Cow, sync::Arc};

const CAP: usize = 128 * 1024;

/// Empty means credentials are held by the inference runtime, not a pattern to
/// replace. Call once on raw output; replacement markers are display data.
pub(super) fn redact<'a>(text: &'a str, key: &str) -> Cow<'a, str> {
    if key.is_empty() {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.replace(key, "[redacted]"))
    }
}

pub(super) struct TextStream<'a> {
    key: &'a str,
    prefixes: Arc<[usize]>,
    pending: String,
}

impl<'a> TextStream<'a> {
    pub(super) fn new(key: &'a str) -> Self {
        let bytes = key.as_bytes();
        let mut prefixes = vec![0; bytes.len()];
        for at in 1..bytes.len() {
            let mut matched = prefixes[at - 1];
            while matched > 0 && bytes[at] != bytes[matched] {
                matched = prefixes[matched - 1];
            }
            if bytes[at] == bytes[matched] {
                matched += 1;
            }
            prefixes[at] = matched;
        }
        Self {
            key,
            prefixes: prefixes.into(),
            pending: String::new(),
        }
    }

    /// Share the credential's prefix table, with independent raw input state.
    pub(super) fn fresh(&self) -> Self {
        Self {
            key: self.key,
            prefixes: self.prefixes.clone(),
            pending: String::new(),
        }
    }

    pub(super) fn push(&mut self, text: &str) -> String {
        if self.key.is_empty() {
            return text.to_owned();
        }
        let mut raw = std::mem::take(&mut self.pending);
        raw.push_str(text);
        let mut display = String::with_capacity(raw.len());
        let mut after = 0;
        for (at, _) in raw.match_indices(self.key) {
            display.push_str(&raw[after..at]);
            display.push_str("[redacted]");
            after = at + self.key.len();
        }

        // Only raw input participates in matching. Display markers never return
        // to this buffer, even when the key itself is part of "[redacted]".
        let remainder = &raw[after..];
        let bytes = self.key.as_bytes();
        let mut matched = 0;
        for byte in remainder.bytes() {
            while matched > 0 && byte != bytes[matched] {
                matched = self.prefixes[matched - 1];
            }
            if byte == bytes[matched] {
                matched += 1;
            }
        }
        let emit = remainder.len() - matched;
        display.push_str(&remainder[..emit]);
        self.pending.push_str(&remainder[emit..]);
        display
    }

    pub(super) fn finish(&mut self) -> String {
        std::mem::take(&mut self.pending)
    }

    pub(super) fn append_to(&mut self, output: &mut String, text: &str) {
        if output.len() >= CAP {
            self.pending.clear();
            return;
        }
        append_bounded(output, &self.push(text));
    }

    pub(super) fn finish_into(&mut self, output: &mut String) {
        append_bounded(output, &self.finish());
    }

    pub(super) fn retained_len(&self) -> usize {
        self.pending.len()
    }
}

pub(super) fn append_bounded(output: &mut String, text: &str) -> bool {
    if output.len() >= CAP {
        return false;
    }
    output.push_str(text);
    if output.len() > CAP {
        let mut cut = CAP;
        while !output.is_char_boundary(cut) {
            cut -= 1;
        }
        output.truncate(cut);
        output.push('\n');
        output.push_str(msg::console_output_cut());
        return false;
    }
    output.len() < CAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_credentials_preserve_unicode_output() {
        let text = "OAuth 网络失败🔑";
        assert_eq!(redact(text, ""), text);
        let mut stream = TextStream::new("");
        let mut output = String::new();
        append_bounded(&mut output, &stream.push(text));
        assert_eq!(output, text);
    }

    #[test]
    fn every_occurrence_is_redacted_before_utf8_safe_output_truncation() {
        assert_eq!(redact("key / key", "key"), "[redacted] / [redacted]");
        assert_eq!(
            redact("密钥🔑 / 密钥🔑", "密钥🔑"),
            "[redacted] / [redacted]"
        );
        let mut stream = TextStream::new("fixture-key");
        let mut output = String::new();
        append_bounded(
            &mut output,
            &stream.push(&format!("fixture-key{}", "界".repeat(100_000))),
        );
        assert!(output.starts_with("[redacted]"));
        assert!(!output.contains("fixture-key"));
        assert!(output.ends_with(msg::console_output_cut()));
        assert!(output.len() <= 128 * 1024 + 1 + msg::console_output_cut().len());
        let cut = output.clone();
        append_bounded(&mut output, "ignored beyond cap");
        assert_eq!(output, cut);
    }

    #[test]
    fn every_unicode_split_matches_one_pass_redaction_without_exposing_prefixes() {
        for key in ["", "a", "aba", "aaaa", "redact", "凭证🔑", "a界a界"] {
            let raw = format!("prefix {key} / {key}{key} suffix");
            for split in (0..=raw.len()).filter(|at| raw.is_char_boundary(*at)) {
                let mut stream = TextStream::new(key);
                let mut display = stream.push(&raw[..split]);
                assert!(key.is_empty() || stream.retained_len() < key.len());
                display.push_str(&stream.push(&raw[split..]));
                assert!(key.is_empty() || stream.retained_len() < key.len());
                display.push_str(&stream.finish());
                assert_eq!(display, redact(&raw, key), "split {split}, key {key:?}");
                assert!(stream.finish().is_empty());
            }
        }
        let mut stream = TextStream::new("fixture-split-key");
        assert_eq!(stream.push("before fixture-"), "before ");
        assert_eq!(stream.push("split-key after"), "[redacted] after");
        assert!(stream.finish().is_empty());
    }

    #[test]
    fn one_character_chunks_and_partial_final_keys_keep_original_text() {
        for (key, raw) in [
            ("ababa", "abababababa"),
            ("aaaa", "aaaaaaaaaa"),
            ("redact", "redact/redacted/redact"),
            ("密钥🔑", "密钥🔑密钥🔑密钥"),
            ("abac", "abaac"),
        ] {
            let mut stream = TextStream::new(key);
            let mut display = String::new();
            for ch in raw.chars() {
                display.push_str(&stream.push(&ch.to_string()));
            }
            display.push_str(&stream.finish());
            assert_eq!(display, redact(raw, key));
        }
    }

    #[test]
    fn all_small_chunk_partitions_match_leftmost_non_overlapping_replacement() {
        for (key, raw) in [
            ("aba", "ababaaba"),
            ("aaaa", "aaaaaa"),
            ("redact", "redactredact"),
            ("密钥🔑", "密钥🔑密钥🔑"),
        ] {
            let mut boundaries: Vec<usize> = raw.char_indices().map(|(at, _)| at).collect();
            boundaries.push(raw.len());
            for mask in 0..1 << (boundaries.len() - 2) {
                let mut stream = TextStream::new(key);
                let mut display = String::new();
                let mut start = 0;
                for (index, at) in boundaries.iter().copied().enumerate().skip(1) {
                    if at == raw.len() || mask & (1 << (index - 1)) != 0 {
                        display.push_str(&stream.push(&raw[start..at]));
                        start = at;
                    }
                }
                display.push_str(&stream.finish());
                assert_eq!(display, raw.replace(key, "[redacted]"));
            }
        }
    }

    #[test]
    fn ending_a_bounded_stream_cannot_restore_text_after_the_cap() {
        let key = "fixture-split-key";
        let mut stream = TextStream::new(key);
        let mut output = String::new();
        append_bounded(&mut output, &stream.push(&"x".repeat(CAP)));
        append_bounded(&mut output, &stream.push("fixture-"));
        append_bounded(&mut output, &stream.finish());
        assert_eq!(output, "x".repeat(CAP));

        let mut stream = TextStream::new(key);
        let mut output = String::new();
        append_bounded(&mut output, &stream.push(&"界".repeat(CAP / 3)));
        append_bounded(&mut output, &stream.push("fixture-"));
        append_bounded(&mut output, &stream.push("split-key"));
        append_bounded(&mut output, &stream.finish());
        assert!(!output.contains(key));
        assert!(output.len() <= CAP + 1 + msg::console_output_cut().len());
    }
}
