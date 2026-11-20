//! Task metadata records: `state/<task>.meta`, one `key=value` per line.
//!
//! Only two keys are read, and only to show a provisional row before the first snapshot
//! arrives: `kind` and the basename of `project`. The snapshot is the source of truth.

/// The few facts the fleet list takes from a meta record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetaInfo {
    pub kind: Option<String>,
    pub project: Option<String>,
}

/// Parses a meta record. Unknown keys and malformed lines are ignored.
pub fn parse(content: &str) -> MetaInfo {
    let mut info = MetaInfo::default();
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "kind" => info.kind = Some(value.to_string()),
            "project" => {
                let name = value
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(value);
                if !name.is_empty() {
                    info.project = Some(name.to_string());
                }
            }
            _ => {}
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_kind_and_project_basename() {
        let m = parse(include_str!("../tests/fixtures/task.meta"));
        assert_eq!(m.kind.as_deref(), Some("ship"));
        assert_eq!(m.project.as_deref(), Some("example-app"));
    }

    #[test]
    fn ignores_noise() {
        let m = parse("no equals sign\nkind=\nproject=/a/b/\nwindow=x=y\n");
        assert_eq!(m.kind, None);
        assert_eq!(m.project.as_deref(), Some("b"));
    }
}
