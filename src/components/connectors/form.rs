use std::collections::HashMap;

use ratatui::{layout::Rect, Frame};

use crate::{
    components::{Component, ComponentEvent, ComponentMsg, ComponentReturn},
    config::{AuthKind, ConnectorApiVersion, ConnectorConfig},
    widgets::{
        form::{
            msg::FormMsg,
            row::RowField,
            select::SelectField,
            text::TextField,
            values::{flatten, list, optional, required, required_url, value},
            FieldComponent, Form,
        },
        popup,
    },
};

/// Number of rows that do not depend on the auth type: `[name | address]` and
/// `[api_version | participant_context_id | auth_type]`.
const BASE_FIELDS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AuthType {
    NoAuth,
    Token,
    BearerToken,
    OAuth2,
    TokenExchange,
}

impl AuthType {
    pub const ALL: [AuthType; 5] = [
        Self::NoAuth,
        Self::Token,
        Self::BearerToken,
        Self::OAuth2,
        Self::TokenExchange,
    ];

    /// Same strings as the `type` tag in the config file.
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthType::NoAuth => "no-auth",
            AuthType::Token => "token",
            AuthType::BearerToken => "bearer-token",
            AuthType::OAuth2 => "oauth2",
            AuthType::TokenExchange => "token-exchange",
        }
    }

    pub fn parse(s: &str) -> Option<AuthType> {
        Self::ALL.into_iter().find(|t| t.as_str() == s)
    }

    pub fn of(kind: &AuthKind) -> AuthType {
        match kind {
            AuthKind::NoAuth => AuthType::NoAuth,
            AuthKind::Token { .. } => AuthType::Token,
            AuthKind::BearerToken { .. } => AuthType::BearerToken,
            AuthKind::OAuth { .. } => AuthType::OAuth2,
            AuthKind::TokenExchange { .. } => AuthType::TokenExchange,
        }
    }
}

/// What the form produces on confirm.
#[derive(Debug)]
pub struct ConnectorFormOutput {
    pub config: ConnectorConfig,
    /// `(alias, secret)` pairs to store in the keyring; only filled-in secrets are listed.
    pub secrets: Vec<(String, String)>,
    /// Name of the connector being edited, `None` when adding a new one.
    pub target: Option<String>,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ConnectorFormMsg {
    Local(FormMsg<ConnectorFormOutput>),
    Submitted(Box<ConnectorFormOutput>),
}

pub struct ConnectorForm {
    form: Form<ConnectorFormOutput>,
    auth_type: AuthType,
    target: Option<String>,
}

impl ConnectorForm {
    /// `existing_names` are the names of the other connectors (used to reject duplicates);
    /// `edit` pre-fills the form with an existing connector.
    pub fn new(existing_names: Vec<String>, edit: Option<&ConnectorConfig>) -> Self {
        let target = edit.map(|c| c.name().to_string());
        let auth_type = edit
            .map(|c| AuthType::of(c.auth()))
            .unwrap_or(AuthType::NoAuth);

        let mut form = Form::default();
        for field in Self::base_fields(edit) {
            form = form.field(field);
        }
        for field in Self::auth_fields(auth_type, edit.map(|c| c.auth())) {
            form = form.field(field);
        }

        let confirm_target = target.clone();
        let form = form.on_confirm(move |fields| {
            parse_values(&flatten(fields), &existing_names, confirm_target.as_deref())
        });

        Self {
            form,
            auth_type,
            target,
        }
    }

    pub fn title(&self) -> String {
        match &self.target {
            Some(name) => format!(" Edit connector {} ", name),
            None => " Add connector ".to_string(),
        }
    }

    #[cfg(test)]
    pub fn form(&self) -> &Form<ConnectorFormOutput> {
        &self.form
    }

    fn secret(name: &str, label: &str) -> TextField {
        TextField::builder()
            .name(name.to_string())
            .label(label.to_string())
            .masked(true)
            .build()
            .expect("secret field")
    }

