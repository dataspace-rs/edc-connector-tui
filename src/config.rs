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
    auth: AuthKind,
    #[serde(default)]
    participant_context_id: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorApiVersion {
    #[default]
    V3,
    V4,
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
    #[serde(rename = "oauth2")]
    OAuth {
        client_id: String,
        token_url: String,
        secret_alias: String,
    },
}

impl AuthKind {
    pub fn kind(&self) -> &str {
        match self {
            AuthKind::NoAuth => "No auth",
            AuthKind::Token { .. } => "Token based",
            AuthKind::OAuth { .. } => "OAuth2",
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
        }
    }
}
