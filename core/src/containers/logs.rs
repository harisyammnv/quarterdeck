//! Container log tails: bounded, ANSI-stripped plain text.

use serde::Serialize;

use crate::timefmt::rfc3339_to_epoch;

/// Most lines kept from a tail.
pub const MAX_LINES: usize = 500;
/// Most bytes of text kept from a tail, counted after stripping.
pub const MAX_BYTES: usize = 256 * 1024;
/// Longest single line kept; the rest is cut.
pub const MAX_LINE_CHARS: usize = 2000;
/// Most raw bytes read from the engine for one tail.
pub const MAX_RAW_BYTES: usize = 1024 * 1024;

/// One chunk of log output as the engine framed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogChunk {
    pub stderr: bool,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct LogLine {
    /// Unix seconds from the engine's timestamp, when it had one.
    #[cfg_attr(feature = "specta", specta(type = Option<f64>))]
    pub at: Option<i64>,
    pub stderr: bool,
    pub text: String,
}

/// The last lines of one container's output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ContainerLogs {
    pub id: String,
    pub lines: Vec<LogLine>,
    /// Older lines were dropped to stay inside the caps.
    pub truncated: bool,
}

/// Removes ANSI escape sequences and control characters other than tab.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                // CSI: ESC [ params final-byte(0x40..=0x7e)
                Some('[') => {
                    chars.next();
                    for n in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] ... (BEL | ESC \\)
                Some(']') => {
                    chars.next();
                    while let Some(n) = chars.next() {
                        if n == '\u{7}' {
                            break;
                        }
                        if n == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                // Two-character escape.
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c == '\t' || !c.is_control() {
            out.push(c);
        }
    }
    out
}

/// Splits engine chunks into lines, reads the leading timestamp, strips escapes and applies
/// the caps, keeping the newest lines.
pub fn to_lines(chunks: &[LogChunk]) -> (Vec<LogLine>, bool) {
    let mut lines = Vec::new();
    for chunk in chunks {
        let text = String::from_utf8_lossy(&chunk.bytes);
        for raw in text.split('\n') {
            let raw = raw.trim_end_matches('\r');
            if raw.is_empty() {
                continue;
            }
            let (at, body) = match raw.split_once(' ') {
                Some((ts, rest)) => match rfc3339_to_epoch(ts) {
                    Some(at) => (Some(at), rest),
                    None => (None, raw),
                },
                None => match rfc3339_to_epoch(raw) {
                    Some(at) => (Some(at), ""),
                    None => (None, raw),
                },
            };
            let mut clean = strip_ansi(body);
            if let Some((cut, _)) = clean.char_indices().nth(MAX_LINE_CHARS) {
                clean.truncate(cut);
            }
            lines.push(LogLine {
                at,
                stderr: chunk.stderr,
                text: clean,
            });
        }
    }
    let mut truncated = false;
    if lines.len() > MAX_LINES {
        lines.drain(..lines.len() - MAX_LINES);
        truncated = true;
    }
    let mut total: usize = lines.iter().map(|l| l.text.len()).sum();
    let mut drop = 0;
    while total > MAX_BYTES && drop < lines.len() {
        total -= lines[drop].text.len();
        drop += 1;
    }
    if drop > 0 {
        lines.drain(..drop);
        truncated = true;
    }
    (lines, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(s: &str) -> LogChunk {
        LogChunk {
            stderr: false,
            bytes: s.as_bytes().to_vec(),
        }
    }

    #[test]
    fn strips_colours_titles_and_controls() {
        assert_eq!(strip_ansi("\u{1b}[1;31mERROR\u{1b}[0m boom"), "ERROR boom");
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}after"), "after");
        assert_eq!(strip_ansi("\u{1b}]8;;x\u{1b}\\link"), "link");
        assert_eq!(strip_ansi("a\u{8}b\tc\u{0}"), "ab\tc");
        assert_eq!(strip_ansi("<script>"), "<script>");
    }

    #[test]
    fn reads_timestamps_and_keeps_untimed_lines() {
        let chunks = [
            out("2026-01-02T03:04:05.123456789Z listening on 8080\n"),
            LogChunk {
                stderr: true,
                bytes: b"no timestamp here\r\n\n".to_vec(),
            },
        ];
        let (lines, truncated) = to_lines(&chunks);
        assert!(!truncated);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].at, Some(1_767_323_045));
        assert_eq!(lines[0].text, "listening on 8080");
        assert!(lines[1].stderr);
        assert_eq!(lines[1].at, None);
        assert_eq!(lines[1].text, "no timestamp here");
    }

    #[test]
    fn keeps_newest_lines_within_count_cap() {
        let text: String = (0..MAX_LINES + 20).map(|i| format!("line {i}\n")).collect();
        let (lines, truncated) = to_lines(&[out(&text)]);
        assert!(truncated);
        assert_eq!(lines.len(), MAX_LINES);
        assert_eq!(
            lines.last().unwrap().text,
            format!("line {}", MAX_LINES + 19)
        );
    }

    #[test]
    fn keeps_within_byte_and_line_length_caps() {
        let long = "x".repeat(MAX_LINE_CHARS * 2);
        let text: String = (0..300).map(|_| format!("{long}\n")).collect();
        let (lines, truncated) = to_lines(&[out(&text)]);
        assert!(truncated);
        assert!(lines.iter().all(|l| l.text.len() == MAX_LINE_CHARS));
        assert!(lines.iter().map(|l| l.text.len()).sum::<usize>() <= MAX_BYTES);
    }

    #[test]
    fn invalid_utf8_is_replaced_not_rejected() {
        let (lines, _) = to_lines(&[LogChunk {
            stderr: false,
            bytes: vec![0x66, 0xff, 0x6f],
        }]);
        assert_eq!(lines[0].text, "f\u{fffd}o");
    }
}