    fn base_fields(cfg: Option<&ConnectorConfig>) -> Vec<FieldComponent> {
        let mut base = RowField::default()
            .name("base")
            .field(TextField::plain(
                "name",
                "Name",
                cfg.map(|c| c.name()).unwrap_or_default(),
            ))
            .field(TextField::plain(
                "address",
                "Address (management URL)",
                cfg.map(|c| c.address()).unwrap_or_default(),
            ));
        base.set_selected(true);

        let meta = RowField::default()
            .name("meta")
            .field(
                SelectField::new(
                    "api_version",
                    "API version (space)",
                    ConnectorApiVersion::ALL.iter().map(|v| v.as_str()),
                )
                .with_value(cfg.map(|c| c.version().as_str()).unwrap_or("v3")),
            )
            .field(TextField::plain(
                "participant_context_id",
                "Participant context id",
                cfg.and_then(|c| c.participant_context_id())
                    .map(String::as_str)
                    .unwrap_or_default(),
            ))
            .field(
                SelectField::new(
                    "auth_type",
                    "Auth type (space)",
                    AuthType::ALL.iter().map(|t| t.as_str()),
                )
                .with_value(
                    cfg.map(|c| AuthType::of(c.auth()))
                        .unwrap_or(AuthType::NoAuth)
                        .as_str(),
                ),
            );

        vec![base.into(), meta.into()]
    }

    /// The rows for `kind`, pre-filled from `auth` when it is of the same kind.
    fn auth_fields(kind: AuthType, auth: Option<&AuthKind>) -> Vec<FieldComponent> {
        let auth = auth.filter(|a| AuthType::of(a) == kind);
        let opt = |v: &Option<String>| v.clone().unwrap_or_default();
        match kind {
            AuthType::NoAuth => vec![],
            AuthType::Token | AuthType::BearerToken => {
                let alias = match auth {
                    Some(AuthKind::Token { token_alias })
                    | Some(AuthKind::BearerToken { token_alias }) => token_alias.as_str(),
                    _ => "",
                };
                vec![RowField::default()
                    .name("token")
                    .field(TextField::plain(
                        "token_alias",
                        "Token alias (keyring)",
                        alias,
                    ))
                    .field(Self::secret("secret", "Token (blank = keep stored)"))
                    .into()]
            }
            AuthType::OAuth2 => {
                let (client_id, token_url, secret_alias) = match auth {
                    Some(AuthKind::OAuth {
                        client_id,
                        token_url,
                        secret_alias,
                    }) => (
                        client_id.as_str(),
                        token_url.as_str(),
                        secret_alias.as_str(),
                    ),
                    _ => ("", "", ""),
                };
                vec![
                    RowField::default()
                        .name("oauth_1")
                        .field(TextField::plain("client_id", "Client id", client_id))
                        .field(TextField::plain("token_url", "Token URL", token_url))
                        .into(),
                    RowField::default()
                        .name("oauth_2")
                        .field(TextField::plain(
                            "secret_alias",
                            "Secret alias (keyring)",
                            secret_alias,
                        ))
                        .field(Self::secret(
                            "client_secret",
                            "Client secret (blank = keep stored)",
                        ))
                        .into(),
                ]
            }
            AuthType::TokenExchange => {
                let (url, file, alias, resource, audience, scopes) = match auth {
                    Some(AuthKind::TokenExchange {
                        token_exchange_url,
                        subject_token_file,
                        subject_token_alias,
                        resource,
                        audience,
                        scopes,
                    }) => (
                        token_exchange_url.clone(),
                        subject_token_file
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        opt(subject_token_alias),
                        opt(resource),
                        opt(audience),
                        scopes.clone().unwrap_or_default().join(","),
                    ),
                    _ => Default::default(),
                };
                vec![
                    RowField::default()
                        .name("exchange_1")
                        .field(TextField::plain(
                            "token_exchange_url",
                            "Token exchange URL",
                            &url,
                        ))
                        .field(TextField::plain(
                            "subject_token_file",
                            "Subject token file",
                            &file,
                        ))
                        .field(TextField::plain(
                            "subject_token_alias",
                            "Subject token alias (keyring)",
                            &alias,
                        ))
                        .into(),
                    RowField::default()
                        .name("exchange_2")
                        .field(Self::secret(
                            "subject_token",
                            "Subject token (blank = keep stored)",
                        ))
                        .field(TextField::plain("resource", "Resource", &resource))
                        .field(TextField::plain("audience", "Audience", &audience))
                        .field(TextField::plain(
                            "scopes",
                            "Scopes (comma separated)",
                            &scopes,
                        ))
                        .into(),
                ]
            }
        }
    }

