use std::collections::HashMap;

use edc_connector_client::types::dcp_scope::{DcpScope, DcpScopeType, NewDcpScope};
use ratatui::widgets::Row;

use crate::{
    components::{
        resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
        table::TableEntry,
    },
    widgets::form::{
        row::RowField,
        select::SelectField,
        text::TextField,
        values::{flatten, optional, required, value},
        Form,
    },
};

use super::{enum_from, enum_str};

#[derive(Debug, Clone)]
pub struct DcpScopeEntry(DcpScope);

pub type DcpScopesMsg = ResourcesMsg<DcpScopeEntry, DcpScopeEntry>;
pub type DcpScopesComponent = ResourcesComponent<DcpScopeEntry, DcpScopeEntry>;

const KINDS: [&str; 2] = ["DEFAULT", "POLICY"];

impl DcpScopeEntry {
    pub fn new(scope: DcpScope) -> Self {
        Self(scope)
    }

    pub fn inner(&self) -> &DcpScope {
        &self.0
    }

    /// The create request: a blank id lets the connector generate one.
    pub fn to_new(&self) -> NewDcpScope {
        NewDcpScope::builder()
            .maybe_id(Some(self.0.id().to_string()).filter(|id| !id.is_empty()))
            .kind(self.0.kind().clone())
            .value(self.0.value())
            .maybe_profile(self.0.profile())
            .maybe_prefix_mapping(self.0.prefix_mapping())
            .build()
    }
}

impl TableEntry for DcpScopeEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.id().to_string(),
            enum_str(self.0.kind()),
            self.0.value().to_string(),
            self.0.profile().unwrap_or_default().to_string(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["ID", "TYPE", "VALUE", "PROFILE"])
    }
}

impl DrawableResource for DcpScopeEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "DcpScopes"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("type", enum_str(self.0.kind())),
            Field::string("value", self.0.value()),
            Field::string("profile", self.0.profile().unwrap_or_default()),
            Field::string("prefixMapping", self.0.prefix_mapping().unwrap_or_default()),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&DcpScopeEntry>) -> Form<DcpScopeEntry> {
    let target = edit.map(|e| e.0.id().to_string());
    let scope = edit.map(|e| &e.0);
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
            scope.map(|s| s.id()).unwrap_or_default(),
        ))
        .field(
            SelectField::new("kind", "Type (space)", KINDS).with_value(
                &scope
                    .map(|s| enum_str(s.kind()))
                    .unwrap_or_else(|| KINDS[0].to_string()),
            ),
        );
    head.set_selected(true);

    Form::default()
        .field(head)
        .field(TextField::plain(
            "value",
            "Value (e.g. org.eclipse.edc.vc.type:MembershipCredential:read)",
            scope.map(|s| s.value()).unwrap_or_default(),
        ))
        .field(
            RowField::default()
                .name("options")
                .field(TextField::plain(
                    "profile",
                    "Dataspace profile",
                    scope.and_then(|s| s.profile()).unwrap_or_default(),
                ))
                .field(TextField::plain(
                    "prefix_mapping",
                    "Prefix mapping",
                    scope.and_then(|s| s.prefix_mapping()).unwrap_or_default(),
                )),
        )
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the id being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<DcpScopeEntry> {
    let id = optional(values, "id");
    if let Some(target) = target {
        if id.as_deref() != Some(target) {
            anyhow::bail!("Id cannot be changed");
        }
    }
    let kind: DcpScopeType = enum_from(&value(values, "kind"), "Type")?;
    let scope = DcpScope::builder()
        .id(id.unwrap_or_default())
        .kind(kind)
        .value(required(values, "value", "Value")?)
        .maybe_profile(optional(values, "profile"))
        .maybe_prefix_mapping(optional(values, "prefix_mapping"))
        .build();
    Ok(DcpScopeEntry(scope))
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
    fn parses_a_new_scope_without_id() {
        let entry = parse_values(
            &values(&[
                ("kind", "POLICY"),
                ("value", " scope:read "),
                ("profile", ""),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.id(), "");
        assert_eq!(entry.0.kind(), &DcpScopeType::Policy);
        assert_eq!(entry.0.value(), "scope:read");
        assert_eq!(entry.0.profile(), None);
        assert_eq!(entry.to_new().id(), None);
    }

    #[test]
    fn value_is_required_and_id_is_immutable_on_edit() {
        let err = parse_values(&values(&[("kind", "DEFAULT")]), None).unwrap_err();
        assert_eq!(err.to_string(), "Value is required");

        let err = parse_values(
            &values(&[("id", "other"), ("kind", "DEFAULT"), ("value", "v")]),
            Some("s1"),
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "Id cannot be changed");

        let ok = parse_values(
            &values(&[("id", "s1"), ("kind", "DEFAULT"), ("value", "v")]),
            Some("s1"),
        )
        .unwrap();
        assert_eq!(ok.to_new().id(), Some("s1"));
    }

    #[test]
    fn edit_form_is_prefilled() {
        let scope = DcpScope::builder()
            .id("s1")
            .kind(DcpScopeType::Policy)
            .value("v")
            .profile("p")
            .build();
        let form = form(Some(&DcpScopeEntry(scope)));
        assert_eq!(form.field_value("id").as_deref(), Some("s1"));
        assert_eq!(form.field_value("kind").as_deref(), Some("POLICY"));
        assert_eq!(form.field_value("profile").as_deref(), Some("p"));
    }
}
