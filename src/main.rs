use app::App;
use clap::{Parser, Subcommand};
use config::{default_file, Config, ConnectorConfig};
use edc_connector_client::{Auth, EdcConnectorClient};
use runner::Runner;
use std::{path::PathBuf, time::Duration};
use types::connector::{Connector, ConnectorStatus};
mod app;
mod components;
mod config;
mod runner;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    tui::install_panic_hook();
    let terminal = tui::init_terminal()?;

    let app = match cli.mode {
        Some(Commands::Connector { url, name, token }) => {
            init_app_single_connector(url, name, token).await
        }
        None => {
            let config = Config::parse(&cli.config.map(Ok).unwrap_or_else(default_file)?)?;
            App::init(config)
        }
    };
    let mut runner = Runner::new(Duration::from_millis(250), app);
    runner.run(terminal).await?;
    tui::restore_terminal()?;
    Ok(())
}

async fn init_app_single_connector(
    url: String,
    name: Option<String>,
    token: Option<String>,
) -> App {
    let auth = token.map(Auth::api_token).unwrap_or_else(|| Auth::NoAuth);
    let client = EdcConnectorClient::builder()
        .management_url(url.clone())
        .with_auth(auth)
        .build()
        .unwrap();

    let cfg = ConnectorConfig::new(
        name.unwrap_or_else(|| url.clone()),
        url,
        config::AuthKind::Token {
            token_alias: "unknown".to_string(),
        },
    );

    let connector = Connector::new(cfg, client, ConnectorStatus::Connected);

    App::init_with_connectors(vec![connector])
}

mod tui {
    use crossterm::{
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use ratatui::prelude::*;
    use std::{
        io::{self, stdout},
        panic,
    };

    pub fn init_terminal() -> io::Result<Terminal<impl Backend>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore_terminal() -> io::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    mode: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Connector {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        token: Option<String>,
    },
}
