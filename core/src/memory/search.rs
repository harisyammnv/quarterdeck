//! Plain-text search across every memory document.

use serde::Serialize;

use super::{MemoryIndex, MemoryKind, MAX_DOC_BYTES};

/// Queries shorter than this match too much to be useful.
pub const MIN_QUERY_CHARS: usize = 2;
const MAX_QUERY_CHARS: usize = 200;
const SNIPPET_CHARS: usize = 160;
const MAX_HITS_PER_DOC: usize = 20;

/// One matching line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemoryHit {
    pub id: String,
    pub kind: MemoryKind,
    pub title: String,
    /// 1-based line number in the document.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub line: u32,
    pub snippet: String,
    /// Matched ranges in `snippet`, as UTF-16 offsets `[start, end)` for the UI.
    #[cfg_attr(feature = "specta", specta(type = Vec<(f64, f64)>))]
    pub ranges: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MemorySearch {
    pub query: String,
    pub hits: Vec<MemoryHit>,
    /// More lines matched than `hits` holds.
    pub truncated: bool,
    /// Documents with at least one match.
    #[cfg_attr(feature = "specta", specta(type = f64))]
    pub documents: u32,
}

/// Folds one character for case-insensitive matching, keeping one char per char so offsets
/// stay aligned.
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Searches every document in `index` for `query`, case-insensitively, returning at most
/// `limit` hits.
pub async fn search(index: &MemoryIndex, query: &str, limit: usize) -> MemorySearch {
    let query: String = query.trim().chars().take(MAX_QUERY_CHARS).collect();
    let mut out = MemorySearch {
        query: query.clone(),
        ..MemorySearch::default()
    };
    let needle: Vec<char> = query.chars().map(fold).collect();
    if needle.len() < MIN_QUERY_CHARS {
        return out;
    }
    for entry in &index.view().items {
        let Ok((text, _)) = index.read_text(&entry.id, MAX_DOC_BYTES).await else {
            continue;
        };
        let mut in_doc = 0usize;
        for (n, line) in text.lines().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            let starts = find_all(&chars, &needle);
            if starts.is_empty() {
                continue;
            }
            if in_doc == 0 {
                out.documents += 1;
            }
            in_doc += 1;
            if in_doc > MAX_HITS_PER_DOC || out.hits.len() >= limit {
                out.truncated = true;
                continue;
            }
            let (snippet, ranges) = snippet(&chars, &starts, needle.len());
            out.hits.push(MemoryHit {
                id: entry.id.clone(),
                kind: entry.kind,
                title: entry.title.clone(),
                line: u32::try_from(n + 1).unwrap_or(u32::MAX),
                snippet,
                ranges,
            });
        }
    }
    out
}

fn find_all(hay: &[char], needle: &[char]) -> Vec<usize> {
    let mut starts = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return starts;
    }
    let mut i = 0;
    while i + needle.len() <= hay.len() {
        if hay[i..i + needle.len()]
            .iter()
            .zip(needle)
            .all(|(h, n)| fold(*h) == *n)
        {
            starts.push(i);
            i += needle.len();
        } else {
            i += 1;
        }
    }
    starts
}

/// Cuts a window of the line around the first match, strips list and heading markers, and
/// maps match positions to UTF-16 offsets within the window.
fn snippet(chars: &[char], starts: &[usize], len: usize) -> (String, Vec<(u32, u32)>) {
    // Leading Markdown markers carry no meaning in a snippet.
    let mut lead = chars.iter().take_while(|c| c.is_whitespace()).count();
    let rest: String = chars[lead..].iter().take(4).collect();
    for marker in ["- ", "* ", "> "] {
        if rest.starts_with(marker) {
            lead += 2;
            break;
        }
    }
    while chars.get(lead) == Some(&'#') {
        lead += 1;
    }
    while chars.get(lead).is_some_and(|c| c.is_whitespace()) {
        lead += 1;
    }
    let first = starts[0];
    let lead = lead.min(first);
    let mut from = lead;
    if first + len > from + SNIPPET_CHARS {
        from = (first + len).saturating_sub(SNIPPET_CHARS / 2).max(lead);
    }
    let to = (from + SNIPPET_CHARS).min(chars.len());
    let prefix = if from > lead { "…" } else { "" };
    let suffix = if to < chars.len() { "…" } else { "" };
    let mut snippet = String::from(prefix);
    let base: usize = prefix.encode_utf16().count();
    snippet.extend(&chars[from..to]);
    snippet.push_str(suffix);
    let u16_at = |i: usize| -> u32 {
        let n: usize = chars[from..i].iter().map(|c| c.len_utf16()).sum();
        u32::try_from(base + n).unwrap_or(u32::MAX)
    };
    let ranges = starts
        .iter()
        .filter(|&&s| s >= from && s + len <= to)
        .map(|&s| (u16_at(s), u16_at(s + len)))
        .collect();
    (snippet, ranges)
}

#[cfg(test)]
mod tests {
    use super::super::tests::fixture;
    use super::*;

    #[tokio::test]
    async fn finds_lines_case_insensitively() {
        let f = fixture();
        let idx = f.index();
        let r = search(&idx, "  CACHE ", 50).await;
        assert_eq!(r.query, "CACHE");
        let ids: Vec<_> = r.hits.iter().map(|h| (h.id.as_str(), h.line)).collect();
        // Core documents come first, then project guides and briefs.
        assert_eq!(ids, [("learnings", 5), ("brief:task-a", 3)]);
        assert_eq!(r.hits[0].snippet, "The cache needs a warm-up run.");
        assert_eq!(r.hits[0].ranges, [(4, 9)]);
        assert_eq!(r.documents, 2);
        assert!(!r.truncated);
    }

    #[tokio::test]
    async fn short_queries_find_nothing() {
        let f = fixture();
        let idx = f.index();
        assert!(search(&idx, "a", 50).await.hits.is_empty());
        assert!(search(&idx, "   ", 50).await.hits.is_empty());
    }

    #[tokio::test]
    async fn limit_marks_truncation() {
        let f = fixture();
        let idx = f.index();
        let r = search(&idx, "the", 1).await;
        assert_eq!(r.hits.len(), 1);
        assert!(r.truncated);
    }

    #[test]
    fn long_lines_are_windowed_around_the_match() {
        let line: Vec<char> = format!("{}needle{}", "x".repeat(300), "y".repeat(300))
            .chars()
            .collect();
        let starts = find_all(&line, &"needle".chars().collect::<Vec<_>>());
        let (s, r) = snippet(&line, &starts, 6);
        assert!(s.starts_with('…') && s.ends_with('…'));
        let (a, b) = r[0];
        let units: Vec<u16> = s.encode_utf16().collect();
        assert_eq!(
            String::from_utf16(&units[a as usize..b as usize]).unwrap(),
            "needle"
        );
    }

    #[test]
    fn utf16_offsets_after_astral_characters() {
        let line: Vec<char> = "🙂 find me".chars().collect();
        let starts = find_all(&line, &"find".chars().collect::<Vec<_>>());
        let (s, r) = snippet(&line, &starts, 4);
        assert_eq!(s, "🙂 find me");
        assert_eq!(r, [(3, 7)]);
    }
}
