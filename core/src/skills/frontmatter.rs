//! `SKILL.md` front matter: a YAML block between `---` lines at the top of the file.
//!
//! Only three values are read: `name`, `description` and `metadata.internal`. Rather than a
//! full YAML parser (which on untrusted input needs guards for aliases, multiple documents
//! and nesting depth), this is a small reader for the shapes skill headers use: top-level
//! `key: value` lines with plain, single- or double-quoted values, folded (`>`) and literal
//! (`|`) block values, and one level of nesting for `metadata`. Anything it cannot read is
//! reported as malformed; nothing it reads can expand or recurse.

/// Front matter larger than this is not parsed.
const MAX_FRONT_MATTER: usize = 16 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FrontMatter {
    pub name: Option<String>,
    pub description: Option<String>,
    /// `metadata.internal: true`.
    pub internal: bool,
    /// A front matter block was there but could not be read.
    pub malformed: bool,
}

/// Splits `text` into its front matter and the Markdown body after it. Text without front
/// matter is all body.
pub fn split(text: &str) -> (FrontMatter, &str) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(first_end) = text.find('\n') else {
        return (FrontMatter::default(), text);
    };
    if text[..first_end].trim_end() != "---" {
        return (FrontMatter::default(), text);
    }
    let rest = &text[first_end + 1..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let t = line.trim_end();
        if t == "---" || t == "..." {
            let yaml = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (parse(yaml), body);
        }
        offset += line.len();
        if offset > MAX_FRONT_MATTER {
            break;
        }
    }
    (malformed(), text)
}

fn malformed() -> FrontMatter {
    FrontMatter {
        malformed: true,
        ..FrontMatter::default()
    }
}

fn parse(yaml: &str) -> FrontMatter {
    if yaml.len() > MAX_FRONT_MATTER {
        return malformed();
    }
    let lines: Vec<&str> = yaml.lines().map(|l| l.trim_end_matches('\r')).collect();
    let mut fm = FrontMatter::default();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if is_blank_or_comment(line) {
            continue;
        }
        let Some((key, value)) = key_value(line) else {
            return malformed();
        };
        // The lines that belong to this key: everything indented under it.
        let start = i;
        while i < lines.len() && (indent(lines[i]) > 0 || lines[i].trim().is_empty()) {
            i += 1;
        }
        let block = &lines[start..i];
        let parsed = match value_of(value, block) {
            Ok(v) => v,
            Err(()) => return malformed(),
        };
        match key {
            "name" => fm.name = parsed,
            "description" => fm.description = parsed,
            "metadata" if value.is_empty() => {
                fm.internal = block
                    .iter()
                    .filter(|l| indent(l) > 0)
                    .filter_map(|l| key_value(l.trim_start()))
                    .any(|(k, v)| k == "internal" && is_true(strip_comment(v)));
            }
            _ => {}
        }
    }
    fm
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// `key: value` at the start of the line, with a plain key.
fn key_value(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = line.split_once(':')?;
    let ok = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if !ok || !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    Some((key, rest.trim()))
}

/// A plain scalar's trailing ` # comment` is not part of it.
fn strip_comment(v: &str) -> &str {
    v.find(" #").map_or(v, |at| &v[..at]).trim_end()
}

fn is_true(v: &str) -> bool {
    matches!(v, "true" | "True" | "TRUE" | "yes" | "Yes" | "on")
}

