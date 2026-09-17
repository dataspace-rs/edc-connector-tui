//! Client calls of the admin workspace (v5 global resources).

use edc_connector_client::types::{
    common_expression_language::CelExpressionTestRequest, query::Query,
};

use crate::{
    components::{
        admin::{
            cached_documents::CachedDocumentEntry,
            cel_expressions::{self, CelExpressionEntry},
            dataspace_profiles::DataspaceProfileEntry,
            dcp_scopes::DcpScopeEntry,
            participants::{self, ParticipantDetail, ParticipantEntry},
            schema_validators::SchemaValidatorEntry,
        },
        resources::msg::ActionInput,
    },
    types::connector::Connector,
};

use super::App;

impl App {
    // --- DCP scopes -------------------------------------------------------------------

    pub async fn fetch_dcp_scopes(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<DcpScopeEntry>> {
        Ok(connector
            .client()
            .dcp_scopes(connector.api_version())
            .query(query)
            .await?
            .into_iter()
            .map(DcpScopeEntry::new)
            .collect())
    }

    pub async fn create_dcp_scope(
        connector: Connector,
        entry: DcpScopeEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .dcp_scopes(connector.api_version())
            .create(&entry.to_new())
            .await?;
        Ok(format!("DCP scope '{}' created", created.id()))
    }

    pub async fn update_dcp_scope(
        connector: Connector,
        entry: DcpScopeEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .dcp_scopes(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!("DCP scope '{}' updated", entry.inner().id()))
    }

    pub async fn delete_dcp_scope(
        connector: Connector,
        entry: DcpScopeEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .dcp_scopes(connector.api_version())
            .delete(entry.inner().id())
            .await?;
        Ok(format!("DCP scope '{}' deleted", entry.inner().id()))
    }

    // --- CEL expressions --------------------------------------------------------------

    pub async fn fetch_cel_expressions(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<CelExpressionEntry>> {
        Ok(connector
            .client()
            .common_expression_language(connector.api_version())
            .query(query)
            .await?
            .into_iter()
            .map(CelExpressionEntry::new)
            .collect())
    }

    pub async fn create_cel_expression(
        connector: Connector,
        entry: CelExpressionEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .common_expression_language(connector.api_version())
            .create(&entry.to_new())
            .await?;
        Ok(format!("CEL expression '{}' created", created.id()))
    }

    pub async fn update_cel_expression(
        connector: Connector,
        entry: CelExpressionEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .common_expression_language(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!("CEL expression '{}' updated", entry.inner().id()))
    }

    pub async fn delete_cel_expression(
        connector: Connector,
        entry: CelExpressionEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .common_expression_language(connector.api_version())
            .delete(entry.inner().id())
            .await?;
        Ok(format!("CEL expression '{}' deleted", entry.inner().id()))
    }

    /// Evaluates the expression against the operands and `ctx` parameters of the test form.
    pub async fn test_cel_expression(
        connector: Connector,
        entry: CelExpressionEntry,
        input: ActionInput,
    ) -> anyhow::Result<String> {
        let test = cel_expressions::parse_test(&input)?;
        let mut request = CelExpressionTestRequest::builder()
            .left_operand(entry.inner().left_operand())
            .expression(entry.inner().expression())
            .operator(test.operator)
            .right_operand(test.right_operand);
        for (k, v) in test.params {
            request = request.param(&k, v);
        }
        let response = connector
            .client()
            .common_expression_language(connector.api_version())
            .test(&request.build())
            .await?;
        match (response.evaluation_result(), response.error()) {
            (_, Some(error)) => anyhow::bail!("Evaluation failed: {}", error),
            (Some(result), None) => Ok(format!("'{}' evaluated to {}", entry.inner().id(), result)),
            (None, None) => Ok(format!("'{}' returned no result", entry.inner().id())),
        }
    }

    // --- cached documents -------------------------------------------------------------

    pub async fn fetch_cached_documents(
        connector: Connector,
        _query: Query,
    ) -> anyhow::Result<Vec<CachedDocumentEntry>> {
        Ok(connector
            .client()
            .cached_documents(connector.api_version())
            .list()
            .await?
            .into_iter()
            .map(CachedDocumentEntry::new)
            .collect())
    }

    pub async fn create_cached_document(
        connector: Connector,
        entry: CachedDocumentEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .cached_documents(connector.api_version())
            .create(&entry.to_new())
            .await?;
        Ok(format!("Cached document '{}' created", created.id()))
    }

    pub async fn update_cached_document(
        connector: Connector,
        entry: CachedDocumentEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .cached_documents(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!("Cached document '{}' updated", entry.inner().id()))
    }

    pub async fn delete_cached_document(
        connector: Connector,
        entry: CachedDocumentEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .cached_documents(connector.api_version())
            .delete(entry.inner().id())
            .await?;
        Ok(format!("Cached document '{}' deleted", entry.inner().id()))
    }

    /// Makes the connector fetch the document from its URL again.
    pub async fn refresh_cached_document(
        connector: Connector,
        entry: CachedDocumentEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .cached_documents(connector.api_version())
            .refresh(entry.inner().id())
            .await?;
        Ok(format!(
            "Cached document '{}' refreshed",
            entry.inner().id()
        ))
    }

    // --- schema validators ------------------------------------------------------------

    pub async fn fetch_schema_validators(
        connector: Connector,
        _query: Query,
    ) -> anyhow::Result<Vec<SchemaValidatorEntry>> {
        Ok(connector
            .client()
            .schema_validators(connector.api_version())
            .list()
            .await?
            .into_iter()
            .map(SchemaValidatorEntry::new)
            .collect())
    }

    pub async fn create_schema_validator(
        connector: Connector,
        entry: SchemaValidatorEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .schema_validators(connector.api_version())
            .create(&entry.to_new())
            .await?;
        Ok(format!("Schema validator '{}' created", created.id()))
    }

    pub async fn update_schema_validator(
        connector: Connector,
        entry: SchemaValidatorEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .schema_validators(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!("Schema validator '{}' updated", entry.inner().id()))
    }

    pub async fn delete_schema_validator(
        connector: Connector,
        entry: SchemaValidatorEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .schema_validators(connector.api_version())
            .delete(entry.inner().id())
            .await?;
        Ok(format!("Schema validator '{}' deleted", entry.inner().id()))
    }

    // --- dataspace profiles -----------------------------------------------------------

    pub async fn fetch_dataspace_profiles(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<DataspaceProfileEntry>> {
        Ok(connector
            .client()
            .dataspace_profiles(connector.api_version())
            .query(query)
            .await?
            .into_iter()
            .map(DataspaceProfileEntry::new)
            .collect())
    }

    pub async fn create_dataspace_profile(
        connector: Connector,
        entry: DataspaceProfileEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .dataspace_profiles(connector.api_version())
            .create(entry.inner())
            .await?;
        Ok(format!("Dataspace profile '{}' created", created.name()))
    }

    pub async fn update_dataspace_profile(
        connector: Connector,
        entry: DataspaceProfileEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .dataspace_profiles(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!(
            "Dataspace profile '{}' updated",
            entry.inner().name()
        ))
    }

    pub async fn delete_dataspace_profile(
        connector: Connector,
        entry: DataspaceProfileEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .dataspace_profiles(connector.api_version())
            .delete(entry.inner().name())
            .await?;
        Ok(format!(
            "Dataspace profile '{}' deleted",
            entry.inner().name()
        ))
    }

    // --- participants -----------------------------------------------------------------

    pub async fn fetch_participants(
        connector: Connector,
        query: Query,
    ) -> anyhow::Result<Vec<ParticipantEntry>> {
        Ok(connector
            .client()
            .participants(connector.api_version())
            .list(query.offset(), query.limit())
            .await?
            .into_iter()
            .map(ParticipantEntry::new)
            .collect())
    }

    /// The context with its profiles and configuration; a missing configuration is not an error.
    pub async fn participant_detail(
        connector: Connector,
        entry: ParticipantEntry,
    ) -> anyhow::Result<ParticipantDetail> {
        let client = connector.client();
        let version = connector.api_version();
        let id = entry.inner().id();
        let context = client.participants(version).get(id).await?;
        let profiles = client.participants(version).profiles(id).await?;
        let config = client.participant_configs(version).get(id).await.ok();
        Ok(ParticipantDetail::new(context, profiles, config))
    }

    pub async fn create_participant(
        connector: Connector,
        entry: ParticipantEntry,
    ) -> anyhow::Result<String> {
        let created = connector
            .client()
            .participants(connector.api_version())
            .create(&entry.to_new())
            .await?;
        Ok(format!("Participant '{}' created", created.id()))
    }

    pub async fn update_participant(
        connector: Connector,
        entry: ParticipantEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .participants(connector.api_version())
            .update(entry.inner())
            .await?;
        Ok(format!("Participant '{}' updated", entry.inner().id()))
    }

    pub async fn delete_participant(
        connector: Connector,
        entry: ParticipantEntry,
    ) -> anyhow::Result<String> {
        connector
            .client()
            .participants(connector.api_version())
            .delete(entry.inner().id())
            .await?;
        Ok(format!("Participant '{}' deleted", entry.inner().id()))
    }

    pub async fn associate_participant_profiles(
        connector: Connector,
        entry: ParticipantEntry,
        input: ActionInput,
    ) -> anyhow::Result<String> {
        let names = participants::parse_profiles(&input);
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        connector
            .client()
            .participants(connector.api_version())
            .associate_profiles(entry.inner().id(), &refs)
            .await?;
        Ok(format!(
            "Participant '{}' associated with [{}]",
            entry.inner().id(),
            refs.join(", ")
        ))
    }

    pub async fn save_participant_config(
        connector: Connector,
        entry: ParticipantEntry,
        input: ActionInput,
    ) -> anyhow::Result<String> {
        let config = participants::parse_config(&input)?;
        connector
            .client()
            .participant_configs(connector.api_version())
            .save(entry.inner().id(), &config)
            .await?;
        Ok(format!(
            "Config of participant '{}' saved",
            entry.inner().id()
        ))
    }
}
