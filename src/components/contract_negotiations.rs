use edc_connector_client::types::contract_negotiation::ContractNegotiation;
use ratatui::widgets::Row;

use crate::components::resources::FieldValue;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
    table::TableEntry,
};

#[derive(Debug, Clone)]
pub struct ContractNegotiationEntry(ContractNegotiation);

impl ContractNegotiationEntry {
    pub fn new(contract_negotiation: ContractNegotiation) -> Self {
        Self(contract_negotiation)
    }
}

pub type ContractNegotiationMsg = ResourcesMsg<ContractNegotiationEntry, ContractNegotiationEntry>;
pub type ContractNegotiationsComponent =
    ResourcesComponent<ContractNegotiationEntry, ContractNegotiationEntry>;

impl TableEntry for ContractNegotiationEntry {
    fn row(&self) -> Row<'_> {
        let private_properties = serde_json::to_string(self.0.private_properties()).unwrap();
        Row::new(vec![
            self.0.id().to_string(),
            format!("{:?}", self.0.kind()),
            format!("{:?}", self.0.state()),
            self.0.counter_party_id().to_string(),
            self.0.contract_agreement_id().cloned().unwrap_or_default(),
            private_properties,
            self.0.created_at().to_string(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "TYPE",
            "STATE",
            "COUNTER_PARTY_ID",
            "CONTRACT_AGREEMENT_ID",
            "PRIVATE_PROPERTIES",
            "CREATED_AT",
        ])
    }
}

impl DrawableResource for ContractNegotiationEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Contract Negotiations"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("type", format!("{:?}", self.0.kind())),
            Field::string("state", format!("{:?}", self.0.state())),
            Field::string("counter_party_id", self.0.counter_party_id()),
            Field::string("counter_party_address", self.0.counter_party_address()),
            Field::string(
                "contract_agreement_id",
                self.0.contract_agreement_id().cloned().unwrap_or_default(),
            ),
            Field::new(
                "callback_addresses".to_string(),
                FieldValue::Json(
                    serde_json::to_string_pretty(&self.0.callback_addresses()).unwrap(),
                ),
            ),
            Field::new(
                "private_properties".to_string(),
                FieldValue::Json(
                    serde_json::to_string_pretty(&self.0.private_properties()).unwrap(),
                ),
            ),
            Field::string("created_at", self.0.created_at().to_string()),
        ]
    }
}
