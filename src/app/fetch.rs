use edc_connector_client::types::query::Query;

use crate::{
    components::{
        agreements::ContractAgreementEntry,
        assets::AssetEntry,
        contract_definitions::ContractDefinitionEntry,
        contract_negotiations::ContractNegotiationEntry,
        edrs::{EdrEntry, EdrMetadataEntry},
        policies::PolicyDefinitionEntry,
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

    pub async fn fetch_contract_agreements(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<ContractAgreementEntry>> {
        Ok(connector
            .client()
            .contract_agreements()
            .query(query)
            .await?
            .into_iter()
            .map(ContractAgreementEntry::new)
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

    pub async fn fetch_edrs(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<EdrMetadataEntry>> {
        Ok(connector
            .client()
            .edrs()
            .query(query)
            .await?
            .into_iter()
            .map(EdrMetadataEntry::new)
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

    pub async fn identity<T>(_connector: Connector, entity: T) -> anyhow::Result<T> {
        Ok(entity)
    }

    pub async fn single_edr(
        connector: Connector,
        edr_entry: EdrMetadataEntry,
    ) -> anyhow::Result<EdrEntry> {
        connector
            .client()
            .edrs()
            .get_data_address(edr_entry.id())
            .await
            .map(|data_address| EdrEntry::new(edr_entry.id().to_string(), data_address))
            .map(Ok)?
    }
}
