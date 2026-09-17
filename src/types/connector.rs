use std::fmt::Debug;

use edc_connector_client::{
    Auth, EdcConnectorApiVersion, EdcConnectorClient, OAuth2Config, SubjectToken,
    TokenExchangeConfig,
};

use crate::{
    config::{AuthKind, ConnectorConfig},
    secrets,
};

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

    /// Builds the runtime connector (client + auth) for `cfg`. Auth failures (missing
    /// keyring entries, invalid settings) do not fail the build: they are reported
    /// through [`ConnectorStatus::Custom`] and the client falls back to no auth.
    pub fn from_config(cfg: ConnectorConfig) -> anyhow::Result<Connector> {
        let (status, auth) = Self::auth_for(&cfg);
        let client = EdcConnectorClient::builder()
            .management_url(cfg.address())
            .with_auth(auth)
            .maybe_participant_context(cfg.participant_context_id())
            .build()?;
        Ok(Connector::new(cfg, client, status))
    }

    pub(crate) fn auth_for(cfg: &ConnectorConfig) -> (ConnectorStatus, Auth) {
        match cfg.auth() {
            AuthKind::NoAuth => (ConnectorStatus::Connected, Auth::NoAuth),
            AuthKind::Token { token_alias } => Self::token_auth(token_alias, Auth::api_token),
            AuthKind::BearerToken { token_alias } => {
                Self::token_auth(token_alias, Auth::bearer_token)
            }
            AuthKind::OAuth {
                client_id,
                secret_alias,
                token_url,
            } => match secrets::load(secret_alias) {
                Ok(pwd) => {
                    let cfg = OAuth2Config::builder()
                        .client_id(client_id)
                        .client_secret(pwd)
                        .token_url(token_url)
                        .build();

                    match Auth::oauth(cfg) {
                        Ok(oauth) => (ConnectorStatus::Connected, oauth),
                        Err(_) => (
                            ConnectorStatus::Custom(format!(
                                "Failed to initialize OAuth2 for alias {}",
                                secret_alias
                            )),
                            Auth::NoAuth,
                        ),
                    }
                }
                Err(_err) => (
                    ConnectorStatus::Custom(format!("Secret not found for alias {}", secret_alias)),
                    Auth::NoAuth,
                ),
            },
            AuthKind::TokenExchange {
                token_exchange_url,
                subject_token_file,
                subject_token_alias,
                resource,
                audience,
                scopes,
            } => {
                let subject_token = match (subject_token_file, subject_token_alias) {
                    (Some(file), None) => SubjectToken::file(file),
                    (None, Some(alias)) => match secrets::load(alias) {
                        Ok(token) => SubjectToken::static_token(token),
                        Err(_err) => {
                            return (
                                ConnectorStatus::Custom(format!(
                                    "Subject token not found for alias {}",
                                    alias
                                )),
                                Auth::NoAuth,
                            )
                        }
                    },
                    _ => {
                        return (
                            ConnectorStatus::Custom(
                                "token-exchange needs exactly one of subject_token_file or subject_token_alias"
                                    .to_string(),
                            ),
                            Auth::NoAuth,
                        )
                    }
                };

                let Some(resource) = resource
                    .clone()
                    .or_else(|| cfg.participant_context_id().cloned())
                else {
                    return (
                        ConnectorStatus::Custom(
                            "token-exchange needs resource or participant_context_id".to_string(),
                        ),
                        Auth::NoAuth,
                    );
                };

                let cfg = TokenExchangeConfig::builder()
                    .token_exchange_url(token_exchange_url)
                    .subject_token(subject_token)
                    .resource(resource)
                    .maybe_audience(audience.clone())
                    .maybe_scopes(scopes.clone())
                    .build();

                match Auth::token_exchange(cfg) {
                    Ok(auth) => (ConnectorStatus::Connected, auth),
                    Err(e) => (
                        ConnectorStatus::Custom(format!(
                            "Failed to initialize token exchange: {}",
                            e
                        )),
                        Auth::NoAuth,
                    ),
                }
            }
        }
    }

    fn token_auth(token_alias: &str, to_auth: fn(String) -> Auth) -> (ConnectorStatus, Auth) {
        match secrets::load(token_alias) {
            Ok(pwd) => (ConnectorStatus::Connected, to_auth(pwd)),
            Err(_err) => (
                ConnectorStatus::Custom(format!("Token not found for alias {}", token_alias)),
                Auth::NoAuth,
            ),
        }
    }

    pub fn config(&self) -> &ConnectorConfig {
        &self.config
    }

    pub fn client(&self) -> &EdcConnectorClient {
        &self.client
    }

    pub fn api_version(&self) -> EdcConnectorApiVersion {
        (*self.config.version()).into()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn connector(toml_str: &str) -> ConnectorConfig {
        let cfg: Config = toml::from_str(toml_str).expect("config should parse");
        cfg.connectors.into_iter().next().unwrap()
    }

    fn custom_status(status: &ConnectorStatus) -> &str {
        match status {
            ConnectorStatus::Custom(msg) => msg,
            ConnectorStatus::Connected => panic!("expected a custom status"),
        }
    }

    #[test]
    fn token_exchange_with_file_and_participant_context_connects() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
participant_context_id = "provider"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", subject_token_file = "/tmp/token" }
"#,
        );
        let (status, auth) = Connector::auth_for(&cfg);
        assert!(matches!(status, ConnectorStatus::Connected), "{status:?}");
        assert!(matches!(auth, Auth::TokenExchange(_)));
    }

    #[test]
    fn token_exchange_without_subject_token_source_fails() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", resource = "provider" }
"#,
        );
        let (status, auth) = Connector::auth_for(&cfg);
        assert!(custom_status(&status).contains("exactly one of"));
        assert!(matches!(auth, Auth::NoAuth));
    }

    #[test]
    fn token_exchange_with_both_subject_token_sources_fails() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", resource = "provider", subject_token_file = "/tmp/token", subject_token_alias = "alias" }
"#,
        );
        let (status, auth) = Connector::auth_for(&cfg);
        assert!(custom_status(&status).contains("exactly one of"));
        assert!(matches!(auth, Auth::NoAuth));
    }

    #[test]
    fn token_exchange_without_resource_or_participant_context_fails() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", subject_token_file = "/tmp/token" }
"#,
        );
        let (status, auth) = Connector::auth_for(&cfg);
        assert!(custom_status(&status).contains("resource or participant_context_id"));
        assert!(matches!(auth, Auth::NoAuth));
    }

    #[test]
    fn token_exchange_with_invalid_url_fails() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "token-exchange", token_exchange_url = "not a url", subject_token_file = "/tmp/token", resource = "provider" }
"#,
        );
        let (status, auth) = Connector::auth_for(&cfg);
        assert!(custom_status(&status).contains("Failed to initialize token exchange"));
        assert!(matches!(auth, Auth::NoAuth));
    }

    #[test]
    fn from_config_builds_no_auth_connector() {
        let cfg = connector(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
"#,
        );
        let connector = Connector::from_config(cfg).unwrap();
        assert!(matches!(connector.status(), ConnectorStatus::Connected));
        assert_eq!(connector.config().name(), "c");
    }
}
