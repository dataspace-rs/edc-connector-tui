use edc_connector_client::types::contract_definition::ContractDefinition;
use ratatui::widgets::Row;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
    table::TableEntry,
};

#[derive(Debug, Clone)]
pub struct ContractDefinitionEntry(ContractDefinition);

impl ContractDefinitionEntry {
    pub fn new(contract_definition: ContractDefinition) -> Self {
        Self(contract_definition)
    }
}

pub type ContractDefinitionsMsg = ResourcesMsg<ContractDefinitionEntry>;
pub type ContractDefinitionsComponent = ResourcesComponent<ContractDefinitionEntry>;

impl TableEntry for ContractDefinitionEntry {
    fn row(&self) -> Row {
        let asset_selector = serde_json::to_string(self.0.assets_selector()).unwrap();
        Row::new(vec![
            self.0.id().to_string(),
            self.0.access_policy_id().to_string(),
            self.0.contract_policy_id().to_string(),
            asset_selector,
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "ACCESS_POLICY_ID",
            "CONTRACT_POLICY_ID",
            "ASSETS_SELECTOR",
        ])
    }
}

impl DrawableResource for ContractDefinitionEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Contract Definitions"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("access_policy_id", self.0.access_policy_id()),
            Field::string("contract_policy_id", self.0.contract_policy_id()),
            Field::json("assets_selector", self.0.assets_selector()),
        ]
    }
}
