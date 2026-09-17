use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

use edc_connector_client::EdcConnectorApiVersion;
use serde::Deserialize;

pub fn get_app_config_path() -> anyhow::Result<std::path::PathBuf> {
    let mut path = if cfg!(target_os = "macos") {
        dirs_next::home_dir().map(|h| h.join(".config"))
    } else {
        dirs_next::config_dir()
    }
    .ok_or_else(|| anyhow::anyhow!("failed to find os config dir."))?;

    path.push("edc-connector-tui");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub connectors: Vec<ConnectorConfig>,
}

impl Config {
    pub fn parse(path: &PathBuf) -> anyhow::Result<Config> {
        let file = File::open(path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;

        let config: Result<Config, toml::de::Error> = toml::from_str(&contents);
        match config {
            Ok(config) => Ok(config),
            Err(e) => panic!("fail to parse config file: {}", e),
        }
    }
}

pub fn default_file() -> anyhow::Result<PathBuf> {
    Ok(get_app_config_path()?.join("config.toml"))
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConnectorConfig {
    name: String,
    address: String,
    #[serde(default)]
    api_version: ConnectorApiVersion,
    #[serde(default)]
    auth: AuthKind,
    #[serde(default)]
    participant_context_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorApiVersion {
    #[default]
    V3,
    V4,
    V5,
}

impl ConnectorApiVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorApiVersion::V3 => "v3",
            ConnectorApiVersion::V4 => "v4",
            ConnectorApiVersion::V5 => "v5",
        }
    }

    /// The EDRs management API only exists in v3; it was removed in v4 and v5.
    pub fn supports_edrs(&self) -> bool {
        matches!(self, ConnectorApiVersion::V3)
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub enum AuthKind {
    #[default]
    NoAuth,
    Token {
        token_alias: String,
    },
    BearerToken {
        token_alias: String,
    },
    #[serde(rename = "oauth2")]
    OAuth {
        client_id: String,
        token_url: String,
        secret_alias: String,
    },
    /// OAuth2 Token Exchange (RFC 8693): a workload credential (the subject token) is
    /// exchanged at a broker for a JWT that is sent as `Authorization: Bearer`.
    TokenExchange {
        token_exchange_url: String,
        /// Path to a file holding the subject token, re-read on every exchange.
        #[serde(default)]
        subject_token_file: Option<PathBuf>,
        /// Keyring alias holding a static subject token.
        #[serde(default)]
        subject_token_alias: Option<String>,
        /// The `resource` parameter; defaults to the connector's `participant_context_id`.
        #[serde(default)]
        resource: Option<String>,
        #[serde(default)]
        audience: Option<String>,
        #[serde(default)]
        scopes: Option<Vec<String>>,
    },
}

impl AuthKind {
    pub fn kind(&self) -> &str {
        match self {
            AuthKind::NoAuth => "No auth",
            AuthKind::Token { .. } => "Token based",
            AuthKind::BearerToken { .. } => "Bearer token",
            AuthKind::OAuth { .. } => "OAuth2",
            AuthKind::TokenExchange { .. } => "Token exchange",
        }
    }
}

impl ConnectorConfig {
    pub fn new(name: String, address: String, auth: AuthKind) -> Self {
        Self {
            name,
            address,
            auth,
            api_version: ConnectorApiVersion::V3,
            participant_context_id: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn auth(&self) -> &AuthKind {
        &self.auth
    }

    pub fn version(&self) -> &ConnectorApiVersion {
        &self.api_version
    }

    pub fn participant_context_id(&self) -> Option<&String> {
        self.participant_context_id.as_ref()
    }
}

impl From<ConnectorApiVersion> for EdcConnectorApiVersion {
    fn from(version: ConnectorApiVersion) -> Self {
        match version {
            ConnectorApiVersion::V3 => EdcConnectorApiVersion::V3,
            ConnectorApiVersion::V4 => EdcConnectorApiVersion::V4,
            ConnectorApiVersion::V5 => EdcConnectorApiVersion::V5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_str: &str) -> Config {
        toml::from_str(toml_str).expect("config should parse")
    }

    #[test]
    fn auth_defaults_to_no_auth_when_omitted() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
"#,
        );
        assert!(matches!(cfg.connectors[0].auth(), AuthKind::NoAuth));
    }

    #[test]
    fn parses_oauth2() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "oauth2", client_id = "id", token_url = "http://idp/token", secret_alias = "alias" }
"#,
        );
        match cfg.connectors[0].auth() {
            AuthKind::OAuth {
                client_id,
                token_url,
                secret_alias,
            } => {
                assert_eq!(client_id, "id");
                assert_eq!(token_url, "http://idp/token");
                assert_eq!(secret_alias, "alias");
            }
            other => panic!("unexpected auth: {other:?}"),
        }
    }

    #[test]
    fn parses_token_exchange_with_file() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
participant_context_id = "provider"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", subject_token_file = "/var/run/secrets/jwtlet/token" }
"#,
        );
        match cfg.connectors[0].auth() {
            AuthKind::TokenExchange {
                token_exchange_url,
                subject_token_file,
                subject_token_alias,
                resource,
                audience,
                scopes,
            } => {
                assert_eq!(token_exchange_url, "http://jwtlet:8080/token");
                assert_eq!(
                    subject_token_file.as_deref(),
                    Some(std::path::Path::new("/var/run/secrets/jwtlet/token"))
                );
                assert!(subject_token_alias.is_none());
                assert!(resource.is_none());
                assert!(audience.is_none());
                assert!(scopes.is_none());
            }
            other => panic!("unexpected auth: {other:?}"),
        }
        assert_eq!(cfg.connectors[0].auth().kind(), "Token exchange");
    }

    #[test]
    fn parses_token_exchange_with_alias_and_options() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"

[connectors.auth]
type = "token-exchange"
token_exchange_url = "http://jwtlet:8080/token"
subject_token_alias = "jwtlet_alias"
resource = "provider"
audience = "my-audience"
scopes = ["management-api:assets:read"]
"#,
        );
        match cfg.connectors[0].auth() {
            AuthKind::TokenExchange {
                subject_token_file,
                subject_token_alias,
                resource,
                audience,
                scopes,
                ..
            } => {
                assert!(subject_token_file.is_none());
                assert_eq!(subject_token_alias.as_deref(), Some("jwtlet_alias"));
                assert_eq!(resource.as_deref(), Some("provider"));
                assert_eq!(audience.as_deref(), Some("my-audience"));
                assert_eq!(
                    scopes.as_deref(),
                    Some(&["management-api:assets:read".to_string()][..])
                );
            }
            other => panic!("unexpected auth: {other:?}"),
        }
    }
}
