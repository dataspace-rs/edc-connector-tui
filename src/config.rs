use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use derive_builder::Builder;
use edc_connector_client::EdcConnectorApiVersion;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub connectors: Vec<ConnectorConfig>,
}

impl Config {
    /// Reads and parses the config file at `path`.
    pub fn load(path: &Path) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read config file {}: {}", path.display(), e))?;
        toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("failed to parse config file {}: {}", path.display(), e))
    }

    /// Like [`Config::load`], but a missing file yields an empty configuration so the
    /// connectors can be created from the TUI and saved later.
    pub fn load_or_default(path: &Path) -> anyhow::Result<Config> {
        match fs::metadata(path) {
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(anyhow::anyhow!(
                "failed to read config file {}: {}",
                path.display(),
                e
            )),
            Ok(_) => Self::load(path),
        }
    }

    /// Serializes the configuration to `path`, replacing the file atomically.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let contents = self.to_toml()?;
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, contents)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

pub fn default_file() -> anyhow::Result<PathBuf> {
    Ok(get_app_config_path()?.join("config.toml"))
}

#[derive(Serialize, Deserialize, Debug, Clone, Builder)]
#[builder(setter(into))]
pub struct ConnectorConfig {
    name: String,
    address: String,
    #[serde(default)]
    #[builder(default)]
    api_version: ConnectorApiVersion,
    #[serde(default)]
    #[builder(default)]
    auth: AuthKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    participant_context_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorApiVersion {
    #[default]
    V3,
    V4,
    V5,
}

impl ConnectorApiVersion {
    pub const ALL: [ConnectorApiVersion; 3] = [Self::V3, Self::V4, Self::V5];

    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectorApiVersion::V3 => "v3",
            ConnectorApiVersion::V4 => "v4",
            ConnectorApiVersion::V5 => "v5",
        }
    }

    pub fn parse(s: &str) -> Option<ConnectorApiVersion> {
        Self::ALL.into_iter().find(|v| v.as_str() == s)
    }

    /// The EDRs management API only exists in v3; it was removed in v4 and v5.
    pub fn supports_edrs(&self) -> bool {
        matches!(self, ConnectorApiVersion::V3)
    }

    /// The admin resources (participants, dataspace profiles, CEL expressions, ...) are
    /// part of the v5 management API only.
    pub fn supports_admin(&self) -> bool {
        matches!(self, ConnectorApiVersion::V5)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject_token_file: Option<PathBuf>,
        /// Keyring alias holding a static subject token.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subject_token_alias: Option<String>,
        /// The `resource` parameter; defaults to the connector's `participant_context_id`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audience: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
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

    pub fn builder() -> ConnectorConfigBuilder {
        ConnectorConfigBuilder::default()
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

    const ALL_KINDS: &str = r#"
[[connectors]]
name = "none"
address = "http://localhost:1/management"

[[connectors]]
name = "token"
address = "http://localhost:2/management"
api_version = "v4"
participant_context_id = "ctx"
auth = { type = "token", token_alias = "a" }

[[connectors]]
name = "bearer"
address = "http://localhost:3/management"
auth = { type = "bearer-token", token_alias = "b" }

[[connectors]]
name = "oauth"
address = "http://localhost:4/management"
auth = { type = "oauth2", client_id = "id", token_url = "http://idp/token", secret_alias = "s" }

[[connectors]]
name = "exchange"
address = "http://localhost:5/management"
api_version = "v5"

[connectors.auth]
type = "token-exchange"
token_exchange_url = "http://jwtlet:8080/token"
subject_token_alias = "jwtlet_alias"
resource = "provider"
audience = "aud"
scopes = ["a", "b"]
"#;

    #[test]
    fn round_trips_all_auth_kinds() {
        let cfg = parse(ALL_KINDS);
        let serialized = cfg.to_toml().expect("config should serialize");
        let reparsed: Config = toml::from_str(&serialized).expect("output should parse");
        assert_eq!(
            format!("{:?}", cfg.connectors),
            format!("{:?}", reparsed.connectors),
            "{serialized}"
        );
    }

    #[test]
    fn no_auth_serializes_as_tagged_table() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
"#,
        );
        let out = cfg.to_toml().unwrap();
        assert!(out.contains("[connectors.auth]"), "{out}");
        assert!(out.contains("type = \"no-auth\""), "{out}");
        assert!(!out.contains("participant_context_id"), "{out}");
    }

    #[test]
    fn none_options_are_omitted() {
        let cfg = parse(
            r#"
[[connectors]]
name = "c"
address = "http://localhost:29193/management"
auth = { type = "token-exchange", token_exchange_url = "http://jwtlet:8080/token", subject_token_file = "/tmp/token" }
"#,
        );
        let out = cfg.to_toml().unwrap();
        assert!(out.contains("subject_token_file = \"/tmp/token\""), "{out}");
        for absent in ["subject_token_alias", "resource", "audience", "scopes"] {
            assert!(!out.contains(absent), "{absent} present in {out}");
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("edc-tui-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn load_or_default_on_missing_file_is_empty() {
        let path = temp_dir("missing").join("config.toml");
        let cfg = Config::load_or_default(&path).unwrap();
        assert!(cfg.connectors.is_empty());
    }

    #[test]
    fn save_then_load_creates_parent_dir() {
        let path = temp_dir("save").join("nested").join("config.toml");
        let cfg = parse(ALL_KINDS);
        cfg.save(&path).unwrap();
        let loaded = Config::load_or_default(&path).unwrap();
        assert_eq!(loaded.connectors.len(), 5);
        assert_eq!(loaded.connectors[4].name(), "exchange");
        assert!(!path.with_extension("toml.tmp").exists());
        let _ = fs::remove_dir_all(path.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn load_reports_parse_errors() {
        let path = temp_dir("bad").join("config.toml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "this is = not [ toml").unwrap();
        let err = match Config::load(&path) {
            Ok(_) => panic!("malformed config should not load"),
            Err(e) => e.to_string(),
        };
        assert!(err.contains("failed to parse config file"), "{err}");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn builder_fills_defaults() {
        let cfg = ConnectorConfig::builder()
            .name("c")
            .address("http://localhost/management")
            .build()
            .unwrap();
        assert_eq!(*cfg.version(), ConnectorApiVersion::V3);
        assert!(matches!(cfg.auth(), AuthKind::NoAuth));
        assert!(cfg.participant_context_id().is_none());
        assert_eq!(
            ConnectorApiVersion::parse("v5"),
            Some(ConnectorApiVersion::V5)
        );
        assert_eq!(ConnectorApiVersion::parse("v9"), None);
    }
}