/// The string value of a key, `None` for null or a non-string (list or mapping) value, and
/// `Err` for something unreadable.
fn value_of(value: &str, block: &[&str]) -> Result<Option<String>, ()> {
    if let Some(header) = value.strip_prefix(['>', '|']) {
        let literal = value.starts_with('|');
        let header = strip_comment(header);
        if !header
            .chars()
            .all(|c| matches!(c, '-' | '+') || c.is_ascii_digit())
        {
            return Err(());
        }
        return Ok(Some(block_scalar(block, literal, header.contains('-'))));
    }
    if value.starts_with('"') {
        return double_quoted(value).map(Some);
    }
    if value.starts_with('\'') {
        return single_quoted(value).map(Some);
    }
    if value.starts_with(['[', '{']) {
        // A flow list or mapping: not a string. Unbalanced brackets are unreadable.
        return if balanced(value) { Ok(None) } else { Err(()) };
    }
    if value.starts_with(['&', '*', '!', '@', '`']) {
        return Err(());
    }
    if value.is_empty() {
        // An empty value, a nested mapping or list (not strings), or a plain string that
        // starts on the next, indented line.
        let first = block.iter().map(|l| l.trim()).find(|t| !t.is_empty());
        match first {
            None => return Ok(None),
            Some(t) if t.starts_with("- ") || t == "-" || key_value(t).is_some() => {
                return Ok(None)
            }
            Some(_) => {}
        }
    }
    // A plain value, possibly continued on indented lines.
    let mut out = strip_comment(value).to_string();
    for l in block {
        let t = l.trim();
        if !t.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(strip_comment(t));
        }
    }
    Ok(match out.as_str() {
        "~" | "null" | "Null" | "NULL" => None,
        _ => Some(out),
    })
}

fn block_scalar(block: &[&str], literal: bool, strip: bool) -> String {
    let base = block
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| indent(l))
        .min()
        .unwrap_or(0);
    let lines: Vec<&str> = block
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                ""
            } else {
                &l[base.min(l.len())..]
            }
        })
        .collect();
    let mut out = String::new();
    if literal {
        out = lines.join("\n");
    } else {
        // Folded: lines join with a space; a blank line is a line break.
        let mut pending_break = false;
        for l in &lines {
            if l.is_empty() {
                pending_break = true;
                continue;
            }
            if pending_break && !out.is_empty() {
                out.push('\n');
            } else if !out.is_empty() {
                out.push(' ');
            }
            pending_break = false;
            out.push_str(l);
        }
    }
    let mut out = out.trim_end_matches('\n').to_string();
    if !strip && !out.is_empty() {
        out.push('\n');
    }
    out
}

fn double_quoted(value: &str) -> Result<String, ()> {
    let mut out = String::new();
    let mut chars = value[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(out),
            '\\' => match chars.next().ok_or(())? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            },
            c => out.push(c),
        }
    }
    Err(())
}

fn single_quoted(value: &str) -> Result<String, ()> {
    let mut out = String::new();
    let mut chars = value[1..].chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                chars.next();
                out.push('\'');
            } else {
                return Ok(out);
            }
        } else {
            out.push(c);
        }
    }
    Err(())
}

