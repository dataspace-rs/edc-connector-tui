//! Access to the system keyring where connector secrets (API tokens, OAuth2 client
//! secrets, subject tokens) are stored under the `edc-connector-tui` service.

pub const SERVICE: &str = "edc-connector-tui";

/// Reads the secret stored under `alias`.
pub fn load(alias: &str) -> keyring::Result<String> {
    keyring::Entry::new(SERVICE, alias).and_then(|entry| entry.get_password())
}

/// Stores `secret` under `alias`, replacing any existing value.
pub fn store(alias: &str, secret: &str) -> anyhow::Result<()> {
    keyring::Entry::new(SERVICE, alias)?.set_password(secret)?;
    Ok(())
}