    /// Replaces the auth rows when the auth type select has changed.
    fn sync_auth_fields(&mut self) {
        let current = self
            .form
            .field_value("auth_type")
            .and_then(|v| AuthType::parse(&v));
        if let Some(kind) = current.filter(|k| *k != self.auth_type) {
            self.auth_type = kind;
            self.form.truncate_fields(BASE_FIELDS);
            for field in Self::auth_fields(kind, None) {
                self.form.push_field(field);
            }
        }
    }
}

/// Validates the (flattened) form values and builds the connector configuration.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    existing_names: &[String],
    target: Option<&str>,
) -> anyhow::Result<ConnectorFormOutput> {
    let name = required(values, "name", "Name")?;
    if target != Some(name.as_str()) && existing_names.contains(&name) {
        anyhow::bail!("A connector named '{}' already exists", name);
    }
    let address = required_url(values, "address", "Address")?;
    let api_version = value(values, "api_version");
    let api_version = ConnectorApiVersion::parse(&api_version)
        .ok_or_else(|| anyhow::anyhow!("Unknown API version '{}'", api_version))?;
    let auth_type = value(values, "auth_type");
    let auth_type = AuthType::parse(&auth_type)
        .ok_or_else(|| anyhow::anyhow!("Unknown auth type '{}'", auth_type))?;

    let mut secrets = vec![];
    let auth = match auth_type {
        AuthType::NoAuth => AuthKind::NoAuth,
        AuthType::Token | AuthType::BearerToken => {
            let token_alias = required(values, "token_alias", "Token alias")?;
            if let Some(secret) = optional(values, "secret") {
                secrets.push((token_alias.clone(), secret));
            }
            if auth_type == AuthType::Token {
                AuthKind::Token { token_alias }
            } else {
                AuthKind::BearerToken { token_alias }
            }
        }
        AuthType::OAuth2 => {
            let secret_alias = required(values, "secret_alias", "Secret alias")?;
            if let Some(secret) = optional(values, "client_secret") {
                secrets.push((secret_alias.clone(), secret));
            }
            AuthKind::OAuth {
                client_id: required(values, "client_id", "Client id")?,
                token_url: required_url(values, "token_url", "Token URL")?,
                secret_alias,
            }
        }
        AuthType::TokenExchange => {
            let token_exchange_url =
                required_url(values, "token_exchange_url", "Token exchange URL")?;
            let subject_token_file = optional(values, "subject_token_file");
            let subject_token_alias = optional(values, "subject_token_alias");
            match (&subject_token_file, &subject_token_alias) {
                (Some(_), None) | (None, Some(_)) => {}
                _ => anyhow::bail!(
                    "Token exchange needs exactly one of subject token file or subject token alias"
                ),
            }
            if let Some(secret) = optional(values, "subject_token") {
                match &subject_token_alias {
                    Some(alias) => secrets.push((alias.clone(), secret)),
                    None => anyhow::bail!(
                        "A subject token can only be stored under a subject token alias"
                    ),
                }
            }
            let scopes = list(values, "scopes");
            AuthKind::TokenExchange {
                token_exchange_url,
                subject_token_file: subject_token_file.map(Into::into),
                subject_token_alias,
                resource: optional(values, "resource"),
                audience: optional(values, "audience"),
                scopes: Some(scopes).filter(|s| !s.is_empty()),
            }
        }
    };

    let config = ConnectorConfig::builder()
        .name(name)
        .address(address)
        .api_version(api_version)
        .auth(auth)
        .participant_context_id(optional(values, "participant_context_id"))
        .build()?;

    Ok(ConnectorFormOutput {
        config,
        secrets,
        target: target.map(String::from),
    })
}

#[async_trait::async_trait]
impl Component for ConnectorForm {
    type Msg = ConnectorFormMsg;
    type Props = ();

