//! Turns captured terminal text into styled plain-text spans.
//!
//! Agent output is untrusted. A captured pane can carry escape sequences that set the window
//! title, write the clipboard (OSC 52), make hyperlinks (OSC 8), query the terminal so it
//! answers, or move the cursor over earlier lines. [`sanitize`] keeps only colour and style
//! (SGR, `ESC [ ... m`) and drops every other escape sequence and control character except
//! newline and tab; carriage returns are dropped too, so `\r\n` becomes `\n`.
//! [`spans`] then reads the SGR sequences into a small closed set of tones, so no escape
//! sequence ever reaches the webview: it gets text and a tone name, and draws them in a `<pre>`.

use serde::Serialize;

/// The longest SGR sequence kept. Real ones are short; a huge one is garbage or an attack.
const MAX_SGR_LEN: usize = 64;

/// Returns `input` with only text, newlines, tabs and SGR sequences left.
pub fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' | '\t' => out.push(c),
            '\u{1b}' => match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    let mut params = String::new();
                    let mut final_byte = None;
                    for p in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&p) {
                            final_byte = Some(p);
                            break;
                        }
                        if !('\u{20}'..='\u{3f}').contains(&p) {
                            // Not a valid CSI byte: abandon the sequence.
                            break;
                        }
                        params.push(p);
                    }
                    let is_sgr = final_byte == Some('m')
                        && params.len() <= MAX_SGR_LEN
                        && params
                            .chars()
                            .all(|p| p.is_ascii_digit() || p == ';' || p == ':');
                    if is_sgr {
                        out.push_str("\u{1b}[");
                        out.push_str(&params);
                        out.push('m');
                    }
                }
                // OSC, DCS, SOS, PM and APC run to a string terminator (BEL or ESC \\).
                Some(']' | 'P' | 'X' | '^' | '_') => {
                    chars.next();
                    skip_string(&mut chars);
                }
                // Any other escape: drop ESC, any intermediate bytes and the final byte
                // (charset selects such as `ESC ( B`, keypad modes, save and restore cursor).
                Some(_) => {
                    while chars
                        .next_if(|p| ('\u{20}'..='\u{2f}').contains(p))
                        .is_some()
                    {}
                    chars.next();
                }
                None => {}
            },
            // 8-bit C1 controls, including the one-byte CSI and OSC introducers.
            '\u{9b}' => {
                // Drop a one-byte CSI and its parameters.
                for p in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&p) {
                        break;
                    }
                }
            }
            '\u{90}' | '\u{9d}' | '\u{98}' | '\u{9e}' | '\u{9f}' => skip_string(&mut chars),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Skips a control string up to and including BEL, ST (`ESC \\` or U+009C).
fn skip_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while let Some(c) = chars.next() {
        match c {
            '\u{7}' | '\u{9c}' => return,
            '\u{1b}' => {
                if chars.peek() == Some(&'\\') {
                    chars.next();
                }
                return;
            }
            _ => {}
        }
    }
}

/// A colour from the closed set the UI styles. Terminal colours collapse onto these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Default,
    Dim,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
}

/// A run of text with one style.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Span {
    pub text: String,
    pub tone: Tone,
    pub bold: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Style {
    fg: Tone,
    dim: bool,
    bold: bool,
}

impl Style {
    fn tone(self) -> Tone {
        if self.fg == Tone::Default && self.dim {
            Tone::Dim
        } else {
            self.fg
        }
    }
}

/// Maps a basic or bright colour index (0 to 15) to a tone.
fn basic(n: u16) -> Tone {
    match n % 8 {
        1 => Tone::Red,
        2 => Tone::Green,
        3 => Tone::Yellow,
        4 => Tone::Blue,
        5 => Tone::Magenta,
        6 => Tone::Cyan,
        // Black and bright black read as secondary text; white is the default ink.
        0 => Tone::Dim,
        _ => Tone::Default,
    }
}

