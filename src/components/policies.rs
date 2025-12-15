use edc_connector_client::types::policy::PolicyDefinition;
use ratatui::widgets::Row;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, FieldValue, ResourcesComponent},
    table::TableEntry,
};

pub type PoliciesMsg = ResourcesMsg<PolicyDefinitionEntry, PolicyDefinitionEntry>;
pub type PolicyDefinitionsComponent =
    ResourcesComponent<PolicyDefinitionEntry, PolicyDefinitionEntry>;

#[derive(Debug, Clone)]
pub struct PolicyDefinitionEntry(PolicyDefinition);

impl PolicyDefinitionEntry {
    pub fn new(definition: PolicyDefinition) -> Self {
        PolicyDefinitionEntry(definition)
    }
}

impl TableEntry for PolicyDefinitionEntry {
    fn row(&self) -> ratatui::widgets::Row<'_> {
        let policy = serde_json::to_string(self.0.policy()).unwrap();
        Row::new(vec![self.0.id().to_string(), policy])
    }

    fn headers() -> ratatui::widgets::Row<'static> {
        Row::new(vec!["ID", "POLICY"])
    }
}

impl DrawableResource for PolicyDefinitionEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Policies"
    }

    fn fields(&self) -> Vec<super::resources::Field> {
        let mut fields = vec![Field::new(
            "id".to_string(),
            FieldValue::Str(self.0.id().to_string()),
        )];

        fields.push(Field::new(
            "policy".to_string(),
            FieldValue::Json(serde_json::to_string_pretty(&self.0.policy()).unwrap()),
        ));

        fields
    }
}
