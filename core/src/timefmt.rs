//! Small time helpers shared by the parsers.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Parses an RFC 3339 timestamp such as `2026-11-20T09:59:35Z` into Unix seconds.
pub fn rfc3339_to_epoch(s: &str) -> Option<i64> {
    OffsetDateTime::parse(s.trim(), &Rfc3339)
        .ok()
        .map(OffsetDateTime::unix_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zulu() {
        assert_eq!(rfc3339_to_epoch("1970-01-01T00:00:10Z"), Some(10));
        assert_eq!(
            rfc3339_to_epoch("2026-11-20T09:59:35Z"),
            Some(1_795_168_775)
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(rfc3339_to_epoch("yesterday"), None);
    }
}
