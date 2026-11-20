//! A draft preview from a known terminal application's visible composer.
//!
//! The terminal input path arms the preview. The rendered editor supplies
//! the text, including edits made by the application itself.

const EDITOR_RADIUS: u16 = 24;
const PREVIEW_LIMIT: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Agent {
    Codex,
    Claude,
}

impl Agent {
    pub(super) fn executable(path: &str) -> Option<Self> {
        let name = path.rsplit(['/', '\\']).next()?;
        let stem = name
            .strip_suffix(".exe")
            .or_else(|| name.strip_suffix(".cmd"))
            .or_else(|| name.strip_suffix(".ps1"))
            .unwrap_or(name);
        match stem {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    pub(super) fn command(line: &str) -> Option<Self> {
        let line = line.trim_start();
        let line = line
            .strip_prefix("exec ")
            .or_else(|| line.strip_prefix("& "))
            .unwrap_or(line)
            .trim_start();
        let executable = if let Some(quote @ ('\'' | '"')) = line.chars().next() {
            line[1..].split_once(quote)?.0
        } else {
            line.split_whitespace().next()?
        };
        Self::executable(executable)
    }

    fn marker(self) -> char {
        match self {
            Self::Codex => '›',
            Self::Claude => '❯',
        }
    }
}

#[derive(Default)]
pub(super) struct Capture {
    // The shell command number separates successive runs of the same agent.
    source: Option<(Agent, u64)>,
    focused: bool,
    edited: bool,
}

impl Capture {
    pub(super) fn input(&mut self, keys: &[u8]) {
        let keys = keys.strip_prefix(b"\x1b[200~").unwrap_or(keys);
        if self.focused && keys.first().is_some_and(|&b| b >= b' ' && b != 0x7f) {
            self.edited = true;
        }
    }

    pub(super) fn update(
        &mut self,
        source: Option<(Agent, u64)>,
        screen: &vt100::Screen,
    ) -> Option<Option<String>> {
        // The outer option lets ordinary shell output skip touching the ledger.
        if source.is_none() && self.source.is_none() {
            return None;
        }
        if source != self.source {
            *self = Self {
                source,
                ..Self::default()
            };
        }
        let editor = source.and_then(|(agent, _)| Editor::read(agent, screen));
        self.focused = editor.is_some();
        Some(editor.filter(|_| self.edited).and_then(Editor::excerpt))
    }
}

struct Editor {
    lines: Vec<String>,
    first: usize,
    end: usize,
    prefix: usize,
    placeholder: bool,
}

impl Editor {
    fn read(agent: Agent, screen: &vt100::Screen) -> Option<Self> {
        let (row, col) = screen.cursor_position();
        let (rows, cols) = screen.size();
        if col < 2 {
            return None;
        }
        // Claude paints its own cursor in reverse video at the reported
        // cursor position and leaves the terminal's hardware cursor hidden.
        if screen.hide_cursor() && (agent != Agent::Claude || !screen.cell(row, col)?.inverse()) {
            return None;
        }
        // A new session can draw the composer near the top of an empty terminal.
        let offset = row.saturating_sub(EDITOR_RADIUS);
        let bound = rows.min(row.saturating_add(EDITOR_RADIUS + 1));
        let cursor = usize::from(row - offset);
        let lines: Vec<_> = screen
            .rows(0, cols)
            .skip(offset.into())
            .take((bound - offset).into())
            .collect();
        let first = (0..=cursor).rev().find(|&index| {
            lines
                .get(index)
                .and_then(|line| prompt_text(agent, line))
                .is_some()
        })?;
        let end = match agent {
            Agent::Codex => {
                let prompt = screen.cell(offset + first as u16, 0)?;
                if !prompt.bold() {
                    return None;
                }
                codex_box_end(screen, &lines, offset, first, cursor).or_else(|| {
                    (cursor + 2..lines.len())
                        .find(|&index| {
                            lines[index - 1].trim().is_empty() && codex_status(&lines[index..])
                        })
                        .map(|footer| footer - 1)
                })?
            }
            Agent::Claude => {
                if !claude_rule(lines.get(first.checked_sub(1)?)?, cols) {
                    return None;
                }
                (cursor + 1..lines.len()).find(|&index| claude_rule(&lines[index], cols))?
            }
        };
        let text = prompt_text(agent, &lines[first])?;
        let prefix = lines[first].len() - text.len();
        let at_start = cursor == first && col == 2;
        let placeholder = at_start
            && end == first + 1
            && (screen
                .cell(offset + first as u16, 2)
                .is_some_and(|cell| cell.dim())
                || agent == Agent::Claude && text.starts_with("Try "));
        Some(Self {
            lines,
            first,
            end,
            prefix,
            placeholder,
        })
    }

    fn excerpt(self) -> Option<String> {
        if self.placeholder {
            return None;
        }
        let mut text: String = self.lines[self.first..self.end]
            .iter()
            .enumerate()
            .flat_map(|(i, line)| {
                let line = if i == 0 {
                    &line[self.prefix..]
                } else {
                    line.strip_prefix("  ").unwrap_or(line)
                };
                (i != 0).then_some('\n').into_iter().chain(line.chars())
            })
            .take(PREVIEW_LIMIT)
            .collect();
        text.truncate(text.trim_end().len());
        (!text.is_empty()).then_some(text)
    }
}

fn prompt_text(agent: Agent, line: &str) -> Option<&str> {
    let line = line.strip_prefix(agent.marker())?;
    if line.is_empty() {
        Some(line)
    } else {
        line.strip_prefix([' ', '\u{a0}'])
    }
}

fn codex_box_end(
    screen: &vt100::Screen,
    lines: &[String],
    offset: u16,
    first: usize,
    cursor: usize,
) -> Option<usize> {
    // Codex paints padding above and below the editor in the same background.
    // Its optional status bar can omit the context indicator entirely.
    let padding = first.checked_sub(1)?;
    let background = |row: usize| screen.cell(offset + row as u16, 0).map(|c| c.bgcolor());
    let paint = background(first)?;
    if paint == vt100::Color::Default || !lines[padding].trim().is_empty() {
        return None;
    }
    let edge = (padding..lines.len()).find(|&row| background(row) != Some(paint))?;
    let bottom = edge.checked_sub(1)?;
    (bottom > cursor && lines[bottom].trim().is_empty()).then_some(bottom)
}

fn codex_status(lines: &[String]) -> bool {
    // Codex draws the status bar over several rows on a narrow terminal.
    let lower = lines
        .iter()
        .take(3)
        .map_while(|line| line.strip_prefix("  "))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    (lower.contains("context ") && lower.contains("% used")) || lower.contains("% context left")
}

fn claude_rule(line: &str, cols: u16) -> bool {
    let line = line.trim_end();
    line.chars().count() >= usize::from(cols.saturating_sub(2))
        && !line.is_empty()
        && line.chars().all(|ch| ch == '─')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(lines: &[&str], cursor: (u16, u16)) -> vt100::Parser {
        let mut parser = vt100::Parser::new(30, 72, 0);
        for (row, line) in lines.iter().enumerate() {
            parser.process(format!("\x1b[{};1H{line}", row + 1).as_bytes());
        }
        let (row, col) = cursor;
        parser.process(format!("\x1b[{};{}H", row + 1, col + 1).as_bytes());
        parser
    }

    const STATUS: &str = "  GPT-6.2-Astra xhigh · Context 12% used · Fast · Workspace";

    #[test]
    fn codex_reports_rendered_edits_and_clears_after_submission() {
        let source = Some((Agent::Codex, 1));
        let mut capture = Capture::default();
        let empty = frame(
            &["\x1b[1m›\x1b[0m \x1b[2mAsk Codex\x1b[0m", "", STATUS],
            (0, 2),
        );
        assert_eq!(capture.update(source, empty.screen()), Some(None));
        capture.input(b"\x1b[I"); // Focus reporting is not typing.
        let draft = frame(
            &[
                "\x1b[1m›\x1b[0m 世界",
                "  second line",
                "",
                "  GPT-6.2-Astra xhigh · Context",
                "  12% used · Fast · Workspace",
            ],
            (1, 13),
        );
        assert_eq!(capture.update(source, draft.screen()), Some(None));
        capture.input("世界".as_bytes());
        assert_eq!(
            capture.update(source, draft.screen()),
            Some(Some("世界\nsecond line".into()))
        );
        let changed = frame(&["\x1b[1m›\x1b[0m edit", "", STATUS], (0, 4));
        assert_eq!(
            capture.update(source, changed.screen()),
            Some(Some("edit".into()))
        );
        assert_eq!(capture.update(source, empty.screen()), Some(None));
    }

    #[test]
    fn codex_uses_the_painted_box_with_a_default_status_bar() {
        let source = Some((Agent::Codex, 0));
        let mut capture = Capture::default();
        let empty = frame(
            &[
                "\x1b[48;2;30;30;30m \x1b[0m",
                "\x1b[48;2;30;30;30;1m›\x1b[22m \x1b[2mAsk Codex\x1b[0m",
                "\x1b[48;2;30;30;30m \x1b[0m",
                "  gpt-5.4 default · /work",
            ],
            (1, 2),
        );
        assert_eq!(capture.update(source, empty.screen()), Some(None));
        capture.input(b"draft");
        let draft = frame(
            &[
                "\x1b[48;2;30;30;30m \x1b[0m",
                "\x1b[48;2;30;30;30;1m›\x1b[22m draft\x1b[0m",
                "\x1b[48;2;30;30;30m  \x1b[0m",
                "\x1b[48;2;30;30;30m  last line\x1b[0m",
                "\x1b[48;2;30;30;30m \x1b[0m",
                "  gpt-5.4 default · /work",
            ],
            (1, 7),
        );
        assert_eq!(
            capture.update(source, draft.screen()),
            Some(Some("draft\n\nlast line".into()))
        );
        let menu = frame(&["\x1b[1m›\x1b[0m approval", "  enter to confirm"], (0, 2));
        assert_eq!(capture.update(source, menu.screen()), Some(None));
    }

    #[test]
    fn claude_requires_its_input_box_and_discards_placeholders_and_menus() {
        let source = Some((Agent::Claude, 1));
        let mut capture = Capture::default();
        let rule = "─".repeat(72);
        let empty = frame(
            &[
                &rule,
                "❯\u{a0}Try \"fix a test\"",
                &rule,
                "  ? for shortcuts",
            ],
            (1, 2),
        );
        let draft = frame(
            &[
                &rule,
                "❯\u{a0}edited by Claude\x1b[7m \x1b[27m\x1b[?25l",
                &rule,
                "  manual mode",
            ],
            (1, 18),
        );
        let menu = frame(
            &[&rule, "  Workspace access", "  ❯ Yes", "  Enter to confirm"],
            (2, 7),
        );
        assert_eq!(capture.update(source, menu.screen()), Some(None));
        capture.input(b"menu answer");
        assert_eq!(capture.update(source, draft.screen()), Some(None));
        capture.input(b"\x1b[200~khor paste\x1b[201~");
        assert_eq!(
            capture.update(source, draft.screen()),
            Some(Some("edited by Claude".into()))
        );
        assert_eq!(capture.update(source, menu.screen()), Some(None));
        assert_eq!(capture.update(source, empty.screen()), Some(None));
        assert_eq!(
            capture.update(Some((Agent::Claude, 2)), draft.screen()),
            Some(None)
        );
        assert_eq!(capture.update(None, draft.screen()), Some(None));
        assert_eq!(capture.update(None, draft.screen()), None);
    }

    #[test]
    fn only_known_executables_select_a_composer() {
        assert_eq!(Agent::command("codex resume"), Some(Agent::Codex));
        assert_eq!(
            Agent::command(r#"& "C:\Program Files\claude.exe" --resume"#),
            Some(Agent::Claude)
        );
        assert_eq!(Agent::command("echo claude"), None);
        assert_eq!(Agent::executable("claude-wrapper"), None);
    }
}