fn balanced(value: &str) -> bool {
    let mut depth = 0i32;
    for c in strip_comment(value).chars() {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            _ => {}
        }
        if depth < 0 {
            return false;
        }
    }
    depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_scalars_and_quotes() {
        let (fm, body) = split(
            "---\nname: \"guide\"\ndescription: >-\n  Load before editing\n  shared files.\nuser-invocable: false\nmetadata:\n  internal: true\n---\n# Body\n",
        );
        assert_eq!(fm.name.as_deref(), Some("guide"));
        assert_eq!(
            fm.description.as_deref(),
            Some("Load before editing shared files.")
        );
        assert!(fm.internal);
        assert!(!fm.malformed);
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn literal_block_and_crlf() {
        let (fm, body) = split("---\r\nname: a\r\ndescription: |\r\n  one\r\n  two\r\n---\r\nbody");
        assert_eq!(fm.name.as_deref(), Some("a"));
        assert_eq!(fm.description.as_deref(), Some("one\ntwo\n"));
        assert_eq!(body, "body");
    }

    #[test]
    fn plain_values_quotes_and_comments() {
        let (fm, _) = split(
            "---\n# a comment\nname: it's-fine # trailing\ndescription: Use when a\n  change needs review.\n---\n",
        );
        assert_eq!(fm.name.as_deref(), Some("it's-fine"));
        assert_eq!(
            fm.description.as_deref(),
            Some("Use when a change needs review.")
        );
        let (fm, _) = split("---\nname: 'it''s'\ndescription: \"say \\\"hi\\\"\"\n---\n");
        assert_eq!(fm.name.as_deref(), Some("it's"));
        assert_eq!(fm.description.as_deref(), Some("say \"hi\""));
        let (fm, _) =
            split("---\ndescription: >\n  First line\n  goes on.\n\n  Second paragraph.\n---\n");
        assert_eq!(
            fm.description.as_deref(),
            Some("First line goes on.\nSecond paragraph.\n")
        );
    }

    #[test]
    fn a_plain_value_can_start_on_the_next_line() {
        let (fm, _) = split(
            "---\nname: patterns\ndescription:\n  Composition patterns that scale. Use when\n  refactoring components.\nlicense: MIT\n---\n",
        );
        assert_eq!(
            fm.description.as_deref(),
            Some("Composition patterns that scale. Use when refactoring components.")
        );
        assert!(!fm.malformed);
    }

    #[test]
    fn other_keys_and_nesting_are_ignored() {
        let (fm, _) = split(
            "---\nname: x\nallowed-tools:\n  - Read\n  - Grep\nmetadata:\n  internal: false\n  tags: [a, b]\nmodel: opus\n---\n",
        );
        assert_eq!(fm.name.as_deref(), Some("x"));
        assert!(!fm.internal);
        assert!(!fm.malformed);
    }

    #[test]
    fn no_front_matter_is_all_body() {
        let (fm, body) = split("# Title\n---\nname: x\n---\n");
        assert_eq!(fm, FrontMatter::default());
        assert_eq!(body, "# Title\n---\nname: x\n---\n");
        assert_eq!(split("").1, "");
        assert_eq!(split("---").1, "---");
    }

    #[test]
    fn malformed_and_unclosed() {
        assert!(split("---\nname: [oops\n---\nbody").0.malformed);
        assert!(split("---\n- a list\n---\n").0.malformed);
        assert!(split("---\nname: \"open\n---\n").0.malformed);
        assert!(split("---\nnot a key line\n---\n").0.malformed);
        let (fm, body) = split("---\nname: x\nno end\n");
        assert!(fm.malformed);
        assert_eq!(body, "---\nname: x\nno end\n");
        let huge = format!("---\nname: x\n{}---\n", "a: b\n".repeat(10_000));
        assert!(split(&huge).0.malformed);
    }

    #[test]
    fn odd_scalars() {
        let (fm, _) = split("---\nname: 42\ndescription: true\n---\n");
        assert_eq!(fm.name.as_deref(), Some("42"));
        assert_eq!(fm.description.as_deref(), Some("true"));
        let (fm, _) = split("---\nname: ~\ndescription: [a]\n---\n");
        assert_eq!(fm.name, None);
        assert_eq!(fm.description, None);
    }

    #[test]
    fn anchors_aliases_and_deep_nesting_never_expand() {
        // What a YAML loader would expand exponentially or recurse into is refused or left as
        // text here, in time linear in the input.
        let mut yaml = String::from("---\nname: bomb\nl0: &l0 [x, x, x, x, x, x, x, x, x]\n");
        for i in 1..10 {
            let p = i - 1;
            yaml.push_str(&format!("l{i}: &l{i} [*l{p}, *l{p}, *l{p}]\n"));
        }
        yaml.push_str(&format!(
            "v: {}{}\n---\n",
            "[".repeat(7000),
            "]".repeat(7000)
        ));
        let start = std::time::Instant::now();
        let (fm, _) = split(&yaml);
        assert!(fm.malformed);
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
        // Deep brackets on an ignored key are just balanced text.
        let deep = format!(
            "---\nname: ok\nv: {}{}\n---\n",
            "[".repeat(7000),
            "]".repeat(7000)
        );
        let (fm, _) = split(&deep);
        assert_eq!(fm.name.as_deref(), Some("ok"));
        assert!(split("---\nname: *alias\n---\n").0.malformed);
    }
}
