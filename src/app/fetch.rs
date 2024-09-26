use edc_connector_client::types::query::Query;

use crate::{
    components::{
        assets::AssetEntry, contract_definitions::ContractDefinitionEntry,
        contract_negotiations::ContractNegotiationEntry, policies::PolicyDefinitionEntry,
        transfer_processes::TransferProcessEntry,
    },
    types::connector::Connector,
};

use super::App;

impl App {
    pub async fn fetch_assets(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<AssetEntry>> {
        Ok(connector
            .client()
            .assets()
            .query(query)
            .await?
            .into_iter()
            .map(AssetEntry::new)
            .collect())
    }

    pub async fn fetch_contract_definitions(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<ContractDefinitionEntry>> {
        Ok(connector
            .client()
            .contract_definitions()
            .query(query)
            .await?
            .into_iter()
            .map(ContractDefinitionEntry::new)
            .collect())
    }

    pub async fn fetch_contract_negotiations(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<ContractNegotiationEntry>> {
        Ok(connector
            .client()
            .contract_negotiations()
            .query(query)
            .await?
            .into_iter()
            .map(ContractNegotiationEntry::new)
            .collect())
    }

    pub async fn fetch_transfer_processes(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<TransferProcessEntry>> {
        Ok(connector
            .client()
            .transfer_processes()
            .query(query)
            .await?
            .into_iter()
            .map(TransferProcessEntry::new)
            .collect())
    }

    pub async fn fetch_policies(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<PolicyDefinitionEntry>> {
        Ok(connector
            .client()
            .policies()
            .query(query)
            .await?
            .into_iter()
            .map(PolicyDefinitionEntry::new)
            .collect())
    }
}
