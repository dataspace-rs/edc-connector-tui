use std::collections::HashMap;

use edc_connector_client::types::schema_validator::{
    NewSchemaValidatorRegistration, SchemaValidatorRegistration,
};
use ratatui::widgets::Row;

use crate::{
    components::{
        resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
        table::TableEntry,
    },
    widgets::form::{
        row::RowField,
        text::TextField,
        values::{flatten, list, optional, required, required_url},
        Form,
    },
};

use super::{join, timestamp};

#[derive(Debug, Clone)]
pub struct SchemaValidatorEntry(SchemaValidatorRegistration);

pub type SchemaValidatorsMsg = ResourcesMsg<SchemaValidatorEntry, SchemaValidatorEntry>;
pub type SchemaValidatorsComponent = ResourcesComponent<SchemaValidatorEntry, SchemaValidatorEntry>;

impl SchemaValidatorEntry {
    pub fn new(registration: SchemaValidatorRegistration) -> Self {
        Self(registration)
    }

    pub fn inner(&self) -> &SchemaValidatorRegistration {
        &self.0
    }

    /// The create request: a blank id lets the connector generate one.
    pub fn to_new(&self) -> NewSchemaValidatorRegistration {
        let mut builder = NewSchemaValidatorRegistration::builder()
            .maybe_id(Some(self.0.id().to_string()).filter(|id| !id.is_empty()))
            .version(self.0.version())
            .validated_type(self.0.validated_type())
            .schema(self.0.schema());
        for profile in self.0.profiles() {
            builder = builder.profile(profile);
        }
        builder.build()
    }
}

impl TableEntry for SchemaValidatorEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.id().to_string(),
            self.0.version().to_string(),
            self.0.validated_type().to_string(),
            self.0.schema().to_string(),
            join(self.0.profiles()),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "VERSION",
            "VALIDATED TYPE",
            "SCHEMA",
            "PROFILES",
        ])
    }
}

impl DrawableResource for SchemaValidatorEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "SchemaValidators"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("version", self.0.version()),
            Field::string("validatedType", self.0.validated_type()),
            Field::string("schema", self.0.schema()),
            Field::string("profiles", join(self.0.profiles())),
            Field::string("updatedAt", timestamp(self.0.updated_at())),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&SchemaValidatorEntry>) -> Form<SchemaValidatorEntry> {
    let target = edit.map(|e| e.0.id().to_string());
    let reg = edit.map(|e| &e.0);
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
            reg.map(|r| r.id()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "version",
            "Management API version (e.g. v5)",
            reg.map(|r| r.version()).unwrap_or("v5"),
        ));
    head.set_selected(true);

    Form::default()
        .field(head)
        .field(TextField::plain(
            "validated_type",
            "Validated @type (e.g. Asset)",
            reg.map(|r| r.validated_type()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "schema",
            "Schema URL",
            reg.map(|r| r.schema()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "profiles",
            "Dataspace profiles (comma separated)",
            &reg.map(|r| join(r.profiles())).unwrap_or_default(),
        ))
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the id being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<SchemaValidatorEntry> {
    let id = optional(values, "id");
    if let Some(target) = target {
        if id.as_deref() != Some(target) {
            anyhow::bail!("Id cannot be changed");
        }
    }
    let mut builder = SchemaValidatorRegistration::builder()
        .id(id.unwrap_or_default())
        .version(required(values, "version", "Version")?)
        .validated_type(required(values, "validated_type", "Validated type")?)
        .schema(required_url(values, "schema", "Schema URL")?);
    for profile in list(values, "profiles") {
        builder = builder.profile(profile);
    }
    Ok(SchemaValidatorEntry(builder.build()))
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
    fn parses_profiles_list() {
        let entry = parse_values(
            &values(&[
                ("id", "sv"),
                ("version", "v5"),
                ("validated_type", "Asset"),
                ("schema", "https://example.org/asset.json"),
                ("profiles", "p1, p2"),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.profiles(), &["p1".to_string(), "p2".to_string()]);
        assert_eq!(entry.to_new().id(), Some("sv"));
    }

    #[test]
    fn schema_must_be_a_url() {
        let err = parse_values(
            &values(&[
                ("version", "v5"),
                ("validated_type", "Asset"),
                ("schema", "x"),
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().starts_with("Schema URL is not a valid URL"));
    }
}
