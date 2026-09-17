use std::collections::HashMap;

use edc_connector_client::types::{
    dataspace_profile::DataspaceProfile,
    participants::{
        NewParticipantContext, ParticipantContext, ParticipantContextConfig,
        ParticipantContextState,
    },
};
use ratatui::widgets::Row;
use serde_json::{Map, Value};

use crate::{
    components::{
        resources::{
            msg::{ActionInput, ResourcesMsg},
            DrawableResource, Field, ResourcesComponent,
        },
        table::TableEntry,
    },
    widgets::form::{
        row::RowField,
        select::SelectField,
        text::TextField,
        values::{flatten, json, list, optional, required, value},
        Form,
    },
};

use super::{compact_json, enum_from, enum_str, join};

#[derive(Debug, Clone)]
pub struct ParticipantEntry(ParticipantContext);

/// The detail view: the context with its associated profiles and configuration.
#[derive(Debug, Clone)]
pub struct ParticipantDetail {
    context: ParticipantContext,
    profiles: Vec<DataspaceProfile>,
    config: Option<ParticipantContextConfig>,
}

pub type ParticipantsMsg = ResourcesMsg<ParticipantEntry, ParticipantDetail>;
pub type ParticipantsComponent = ResourcesComponent<ParticipantEntry, ParticipantDetail>;

const STATES: [&str; 3] = ["CREATED", "ACTIVATED", "DEACTIVATED"];

impl ParticipantEntry {
    pub fn new(context: ParticipantContext) -> Self {
        Self(context)
    }

    pub fn inner(&self) -> &ParticipantContext {
        &self.0
    }

    /// The create request: a blank id lets the connector generate one.
    pub fn to_new(&self) -> NewParticipantContext {
        let mut builder = NewParticipantContext::builder()
            .maybe_id(Some(self.0.id().to_string()).filter(|id| !id.is_empty()))
            .identity(self.0.identity());
        for (k, v) in self.0.properties().iter() {
            builder = builder.property(k, v.0.clone());
        }
        builder.build()
    }
}

impl ParticipantDetail {
    pub fn new(
        context: ParticipantContext,
        profiles: Vec<DataspaceProfile>,
        config: Option<ParticipantContextConfig>,
    ) -> Self {
        Self {
            context,
            profiles,
            config,
        }
    }

    fn profile_names(&self) -> Vec<String> {
        self.profiles.iter().map(|p| p.name().to_string()).collect()
    }
}

impl TableEntry for ParticipantEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.id().to_string(),
            self.0.identity().to_string(),
            enum_str(self.0.state()),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec!["ID", "IDENTITY", "STATE"])
    }
}

impl DrawableResource for ParticipantDetail {
    fn id(&self) -> &str {
        self.context.id()
    }

