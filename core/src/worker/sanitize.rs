//! Makes captured terminal text safe to hand to a terminal emulator in the webview.
//!
//! Agent output is untrusted. A captured pane can carry escape sequences that set the window
//! title, write the clipboard (OSC 52), make hyperlinks (OSC 8), query the terminal so it
//! answers, or move the cursor over earlier lines. Only colour and style (SGR, `ESC [ ... m`)
//! survive here; every other escape sequence and control character is dropped, except
//! newline and tab. Carriage returns are dropped too, so `\r\n` becomes `\n`.

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

#[cfg(test)]
mod tests {
    use super::*;

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
