//! `SKILL.md` front matter: a YAML block between `---` lines at the top of the file.

use yaml_rust2::parser::{Event, EventReceiver, Parser};
use yaml_rust2::{Yaml, YamlLoader};

/// Front matter larger than this is not parsed.
const MAX_FRONT_MATTER: usize = 16 * 1024;
/// Front matter nested deeper than this is refused: skill headers are flat, and a deeply
/// nested value would make loading and dropping it recurse.
const MAX_DEPTH: u32 = 16;

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
    let fm = FrontMatter {
        malformed: true,
        ..FrontMatter::default()
    };
    (fm, text)
}

fn parse(yaml: &str) -> FrontMatter {
    if yaml.len() > MAX_FRONT_MATTER {
        return FrontMatter {
            malformed: true,
            ..FrontMatter::default()
        };
    }
    // The loader copies the node an alias points to, so nested aliases grow exponentially.
    // Skill front matter has no use for them; refuse any before loading.
    if has_alias(yaml) {
        return FrontMatter {
            malformed: true,
            ..FrontMatter::default()
        };
    }
    let Ok(docs) = YamlLoader::load_from_str(yaml) else {
        return FrontMatter {
            malformed: true,
            ..FrontMatter::default()
        };
    };
    let Some(doc) = docs.first() else {
        return FrontMatter::default();
    };
    if !matches!(doc, Yaml::Hash(_)) {
        return FrontMatter {
            malformed: true,
            ..FrontMatter::default()
        };
    }
    FrontMatter {
        name: scalar(&doc["name"]),
        description: scalar(&doc["description"]),
        internal: doc["metadata"]["internal"].as_bool().unwrap_or(false),
        malformed: false,
    }
}

/// True when the YAML uses an alias in any document, holds more than one document, nests
/// deeper than [MAX_DEPTH], or cannot be parsed at all. Every document is scanned, because the loader loads them all.
fn has_alias(yaml: &str) -> bool {
    #[derive(Default)]
    struct Scan {
        alias: bool,
        documents: u32,
        depth: u32,
        deepest: u32,
    }
    impl EventReceiver for Scan {
        fn on_event(&mut self, ev: Event) {
            match ev {
                Event::Alias(_) => self.alias = true,
                Event::DocumentStart => self.documents += 1,
                Event::SequenceStart(..) | Event::MappingStart(..) => {
                    self.depth += 1;
                    self.deepest = self.deepest.max(self.depth);
                }
                Event::SequenceEnd | Event::MappingEnd => {
                    self.depth = self.depth.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
    let mut seen = Scan::default();
    let parsed = Parser::new_from_str(yaml).load(&mut seen, true);
    parsed.is_err() || seen.alias || seen.documents > 1 || seen.deepest > MAX_DEPTH
}

/// A string, or a number or boolean written where a string was meant.
fn scalar(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) | Yaml::Real(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
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
        let (fm, body) = split("---\nname: x\nno end\n");
        assert!(fm.malformed);
        assert_eq!(body, "---\nname: x\nno end\n");
        let huge = format!("---\nname: x\n{}---\n", "a: b\n".repeat(10_000));
        assert!(split(&huge).0.malformed);
    }

    #[test]
    fn deep_nesting_is_refused() {
        // Balanced brackets 7000 deep fit under the size cap.
        let deep = format!(
            "---\nname: x\nv: {}{}\n---\n",
            "[".repeat(7000),
            "]".repeat(7000)
        );
        let (fm, _) = split(&deep);
        assert!(fm.malformed);
        let ok = "---\nname: x\nmetadata:\n  internal: true\n  tags: [a, b]\n---\n";
        assert!(!split(ok).0.malformed);
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
    fn aliases_are_refused_before_loading() {
        // Ten levels of nine aliases each would expand to 9^10 nodes if loaded.
        let mut yaml = String::from("---\nname: bomb\nl0: &l0 [x, x, x, x, x, x, x, x, x]\n");
        for i in 1..10 {
            let p = i - 1;
            yaml.push_str(&format!(
                "l{i}: &l{i} [*l{p}, *l{p}, *l{p}, *l{p}, *l{p}, *l{p}, *l{p}, *l{p}, *l{p}]\n"
            ));
        }
        yaml.push_str("---\n");
        let start = std::time::Instant::now();
        let (fm, _) = split(&yaml);
        assert!(fm.malformed);
        assert_eq!(fm.name, None);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        // A second document is refused whatever it holds (split ends the block at the first
        // closing line, so this guards the parser call itself).
        assert!(has_alias("name: a\n---\nname: b\n"));
        // Anchors alone are harmless.
        assert!(!split("---\nname: &n ok\n---\n").0.malformed);
    }
}