    fn title() -> &'static str {
        "Participants"
    }

    fn fields(&self) -> Vec<Field> {
        let empty = HashMap::new();
        let entries = self.config.as_ref().map(|c| c.entries()).unwrap_or(&empty);
        let private_entries = self
            .config
            .as_ref()
            .map(|c| c.private_entries())
            .unwrap_or(&empty);
        vec![
            Field::string("id", self.context.id()),
            Field::string("identity", self.context.identity()),
            Field::string("state", enum_str(self.context.state())),
            Field::string("profiles", join(&self.profile_names())),
            Field::json("properties", self.context.properties()),
            Field::json("config.entries", entries),
            Field::json("config.privateEntries", private_entries),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&ParticipantEntry>) -> Form<ParticipantEntry> {
    let target = edit.map(|e| e.0.id().to_string());
    let ctx = edit.map(|e| &e.0);
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
            ctx.map(|c| c.id()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "identity",
            "Identity (e.g. did:web:participant)",
            ctx.map(|c| c.identity()).unwrap_or_default(),
        ));
    head.set_selected(true);

    Form::default()
        .field(head)
        .field(
            SelectField::new("state", "State (space; applied on edit only)", STATES).with_value(
                &ctx.map(|c| enum_str(c.state()))
                    .unwrap_or_else(|| STATES[0].to_string()),
            ),
        )
        .field(TextField::plain(
            "properties",
            "Properties (JSON object)",
            &ctx.map(|c| compact_json(c.properties()))
                .unwrap_or_default(),
        ))
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the id being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<ParticipantEntry> {
    let id = optional(values, "id");
    if let Some(target) = target {
        if id.as_deref() != Some(target) {
            anyhow::bail!("Id cannot be changed");
        }
    }
    let state: ParticipantContextState = enum_from(&value(values, "state"), "State")?;
    let properties: Map<String, Value> =
        json(values, "properties", "Properties")?.unwrap_or_default();
    let mut builder = ParticipantContext::builder()
        .id(id.unwrap_or_default())
        .identity(required(values, "identity", "Identity")?)
        .state(state);
    for (k, v) in properties {
        builder = builder.property(&k, v);
    }
    Ok(ParticipantEntry(builder.build()))
}

/// The "associate profiles" form, pre-filled from the detail view when it is loaded.
pub fn profiles_form(
    _: &ParticipantEntry,
    detail: Option<&ParticipantDetail>,
) -> Form<ActionInput> {
    let mut field = TextField::plain(
        "profiles",
        "Dataspace profile names (comma separated)",
        &detail.map(|d| join(&d.profile_names())).unwrap_or_default(),
    );
    field.set_selected(true);
    Form::default()
        .field(field)
        .on_confirm(|fields| Ok(flatten(fields)))
}

pub(crate) fn parse_profiles(values: &ActionInput) -> Vec<String> {
    list(values, "profiles")
}

/// The "edit config" form, pre-filled from the detail view when it is loaded.
pub fn config_form(_: &ParticipantEntry, detail: Option<&ParticipantDetail>) -> Form<ActionInput> {
    let config = detail.and_then(|d| d.config.as_ref());
    let mut row = RowField::default()
        .name("config")
        .field(TextField::plain(
            "entries",
            "Entries (JSON object of strings)",
            &config
                .map(|c| compact_json(c.entries()))
                .unwrap_or_default(),
        ))
        .field(TextField::plain(
            "private_entries",
            "Private entries (JSON object of strings)",
            &config
                .map(|c| compact_json(c.private_entries()))
                .unwrap_or_default(),
        ));
    row.set_selected(true);
    Form::default()
        .field(row)
        .on_confirm(|fields| Ok(flatten(fields)))
}

pub(crate) fn parse_config(values: &ActionInput) -> anyhow::Result<ParticipantContextConfig> {
    let entries: HashMap<String, String> = json(values, "entries", "Entries")?.unwrap_or_default();
    let private_entries: HashMap<String, String> =
        json(values, "private_entries", "Private entries")?.unwrap_or_default();
    Ok(ParticipantContextConfig::builder()
        .entries(entries)
        .private_entries(private_entries)
        .build())
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
    fn parses_state_and_properties() {
        let entry = parse_values(
            &values(&[
                ("identity", "did:web:p"),
                ("state", "ACTIVATED"),
                ("properties", r#"{"k":"v"}"#),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.identity(), "did:web:p");
        assert_eq!(entry.0.state(), &ParticipantContextState::Activated);
        assert_eq!(
            entry.0.property::<String>("k").unwrap().as_deref(),
            Some("v")
        );
        assert_eq!(entry.to_new().id(), None);
        assert!(parse_values(&values(&[("state", "CREATED")]), None).is_err());
    }

    #[test]
    fn action_inputs_parse() {
        assert_eq!(
            parse_profiles(&values(&[("profiles", "a, b")])),
            vec!["a", "b"]
        );
        let config = parse_config(&values(&[
            ("entries", r#"{"edc.participant.id":"p"}"#),
            ("private_entries", ""),
        ]))
        .unwrap();
        assert_eq!(
            config
                .entries()
                .get("edc.participant.id")
                .map(String::as_str),
            Some("p")
        );
        assert!(config.private_entries().is_empty());
        assert!(parse_config(&values(&[("entries", r#"{"k": 1}"#)])).is_err());
    }

    #[test]
    fn config_form_is_prefilled_from_the_detail() {
        let context = ParticipantContext::builder()
            .id("p")
            .identity("did:web:p")
            .build();
        let config = ParticipantContextConfig::builder()
            .entries(HashMap::from([("a".to_string(), "1".to_string())]))
            .build();
        let detail = ParticipantDetail::new(context.clone(), vec![], Some(config));
        let form = config_form(&ParticipantEntry(context.clone()), Some(&detail));
        assert_eq!(form.field_value("entries").as_deref(), Some(r#"{"a":"1"}"#));
        let blank = config_form(&ParticipantEntry(context), None);
        assert_eq!(blank.field_value("entries").as_deref(), Some(""));
    }
}