fn apply_sgr(style: &mut Style, params: &str) {
    // Colon sub-parameters (`38:2:r:g:b`) are one parameter; only their head matters here.
    let mut codes = params.split(';').map(|p| {
        let head = p.split(':').next().unwrap_or("");
        (head.parse::<u16>().unwrap_or(0), p.contains(':'))
    });
    while let Some((code, has_sub)) = codes.next() {
        match code {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            30..=37 => style.fg = basic(code - 30),
            90..=97 => style.fg = basic(code - 90 + 8),
            39 => style.fg = Tone::Default,
            38 if has_sub => style.fg = Tone::Default,
            38 => match codes.next().map(|c| c.0) {
                Some(5) => {
                    let n = codes.next().map_or(0, |c| c.0);
                    style.fg = if n < 16 { basic(n) } else { Tone::Default };
                }
                Some(2) => {
                    codes.next();
                    codes.next();
                    codes.next();
                    style.fg = Tone::Default;
                }
                _ => {}
            },
            // Background colours carry their own sub-parameters; skip them.
            48 if !has_sub => match codes.next().map(|c| c.0) {
                Some(5) => {
                    codes.next();
                }
                Some(2) => {
                    codes.next();
                    codes.next();
                    codes.next();
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// Splits sanitized text into lines of styled spans. Style carries across lines, as it does
/// in a terminal.
pub fn spans(sanitized: &str) -> Vec<Vec<Span>> {
    let mut style = Style::default();
    sanitized
        .split('\n')
        .map(|line| {
            let mut out: Vec<Span> = Vec::new();
            let mut push = |text: &str, style: Style| {
                if text.is_empty() {
                    return;
                }
                let (tone, bold) = (style.tone(), style.bold);
                match out.last_mut() {
                    Some(last) if last.tone == tone && last.bold == bold => {
                        last.text.push_str(text);
                    }
                    _ => out.push(Span {
                        text: text.to_string(),
                        tone,
                        bold,
                    }),
                }
            };
            let mut rest = line;
            while let Some(esc) = rest.find("\u{1b}[") {
                push(&rest[..esc], style);
                let after = &rest[esc + 2..];
                let end = after.find('m').unwrap_or(after.len());
                apply_sgr(&mut style, &after[..end]);
                rest = after.get(end + 1..).unwrap_or("");
            }
            push(rest, style);
            out
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(text: &str, tone: Tone, bold: bool) -> Span {
        Span {
            text: text.into(),
            tone,
            bold,
        }
    }

    #[test]
    fn spans_read_colours_into_tones() {
        let lines = spans(&sanitize(
            "\u{1b}[32m720 passed\u{1b}[0m in 9s\n\u{1b}[2mdim\u{1b}[22m \u{1b}[1;94mB\u{1b}[39mx",
        ));
        assert_eq!(
            lines,
            vec![
                vec![
                    sp("720 passed", Tone::Green, false),
                    sp(" in 9s", Tone::Default, false)
                ],
                vec![
                    sp("dim", Tone::Dim, false),
                    sp(" ", Tone::Default, false),
                    sp("B", Tone::Blue, true),
                    sp("x", Tone::Default, true),
                ],
            ]
        );
    }

    #[test]
    fn spans_handle_extended_colours_and_carry_style() {
        let lines = spans("\u{1b}[38;5;1mr\u{1b}[38;2;1;2;3mt\u{1b}[48;5;4mb\nnext\u{1b}[0m!");
        assert_eq!(
            lines,
            vec![
                vec![sp("r", Tone::Red, false), sp("tb", Tone::Default, false)],
                vec![sp("next!", Tone::Default, false)],
            ]
        );
        assert_eq!(spans(""), vec![Vec::<Span>::new()]);
    }

    #[test]
    fn keeps_text_newlines_tabs_and_colour() {
        let s = "\u{1b}[1;32mok\u{1b}[0m\tdone\nnext ✓";
        assert_eq!(sanitize(s), s);
        assert_eq!(sanitize("\u{1b}[38:2:1:2:3mx"), "\u{1b}[38:2:1:2:3mx");
    }

    #[test]
    fn drops_carriage_returns_and_other_controls() {
        assert_eq!(sanitize("a\r\nb\u{7}\u{8}c\u{0}"), "a\nbc");
    }

    #[test]
    fn drops_osc_title_clipboard_and_hyperlinks() {
        assert_eq!(sanitize("\u{1b}]0;pwned\u{7}x"), "x");
        assert_eq!(sanitize("\u{1b}]52;c;ZWNobyBoaQ==\u{1b}\\x"), "x");
        assert_eq!(
            sanitize("\u{1b}]8;;https://example.com\u{1b}\\link\u{1b}]8;;\u{1b}\\"),
            "link"
        );
        // Unterminated OSC swallows the rest rather than leaking it.
        assert_eq!(sanitize("a\u{1b}]0;never ends"), "a");
    }

    #[test]
    fn drops_cursor_moves_queries_and_dcs() {
        assert_eq!(sanitize("\u{1b}[2J\u{1b}[Hx\u{1b}[6n\u{1b}[?1049h"), "x");
        assert_eq!(sanitize("\u{1b}Pq#0;2;0;0;0\u{1b}\\y"), "y");
        assert_eq!(sanitize("\u{1b}(Bz\u{1b}7\u{1b}8"), "z");
        // A private-mode "m" is not SGR.
        assert_eq!(sanitize("\u{1b}[>4;2mz"), "z");
    }

    #[test]
    fn drops_eight_bit_controls() {
        assert_eq!(sanitize("\u{9b}31mx\u{9d}0;t\u{9c}y"), "xy");
    }

    #[test]
    fn caps_sgr_length() {
        let long = format!("\u{1b}[{}mx", "1;".repeat(MAX_SGR_LEN));
        assert_eq!(sanitize(&long), "x");
    }
}
