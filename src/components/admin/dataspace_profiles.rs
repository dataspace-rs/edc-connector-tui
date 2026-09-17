use std::collections::HashMap;

use edc_connector_client::types::dataspace_profile::{
    DataspaceProfile, DataspaceProtocol, TrustedIssuer,
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
        values::{flatten, json, list, required, value},
        Form,
    },
};

use super::{compact_json, join};

#[derive(Debug, Clone)]
pub struct DataspaceProfileEntry(DataspaceProfile);

pub type DataspaceProfilesMsg = ResourcesMsg<DataspaceProfileEntry, DataspaceProfileEntry>;
pub type DataspaceProfilesComponent =
    ResourcesComponent<DataspaceProfileEntry, DataspaceProfileEntry>;

impl DataspaceProfileEntry {
    pub fn new(profile: DataspaceProfile) -> Self {
        Self(profile)
    }

    pub fn inner(&self) -> &DataspaceProfile {
        &self.0
    }
}

impl TableEntry for DataspaceProfileEntry {
    fn row(&self) -> Row<'_> {
        let protocol = self.0.protocol();
        Row::new(vec![
            self.0.name().to_string(),
            protocol.version().to_string(),
            protocol.binding().to_string(),
            protocol.path().to_string(),
            protocol.namespace().to_string(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["NAME", "VERSION", "BINDING", "PATH", "NAMESPACE"])
    }
}

impl DrawableResource for DataspaceProfileEntry {
    fn id(&self) -> &str {
        self.0.name()
    }

    fn title() -> &'static str {
        "DataspaceProfiles"
    }

    fn fields(&self) -> Vec<Field> {
        let protocol = self.0.protocol();
        vec![
            Field::string("name", self.0.name()),
            Field::string("protocol.version", protocol.version()),
            Field::string("protocol.path", protocol.path()),
            Field::string("protocol.binding", protocol.binding()),
            Field::string("protocol.namespace", protocol.namespace()),
            Field::string("jsonLdContextsUrl", join(self.0.json_ld_contexts_url())),
            Field::json("trustedIssuers", self.0.trusted_issuers()),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&DataspaceProfileEntry>) -> Form<DataspaceProfileEntry> {
    let target = edit.map(|e| e.0.name().to_string());
    let profile = edit.map(|e| &e.0);
    let protocol = profile.map(|p| p.protocol());
    let mut name = TextField::plain(
        "name",
        "Name",
        profile.map(|p| p.name()).unwrap_or_default(),
    );
    name.set_selected(true);

    Form::default()
        .field(name)
        .field(
            RowField::default()
                .name("protocol")
                .field(TextField::plain(
                    "version",
                    "Protocol version (e.g. 2025-1)",
                    protocol.map(|p| p.version()).unwrap_or_default(),
                ))
                .field(TextField::plain(
                    "path",
                    "Path (e.g. /2025-1)",
                    protocol.map(|p| p.path()).unwrap_or_default(),
                ))
                .field(TextField::plain(
                    "binding",
                    "Binding (e.g. HTTPS)",
                    protocol.map(|p| p.binding()).unwrap_or("HTTPS"),
                ))
                .field(TextField::plain(
                    "namespace",
                    "Namespace",
                    protocol.map(|p| p.namespace()).unwrap_or_default(),
                )),
        )
        .field(TextField::plain(
            "json_ld_contexts_url",
            "JSON-LD context URLs (comma separated)",
            &profile
                .map(|p| join(p.json_ld_contexts_url()))
                .unwrap_or_default(),
        ))
        .field(TextField::plain(
            "trusted_issuers",
            r#"Trusted issuers (JSON array of {"@id": did, "supportedTypes": [..]})"#,
            &profile
                .map(|p| compact_json(p.trusted_issuers()))
                .unwrap_or_default(),
        ))
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the name being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<DataspaceProfileEntry> {
    let name = required(values, "name", "Name")?;
    if let Some(target) = target {
        if name != target {
            anyhow::bail!("Name cannot be changed");
        }
    }
    let protocol = DataspaceProtocol::builder()
        .version(required(values, "version", "Protocol version")?)
        .path(value(values, "path"))
        .binding(required(values, "binding", "Binding")?)
        .namespace(value(values, "namespace"))
        .build();
    let issuers: Vec<TrustedIssuer> =
        json(values, "trusted_issuers", "Trusted issuers")?.unwrap_or_default();

    let mut builder = DataspaceProfile::builder().name(name).protocol(protocol);
    for url in list(values, "json_ld_contexts_url") {
        builder = builder.json_ld_context_url(url);
    }
    for issuer in issuers {
        builder = builder.trusted_issuer(issuer);
    }
    Ok(DataspaceProfileEntry(builder.build()))
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
    fn parses_protocol_contexts_and_issuers() {
        let entry = parse_values(
            &values(&[
                ("name", "dsp-2025"),
                ("version", "2025-1"),
                ("path", "/2025-1"),
                ("binding", "HTTPS"),
                ("namespace", "https://w3id.org/dspace/2025/1/"),
                ("json_ld_contexts_url", "https://a, https://b"),
                (
                    "trusted_issuers",
                    r#"[{"@id":"did:web:issuer","supportedTypes":["MembershipCredential"]}]"#,
                ),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.name(), "dsp-2025");
        assert_eq!(entry.0.protocol().version(), "2025-1");
        assert_eq!(entry.0.json_ld_contexts_url().len(), 2);
        assert_eq!(entry.0.trusted_issuers()[0].id(), "did:web:issuer");
        assert_eq!(
            entry.0.trusted_issuers()[0].supported_types(),
            &["MembershipCredential".to_string()]
        );
    }

    #[test]
    fn name_is_required_and_immutable() {
        let err =
            parse_values(&values(&[("version", "1"), ("binding", "HTTPS")]), None).unwrap_err();
        assert_eq!(err.to_string(), "Name is required");
        let err = parse_values(
            &values(&[("name", "b"), ("version", "1"), ("binding", "HTTPS")]),
            Some("a"),
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "Name cannot be changed");
    }
}
