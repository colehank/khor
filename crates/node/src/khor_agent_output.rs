//! Credential-safe text at the agent's display/receipt boundaries.
use khor_catalog::msg;
use std::borrow::Cow;

/// Empty means credentials are held by the inference runtime, not a pattern to
/// replace. Call once on raw output; replacement markers are display data.
pub(super) fn redact<'a>(text: &'a str, key: &str) -> Cow<'a, str> {
    if key.is_empty() {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.replace(key, "[redacted]"))
    }
}

pub(super) fn append_text(output: &mut String, text: &str, key: &str) {
    const CAP: usize = 128 * 1024;
    if output.len() >= CAP {
        return;
    }
    output.push_str(&redact(text, key));
    if output.len() > CAP {
        let mut cut = CAP;
        while !output.is_char_boundary(cut) {
            cut -= 1;
        }
        output.truncate(cut);
        output.push('\n');
        output.push_str(msg::console_output_cut());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_credentials_preserve_unicode_output() {
        let text = "OAuth 网络失败🔑";
        assert_eq!(redact(text, ""), text);
        let mut output = String::new();
        append_text(&mut output, text, "");
        assert_eq!(output, text);
    }

    #[test]
    fn every_occurrence_is_redacted_before_utf8_safe_output_truncation() {
        assert_eq!(redact("key / key", "key"), "[redacted] / [redacted]");
        assert_eq!(
            redact("密钥🔑 / 密钥🔑", "密钥🔑"),
            "[redacted] / [redacted]"
        );
        let mut output = String::new();
        append_text(
            &mut output,
            &format!("fixture-key{}", "界".repeat(100_000)),
            "fixture-key",
        );
        assert!(output.starts_with("[redacted]"));
        assert!(!output.contains("fixture-key"));
        assert!(output.ends_with(msg::console_output_cut()));
        assert!(output.len() <= 128 * 1024 + 1 + msg::console_output_cut().len());
        let cut = output.clone();
        append_text(&mut output, "ignored beyond cap", "");
        assert_eq!(output, cut);
    }
}
