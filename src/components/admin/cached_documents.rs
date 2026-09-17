use std::collections::HashMap;

use edc_connector_client::types::cached_document::{
    CachedDocument, DocumentType, NewCachedDocument, PullStrategy,
};
use ratatui::widgets::Row;
use serde_json::Value;

use crate::{
    components::{
        resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
        table::TableEntry,
    },
    widgets::form::{
        row::RowField,
        select::SelectField,
        text::TextField,
        values::{flatten, json, optional, required_url, value},
        Form,
    },
};

use super::{compact_json, enum_from, enum_str, timestamp};

#[derive(Debug, Clone)]
pub struct CachedDocumentEntry(CachedDocument);

pub type CachedDocumentsMsg = ResourcesMsg<CachedDocumentEntry, CachedDocumentEntry>;
pub type CachedDocumentsComponent = ResourcesComponent<CachedDocumentEntry, CachedDocumentEntry>;

const DOCUMENT_TYPES: [&str; 2] = ["JSON_LD", "JSON_SCHEMA"];
const PULL_STRATEGIES: [&str; 3] = ["NEVER", "IF_NOT_PRESENT", "ALWAYS"];

impl CachedDocumentEntry {
    pub fn new(document: CachedDocument) -> Self {
        Self(document)
    }

    pub fn inner(&self) -> &CachedDocument {
        &self.0
    }

    /// The create request: a blank id lets the connector generate one.
    pub fn to_new(&self) -> NewCachedDocument {
        NewCachedDocument::builder()
            .maybe_id(Some(self.0.id().to_string()).filter(|id| !id.is_empty()))
            .url(self.0.url())
            .document_type(self.0.document_type().clone())
            .pull_strategy(self.0.pull_strategy().clone())
            .maybe_content(self.0.content().cloned())
            .build()
    }
}

impl TableEntry for CachedDocumentEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.id().to_string(),
            self.0.url().to_string(),
            enum_str(self.0.document_type()),
            enum_str(self.0.pull_strategy()),
            timestamp(self.0.updated_at()),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["ID", "URL", "TYPE", "PULL STRATEGY", "UPDATED AT"])
    }
}

impl DrawableResource for CachedDocumentEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "CachedDocuments"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("url", self.0.url()),
            Field::string("documentType", enum_str(self.0.document_type())),
            Field::string("pullStrategy", enum_str(self.0.pull_strategy())),
            Field::string("updatedAt", timestamp(self.0.updated_at())),
            Field::json("content", &self.0.content().cloned().unwrap_or(Value::Null)),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&CachedDocumentEntry>) -> Form<CachedDocumentEntry> {
    let target = edit.map(|e| e.0.id().to_string());
    let doc = edit.map(|e| &e.0);
    let id_label = if edit.is_some() {
        "Id"
    } else {
        "Id (blank = generated)"
    };
    let mut head = RowField::default()
        .name("head")
        .field(TextField::plain(
            "id",
            id_label,
            doc.map(|d| d.id()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "url",
            "URL",
            doc.map(|d| d.url()).unwrap_or_default(),
        ));
    head.set_selected(true);

    Form::default()
        .field(head)
        .field(
            RowField::default()
                .name("kind")
                .field(
                    SelectField::new("document_type", "Document type (space)", DOCUMENT_TYPES)
                        .with_value(
                            &doc.map(|d| enum_str(d.document_type()))
                                .unwrap_or_else(|| DOCUMENT_TYPES[0].to_string()),
                        ),
                )
                .field(
                    SelectField::new("pull_strategy", "Pull strategy (space)", PULL_STRATEGIES)
                        .with_value(
                            &doc.map(|d| enum_str(d.pull_strategy()))
                                .unwrap_or_else(|| PULL_STRATEGIES[0].to_string()),
                        ),
                ),
        )
        .field(TextField::plain(
            "content",
            "Content (JSON, blank = fetched from the URL)",
            &doc.and_then(|d| d.content())
                .map(compact_json)
                .unwrap_or_default(),
        ))
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the id being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<CachedDocumentEntry> {
    let id = optional(values, "id");
    if let Some(target) = target {
        if id.as_deref() != Some(target) {
            anyhow::bail!("Id cannot be changed");
        }
    }
    let document_type: DocumentType = enum_from(&value(values, "document_type"), "Document type")?;
    let pull_strategy: PullStrategy = enum_from(&value(values, "pull_strategy"), "Pull strategy")?;
    let document = CachedDocument::builder()
        .id(id.unwrap_or_default())
        .url(required_url(values, "url", "URL")?)
        .document_type(document_type)
        .pull_strategy(pull_strategy)
        .maybe_content(json::<Value>(values, "content", "Content")?)
        .build();
    Ok(CachedDocumentEntry(document))
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
    fn parses_enums_and_optional_content() {
        let entry = parse_values(
            &values(&[
                ("url", "https://example.org/ctx.jsonld"),
                ("document_type", "JSON_SCHEMA"),
                ("pull_strategy", "ALWAYS"),
                ("content", r#"{"@context": {}}"#),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.document_type(), &DocumentType::JsonSchema);
        assert_eq!(entry.0.pull_strategy(), &PullStrategy::Always);
        assert!(entry.0.content().unwrap().get("@context").is_some());
        assert_eq!(entry.to_new().id(), None);

        let blank = parse_values(
            &values(&[
                ("url", "https://example.org/x"),
                ("document_type", "JSON_LD"),
                ("pull_strategy", "NEVER"),
            ]),
            None,
        )
        .unwrap();
        assert!(blank.0.content().is_none());
    }

    #[test]
    fn url_must_be_valid() {
        let err = parse_values(
            &values(&[
                ("url", "nope"),
                ("document_type", "JSON_LD"),
                ("pull_strategy", "NEVER"),
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().starts_with("URL is not a valid URL"));
    }
}
