use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

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
    auth: AuthKind,
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
}

impl AuthKind {
    pub fn kind(&self) -> &str {
        match self {
            AuthKind::NoAuth => "No auth",
            AuthKind::Token { .. } => "Token based",
        }
    }
}

impl ConnectorConfig {
    pub fn new(name: String, address: String, auth: AuthKind) -> Self {
        Self {
            name,
            address,
            auth,
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
}
