//! Helpers to turn the fields a [`super::Form`] hands to its confirm callback into typed values.

use std::collections::HashMap;

use serde::de::DeserializeOwned;

use super::FieldComponent;

/// Flattens rows so every leaf field is reachable by its own name.
pub(crate) fn flatten(fields: HashMap<String, FieldComponent>) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for (name, field) in fields {
        match field {
            FieldComponent::Row(row) => values.extend(flatten(row.as_map())),
            other => {
                if let Ok(value) = other.try_into() {
                    values.insert(name, value);
                }
            }
        }
    }
    values
}

/// Trimmed value of `name`, empty when the field is missing.
pub(crate) fn value(values: &HashMap<String, String>, name: &str) -> String {
    values
        .get(name)
        .map(|v| v.trim().to_string())
        .unwrap_or_default()
}

/// Trimmed value of `name`, `None` when blank.
pub(crate) fn optional(values: &HashMap<String, String>, name: &str) -> Option<String> {
    Some(value(values, name)).filter(|v| !v.is_empty())
}

/// Trimmed value of `name`, failing with `"{label} is required"` when blank.
pub(crate) fn required(
    values: &HashMap<String, String>,
    name: &str,
    label: &str,
) -> anyhow::Result<String> {
    optional(values, name).ok_or_else(|| anyhow::anyhow!("{} is required", label))
}

/// Like [`required`], and the value must be an http(s) URL.
pub(crate) fn required_url(
    values: &HashMap<String, String>,
    name: &str,
    label: &str,
) -> anyhow::Result<String> {
    let raw = required(values, name, label)?;
    let parsed = url::Url::parse(&raw)
        .map_err(|e| anyhow::anyhow!("{} is not a valid URL: {}", label, e))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!("{} must be an http(s) URL", label);
    }
    Ok(raw)
}

/// Comma separated list: entries are trimmed and blanks dropped.
pub(crate) fn list(values: &HashMap<String, String>, name: &str) -> Vec<String> {
    value(values, name)
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// JSON typed value: `None` when blank, an error naming `label` when it does not parse.
pub(crate) fn json<T: DeserializeOwned>(
    values: &HashMap<String, String>,
    name: &str,
    label: &str,
) -> anyhow::Result<Option<T>> {
    match optional(values, name) {
        None => Ok(None),
        Some(raw) => serde_json::from_str(&raw)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("{} is not valid JSON: {}", label, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn list_trims_and_drops_blanks() {
        let v = values(&[("scopes", " a, b ,, c ")]);
        assert_eq!(list(&v, "scopes"), vec!["a", "b", "c"]);
        assert!(list(&v, "missing").is_empty());
    }

    #[test]
    fn json_is_optional_and_typed() {
        let v = values(&[("props", r#"{"k": 1}"#), ("blank", "  "), ("bad", "{")]);
        let props: Option<serde_json::Value> = json(&v, "props", "Properties").unwrap();
        assert_eq!(props.unwrap()["k"], 1);
        let blank: Option<serde_json::Value> = json(&v, "blank", "Blank").unwrap();
        assert!(blank.is_none());
        let err = json::<serde_json::Value>(&v, "bad", "Bad").unwrap_err();
        assert!(err.to_string().starts_with("Bad is not valid JSON"));
    }
}
