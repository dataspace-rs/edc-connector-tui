//! The v5 admin resources: global (participant independent) objects of an EDC-V connector.
//!
//! Every module follows the same shape: an entry newtype implementing
//! [`crate::components::table::TableEntry`] and
//! [`crate::components::resources::DrawableResource`], a `form` building the add/edit popup
//! and a pure `parse_values` turning the form values into the entry.

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub mod cached_documents;
pub mod cel_expressions;
pub mod dataspace_profiles;
pub mod dcp_scopes;
pub mod participants;
pub mod schema_validators;

/// The wire form of a serde enum value, e.g. `JSON_LD` for `DocumentType::JsonLd`.
pub fn enum_str<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => String::new(),
    }
}

/// Parses the wire form of a serde enum value (see [`enum_str`]).
pub fn enum_from<T: DeserializeOwned>(raw: &str, label: &str) -> anyhow::Result<T> {
    serde_json::from_value(Value::String(raw.to_string()))
        .map_err(|e| anyhow::anyhow!("{} '{}' is not valid: {}", label, raw, e))
}

/// Comma separated rendering of a list, the inverse of
/// [`crate::widgets::form::values::list`].
pub fn join(values: &[String]) -> String {
    values.join(", ")
}

/// Single line JSON, used to pre-fill form fields holding structured values.
pub fn compact_json<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Milliseconds since the epoch as reported by the connector, or `n/a`.
pub fn timestamp(millis: Option<i64>) -> String {
    millis
        .map(|ms| format!("{} (epoch ms)", ms))
        .unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edc_connector_client::types::cached_document::DocumentType;

    #[test]
    fn enums_round_trip_through_their_wire_form() {
        assert_eq!(enum_str(&DocumentType::JsonLd), "JSON_LD");
        assert_eq!(enum_str(&DocumentType::Other("X".to_string())), "X");
        let parsed: DocumentType = enum_from("JSON_SCHEMA", "Type").unwrap();
        assert_eq!(parsed, DocumentType::JsonSchema);
        let other: DocumentType = enum_from("FUTURE", "Type").unwrap();
        assert_eq!(other, DocumentType::Other("FUTURE".to_string()));
    }

    #[test]
    fn join_and_timestamp_render() {
        assert_eq!(join(&["a".to_string(), "b".to_string()]), "a, b");
        assert_eq!(timestamp(None), "n/a");
        assert!(timestamp(Some(5)).starts_with("5"));
    }
}
