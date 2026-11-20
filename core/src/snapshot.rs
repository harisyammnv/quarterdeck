//! The bearings snapshot: `bin/fm-bearings-snapshot.sh --json`, schema `fm-bearings.v1`.
//!
//! Only the fields the fleet list needs are modelled; everything else is ignored so the
//! snapshot can grow without breaking the app. Missing optional fields default to empty.

use serde::Deserialize;

use crate::timefmt::rfc3339_to_epoch;

/// The schema this parser was written against.
pub const SCHEMA: &str = "fm-bearings.v1";

#[derive(Clone, Debug, Deserialize)]
pub struct Bearings {
    pub schema: String,
    #[serde(default)]
    pub home: Option<String>,
    #[serde(default)]
    pub generated: Option<String>,
    #[serde(default)]
    pub in_flight: Vec<InFlight>,
    #[serde(default)]
    pub omitted: Vec<Omitted>,
}

/// One task row from `in_flight`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct InFlight {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub doing: Option<String>,
}

/// A surface the bounded default left out, and the flag that reveals it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Omitted {
    pub surface: String,
    #[serde(default)]
    pub reveal: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected snapshot schema {found:?}, expected {SCHEMA}")]
    Schema { found: String },
}

impl Bearings {
    /// Parses snapshot JSON and checks the schema.
    pub fn parse(json: &str) -> Result<Self, SnapshotError> {
        let b: Self = serde_json::from_str(json)?;
        if b.schema != SCHEMA {
            return Err(SnapshotError::Schema { found: b.schema });
        }
        Ok(b)
    }

    /// `generated` as Unix seconds.
    pub fn generated_epoch(&self) -> Option<i64> {
        self.generated.as_deref().and_then(rfc3339_to_epoch)
    }

    /// Omitted surfaces that concern the fleet list. With `--all-in-flight` this should be
    /// empty; anything here means the list may be incomplete and the UI says so.
    pub fn fleet_omissions(&self) -> Vec<String> {
        self.omitted
            .iter()
            .filter(|o| o.surface.contains("in_flight") || o.surface.contains("in flight"))
            .map(|o| o.surface.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/bearings.json");

    #[test]
    fn parses_real_shape() {
        let b = Bearings::parse(FIXTURE).unwrap();
        assert_eq!(b.schema, SCHEMA);
        assert_eq!(b.in_flight.len(), 6);
        let row = &b.in_flight[1];
        assert_eq!(row.id, "app-login-fix");
        assert_eq!(row.kind.as_deref(), Some("ship"));
        assert_eq!(row.state.as_deref(), Some("unknown"));
        assert_eq!(row.repo.as_deref(), Some("example-app"));
        assert_eq!(
            row.doing.as_deref(),
            Some("backend unreachable (orca endpoint state: unreadable)")
        );
        assert_eq!(b.generated_epoch(), Some(1_795_168_775));
        assert!(b.fleet_omissions().is_empty());
        assert_eq!(b.omitted.len(), 3);
    }

    #[test]
    fn rejects_other_schema() {
        let err = Bearings::parse(r#"{"schema":"fm-fleet-snapshot.v1"}"#).unwrap_err();
        assert!(matches!(err, SnapshotError::Schema { .. }));
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        let b = Bearings::parse(r#"{"schema":"fm-bearings.v1","in_flight":[{"id":"a"}]}"#).unwrap();
        assert_eq!(b.in_flight[0].kind, None);
        assert_eq!(b.generated_epoch(), None);
    }

    #[test]
    fn reports_fleet_omission() {
        let b = Bearings::parse(
            r#"{"schema":"fm-bearings.v1","omitted":[{"surface":"in_flight showing 20 of 31","reveal":"--all-in-flight"}]}"#,
        )
        .unwrap();
        assert_eq!(
            b.fleet_omissions(),
            vec!["in_flight showing 20 of 31".to_string()]
        );
    }

    #[test]
    fn rejects_truncated_json() {
        assert!(matches!(
            Bearings::parse("{\"schema\":"),
            Err(SnapshotError::Json(_))
        ));
    }
}
