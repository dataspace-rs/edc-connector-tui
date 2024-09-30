use edc_connector_client::types::contract_agreement::ContractAgreement;
use ratatui::widgets::Row;

use crate::components::resources::FieldValue;

use super::{
    resources::{msg::ResourcesMsg, DrawableResource, Field, ResourcesComponent},
    table::TableEntry,
};

#[derive(Debug, Clone)]
pub struct ContractAgreementEntry(ContractAgreement);

impl ContractAgreementEntry {
    pub fn new(contract_agreement: ContractAgreement) -> Self {
        Self(contract_agreement)
    }
}

pub type ContractAgreementsMsg = ResourcesMsg<ContractAgreementEntry, ContractAgreementEntry>;
pub type ContractAgreementsComponent =
    ResourcesComponent<ContractAgreementEntry, ContractAgreementEntry>;

impl TableEntry for ContractAgreementEntry {
    fn row(&self) -> Row {
        let policy = serde_json::to_string(self.0.policy()).unwrap();
        Row::new(vec![
            self.0.id().to_string(),
            format!("{:?}", self.0.contract_signing_date()),
            format!("{:?}", self.0.asset_id()),
            self.0.consumer_id().to_string(),
            self.0.provider_id().to_string(),
            policy,
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "SIGN_DATE",
            "ASSET_ID",
            "CONSUMER_ID",
            "PROVIDER_ID",
            "POLICY",
        ])
    }
}

impl DrawableResource for ContractAgreementEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "Contract Agreements"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string(
                "contract_sign_date",
                format!("{:?}", self.0.contract_signing_date()),
            ),
            Field::string("asset_id", self.0.asset_id()),
            Field::string("consumer_id", self.0.consumer_id()),
            Field::string("provider_id", self.0.provider_id()),
            Field::new(
                "policy".to_string(),
                FieldValue::Json(serde_json::to_string_pretty(&self.0.policy()).unwrap()),
            ),
        ]
    }
}