    fn view(&mut self, f: &mut Frame, _rect: Rect) {
        let area = popup::centered(f.area(), 80, 70);
        let content = popup::framed(f, area, &self.title());
        self.form.view(f, content);
    }

    async fn update(
        &mut self,
        msg: ComponentMsg<Self::Msg>,
    ) -> anyhow::Result<ComponentReturn<Self::Msg>> {
        match msg.take() {
            ConnectorFormMsg::Local(form) => {
                let ret = Self::forward_update(&mut self.form, form.into(), |msg| match msg {
                    FormMsg::Local(local) => ConnectorFormMsg::Local(FormMsg::Local(local)),
                    FormMsg::Outer(out) => ConnectorFormMsg::Submitted(Box::new(out)),
                })
                .await?;
                self.sync_auth_fields();
                Ok(ret)
            }
            ConnectorFormMsg::Submitted(_) => Ok(ComponentReturn::empty()),
        }
    }

    fn handle_event(
        &mut self,
        evt: ComponentEvent,
    ) -> anyhow::Result<Vec<ComponentMsg<Self::Msg>>> {
        Self::forward_event(&mut self.form, evt, ConnectorFormMsg::Local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        let mut map: HashMap<String, String> = [
            ("name", "c"),
            ("address", "http://localhost:29193/management"),
            ("api_version", "v3"),
            ("auth_type", "no-auth"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.to_string());
        }
        map
    }

    fn parse(pairs: &[(&str, &str)]) -> anyhow::Result<ConnectorFormOutput> {
        parse_values(&values(pairs), &[], None)
    }

    fn err(pairs: &[(&str, &str)]) -> String {
        match parse(pairs) {
            Ok(out) => panic!("expected an error, got {out:?}"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn no_auth_connector() {
        let out = parse(&[("participant_context_id", "  "), ("api_version", "v5")]).unwrap();
        assert_eq!(out.config.name(), "c");
        assert_eq!(*out.config.version(), ConnectorApiVersion::V5);
        assert!(out.config.participant_context_id().is_none());
        assert!(matches!(out.config.auth(), AuthKind::NoAuth));
        assert!(out.secrets.is_empty());
        assert!(out.target.is_none());
    }

    #[test]
    fn name_and_address_are_validated() {
        assert!(err(&[("name", " ")]).contains("Name is required"));
        assert!(err(&[("address", "")]).contains("Address is required"));
        assert!(err(&[("address", "not a url")]).contains("not a valid URL"));
        assert!(err(&[("address", "ftp://host/x")]).contains("http(s)"));
    }

    #[test]
    fn duplicate_names_are_rejected_unless_editing_that_connector() {
        let existing = vec!["c".to_string()];
        let dup = parse_values(&values(&[]), &existing, None);
        assert!(dup.unwrap_err().to_string().contains("already exists"));
        assert!(parse_values(&values(&[]), &existing, Some("c")).is_ok());
        assert!(parse_values(&values(&[]), &existing, Some("other")).is_err());
    }

    #[test]
    fn token_auth_collects_secret_only_when_filled() {
        let out = parse(&[("auth_type", "token"), ("token_alias", "a"), ("secret", "")]).unwrap();
        assert!(matches!(out.config.auth(), AuthKind::Token { token_alias } if token_alias == "a"));
        assert!(out.secrets.is_empty());

        let out = parse(&[
            ("auth_type", "bearer-token"),
            ("token_alias", "a"),
            ("secret", "s3cret"),
        ])
        .unwrap();
        assert!(matches!(out.config.auth(), AuthKind::BearerToken { .. }));
        assert_eq!(out.secrets, vec![("a".to_string(), "s3cret".to_string())]);

        assert!(err(&[("auth_type", "token")]).contains("Token alias is required"));
    }

    #[test]
    fn oauth2_requires_all_fields() {
        let base = [
            ("auth_type", "oauth2"),
            ("client_id", "id"),
            ("token_url", "http://idp/token"),
            ("secret_alias", "s"),
        ];
        let out = parse(&base).unwrap();
        assert!(matches!(out.config.auth(), AuthKind::OAuth { .. }));
        assert!(err(&[("auth_type", "oauth2"), ("client_id", "id")]).contains("required"));
        let mut bad = base.to_vec();
        bad[2] = ("token_url", "nope");
        assert!(err(&bad).contains("Token URL"));
    }

    #[test]
    fn token_exchange_needs_exactly_one_subject_source() {
        let base = [
            ("auth_type", "token-exchange"),
            ("token_exchange_url", "http://jwtlet:8080/token"),
        ];
        assert!(err(&base).contains("exactly one"));
        let mut both = base.to_vec();
        both.push(("subject_token_file", "/tmp/t"));
        both.push(("subject_token_alias", "a"));
        assert!(err(&both).contains("exactly one"));

        let mut file = base.to_vec();
        file.push(("subject_token_file", "/tmp/t"));
        file.push(("subject_token", "x"));
        assert!(err(&file).contains("subject token alias"));

        let mut alias = base.to_vec();
        alias.push(("subject_token_alias", "a"));
        alias.push(("subject_token", "x"));
        alias.push(("scopes", " a, b ,, "));
        alias.push(("resource", ""));
        let out = parse(&alias).unwrap();
        match out.config.auth() {
            AuthKind::TokenExchange {
                subject_token_file,
                subject_token_alias,
                resource,
                scopes,
                ..
            } => {
                assert!(subject_token_file.is_none());
                assert_eq!(subject_token_alias.as_deref(), Some("a"));
                assert!(resource.is_none());
                assert_eq!(
                    scopes.as_deref(),
                    Some(&["a".to_string(), "b".to_string()][..])
                );
            }
            other => panic!("unexpected auth {other:?}"),
        }
        assert_eq!(out.secrets, vec![("a".to_string(), "x".to_string())]);
    }

    async fn press(form: &mut ConnectorForm, code: KeyCode) {
        for m in form.handle_event(KeyEvent::from(code).into()).unwrap() {
            form.update(m).await.unwrap();
        }
    }

    #[tokio::test]
    async fn changing_auth_type_swaps_fields_and_keeps_base_values() {
        let mut form = ConnectorForm::new(vec![], None);
        assert_eq!(form.form().fields_len(), 2);
        for c in "abc".chars() {
            press(&mut form, KeyCode::Char(c)).await;
        }
        press(&mut form, KeyCode::Down).await;
        press(&mut form, KeyCode::Right).await;
        press(&mut form, KeyCode::Right).await;

        press(&mut form, KeyCode::Char(' ')).await; // token
        assert_eq!(form.form().fields_len(), 3);
        assert_eq!(form.form().field_value("token_alias").as_deref(), Some(""));
        press(&mut form, KeyCode::Char(' ')).await; // bearer-token
        assert_eq!(form.form().fields_len(), 3);
        press(&mut form, KeyCode::Char(' ')).await; // oauth2
        assert_eq!(form.form().fields_len(), 4);
        press(&mut form, KeyCode::Char(' ')).await; // token-exchange
        assert_eq!(form.form().fields_len(), 4);
        assert!(form.form().field_value("scopes").is_some());
        press(&mut form, KeyCode::Char(' ')).await; // no-auth
        assert_eq!(form.form().fields_len(), 2);
        assert_eq!(form.form().field_value("name").as_deref(), Some("abc"));
    }

    #[test]
    fn edit_prefills_matching_auth_fields() {
        let cfg = ConnectorConfig::builder()
            .name("c")
            .address("http://localhost/m")
            .api_version(ConnectorApiVersion::V4)
            .auth(AuthKind::OAuth {
                client_id: "id".into(),
                token_url: "http://idp".into(),
                secret_alias: "s".into(),
            })
            .build()
            .unwrap();
        let form = ConnectorForm::new(vec![], Some(&cfg));
        assert_eq!(form.form().fields_len(), 4);
        assert_eq!(
            form.form().field_value("api_version").as_deref(),
            Some("v4")
        );
        assert_eq!(
            form.form().field_value("auth_type").as_deref(),
            Some("oauth2")
        );
        assert_eq!(form.form().field_value("client_id").as_deref(), Some("id"));
        assert_eq!(
            form.form().field_value("client_secret").as_deref(),
            Some("")
        );
        assert_eq!(form.title(), " Edit connector c ");
    }
}
