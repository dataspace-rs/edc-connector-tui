use std::fmt::Debug;

use edc_connector_client::EdcConnectorClient;

use crate::config::ConnectorConfig;

#[derive(Clone)]
pub struct Connector {
    config: ConnectorConfig,
    client: EdcConnectorClient,
    status: ConnectorStatus,
}

#[derive(Clone, Debug)]
pub enum ConnectorStatus {
    Connected,
    Custom(String),
}

impl ConnectorStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ConnectorStatus::Connected => "connected",
            ConnectorStatus::Custom(msg) => msg,
        }
    }
}

impl Connector {
    pub fn new(
        config: ConnectorConfig,
        client: EdcConnectorClient,
        status: ConnectorStatus,
    ) -> Self {
        Self {
            config,
            client,
            status,
        }
    }

    pub fn config(&self) -> &ConnectorConfig {
        &self.config
    }

    pub fn client(&self) -> &EdcConnectorClient {
        &self.client
    }

    pub fn status(&self) -> &ConnectorStatus {
        &self.status
    }
}

impl Debug for Connector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connctor")
            .field("config", &self.config)
            .finish()
    }
}
