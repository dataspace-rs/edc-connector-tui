use std::collections::HashMap;

use edc_connector_client::types::common_expression_language::{
    CommonExpressionLanguage, NewCommonExpressionLanguage,
};
use ratatui::widgets::Row;
use serde_json::{Map, Value};

use crate::{
    components::{
        resources::{
            msg::{ActionInput, ResourcesMsg},
            DrawableResource, Field, ResourcesComponent,
        },
        table::TableEntry,
    },
    widgets::form::{
        row::RowField,
        text::TextField,
        values::{flatten, json, list, optional, required, value},
        Form,
    },
};

use super::{compact_json, join};

#[derive(Debug, Clone)]
pub struct CelExpressionEntry(CommonExpressionLanguage);

pub type CelExpressionsMsg = ResourcesMsg<CelExpressionEntry, CelExpressionEntry>;
pub type CelExpressionsComponent = ResourcesComponent<CelExpressionEntry, CelExpressionEntry>;

/// The parsed input of the "test expression" form.
#[derive(Debug, PartialEq)]
pub struct CelTest {
    pub operator: String,
    pub right_operand: Value,
    pub params: Map<String, Value>,
}

impl CelExpressionEntry {
    pub fn new(expression: CommonExpressionLanguage) -> Self {
        Self(expression)
    }

    pub fn inner(&self) -> &CommonExpressionLanguage {
        &self.0
    }

    /// The create request: a blank id lets the connector generate one.
    pub fn to_new(&self) -> NewCommonExpressionLanguage {
        let mut builder = NewCommonExpressionLanguage::builder()
            .maybe_id(Some(self.0.id().to_string()).filter(|id| !id.is_empty()))
            .left_operand(self.0.left_operand().to_string())
            .maybe_description(self.0.description().clone())
            .scopes(self.0.scopes().clone())
            .actions(self.0.actions().clone())
            .expression(self.0.expression().to_string());
        for (k, v) in self.0.properties().iter() {
            builder = builder.property(k, v.0.clone());
        }
        for (k, v) in self.0.private_properties().iter() {
            builder = builder.private_property(k, v.0.clone());
        }
        builder.build()
    }
}

impl TableEntry for CelExpressionEntry {
    fn row(&self) -> Row<'_> {
        Row::new(vec![
            self.0.id().to_string(),
            self.0.left_operand().to_string(),
            join(self.0.scopes()),
            join(self.0.actions()),
            self.0.description().clone().unwrap_or_default(),
        ])
    }

    fn headers() -> Row<'static> {
        Row::new(vec![
            "ID",
            "LEFT OPERAND",
            "SCOPES",
            "ACTIONS",
            "DESCRIPTION",
        ])
    }
}

impl DrawableResource for CelExpressionEntry {
    fn id(&self) -> &str {
        self.0.id()
    }

    fn title() -> &'static str {
        "CelExpressions"
    }

    fn fields(&self) -> Vec<Field> {
        vec![
            Field::string("id", self.0.id()),
            Field::string("leftOperand", self.0.left_operand()),
            Field::string("expression", self.0.expression()),
            Field::string(
                "description",
                self.0.description().clone().unwrap_or_default(),
            ),
            Field::string("scopes", join(self.0.scopes())),
            Field::string("actions", join(self.0.actions())),
            Field::json("properties", self.0.properties()),
            Field::json("privateProperties", self.0.private_properties()),
        ]
    }
}

/// The add (`None`) or edit (`Some`) form.
pub fn form(edit: Option<&CelExpressionEntry>) -> Form<CelExpressionEntry> {
    let target = edit.map(|e| e.0.id().to_string());
    let cel = edit.map(|e| &e.0);
    let id_label = if edit.is_some() {
        "Id"
    } else {
        "Id (blank = generated)"
    };
    let mut head = RowField::default()
        .name("head")
        .field(TextField::plain(
            "id",
            id_label,
            cel.map(|c| c.id()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "left_operand",
            "Left operand",
            cel.map(|c| c.left_operand()).unwrap_or_default(),
        ));
    head.set_selected(true);

    Form::default()
        .field(head)
        .field(TextField::plain(
            "expression",
            "Expression",
            cel.map(|c| c.expression()).unwrap_or_default(),
        ))
        .field(TextField::plain(
            "description",
            "Description",
            &cel.and_then(|c| c.description().clone())
                .unwrap_or_default(),
        ))
        .field(
            RowField::default()
                .name("lists")
                .field(TextField::plain(
                    "scopes",
                    "Scopes (comma separated)",
                    &cel.map(|c| join(c.scopes())).unwrap_or_default(),
                ))
                .field(TextField::plain(
                    "actions",
                    "Actions (comma separated, e.g. use)",
                    &cel.map(|c| join(c.actions())).unwrap_or_default(),
                )),
        )
        .field(
            RowField::default()
                .name("props")
                .field(TextField::plain(
                    "properties",
                    "Properties (JSON object)",
                    &cel.map(|c| compact_json(c.properties()))
                        .unwrap_or_default(),
                ))
                .field(TextField::plain(
                    "private_properties",
                    "Private properties (JSON object)",
                    &cel.map(|c| compact_json(c.private_properties()))
                        .unwrap_or_default(),
                )),
        )
        .on_confirm(move |fields| parse_values(&flatten(fields), target.as_deref()))
}

/// Validates the flattened form values; `target` is the id being edited.
pub(crate) fn parse_values(
    values: &HashMap<String, String>,
    target: Option<&str>,
) -> anyhow::Result<CelExpressionEntry> {
    let id = optional(values, "id");
    if let Some(target) = target {
        if id.as_deref() != Some(target) {
            anyhow::bail!("Id cannot be changed");
        }
    }
    let properties: Map<String, Value> =
        json(values, "properties", "Properties")?.unwrap_or_default();
    let private_properties: Map<String, Value> =
        json(values, "private_properties", "Private properties")?.unwrap_or_default();

    let mut builder = CommonExpressionLanguage::builder()
        .id(id.unwrap_or_default())
        .left_operand(required(values, "left_operand", "Left operand")?)
        .maybe_description(optional(values, "description"))
        .scopes(list(values, "scopes"))
        .actions(list(values, "actions"))
        .expression(required(values, "expression", "Expression")?);
    for (k, v) in properties {
        builder = builder.property(&k, v);
    }
    for (k, v) in private_properties {
        builder = builder.private_property(&k, v);
    }
    Ok(CelExpressionEntry(builder.build()))
}

/// The "test expression" form: the constraint operands and the `ctx` parameters.
pub fn test_form(_: &CelExpressionEntry, _: Option<&CelExpressionEntry>) -> Form<ActionInput> {
    let mut head = RowField::default()
        .name("head")
        .field(TextField::plain("operator", "Operator (e.g. eq)", ""))
        .field(TextField::plain(
            "right_operand",
            "Right operand (JSON or text)",
            "",
        ));
    head.set_selected(true);
    Form::default()
        .field(head)
        .field(TextField::plain(
            "params",
            "Parameters exposed as ctx (JSON object)",
            "",
        ))
        .on_confirm(|fields| Ok(flatten(fields)))
}

/// Parses the "test expression" input; a right operand that is not JSON is used as text.
pub(crate) fn parse_test(values: &ActionInput) -> anyhow::Result<CelTest> {
    let raw = value(values, "right_operand");
    let right_operand = serde_json::from_str(&raw).unwrap_or(Value::String(raw));
    Ok(CelTest {
        operator: required(values, "operator", "Operator")?,
        right_operand,
        params: json(values, "params", "Parameters")?.unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn parses_lists_and_json_properties() {
        let entry = parse_values(
            &values(&[
                ("left_operand", "MembershipCredential"),
                ("expression", "ctx.x == 1"),
                ("scopes", "a, b"),
                ("actions", "use"),
                ("properties", r#"{"k":"v"}"#),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(entry.0.scopes(), &vec!["a".to_string(), "b".to_string()]);
        assert_eq!(entry.0.actions(), &vec!["use".to_string()]);
        assert_eq!(
            entry.0.properties().get::<String>("k").unwrap().as_deref(),
            Some("v")
        );
        assert_eq!(entry.0.description(), &None);
        let new = serde_json::to_value(entry.to_new()).unwrap();
        assert!(new.get("@id").is_none());
        assert_eq!(new["properties"]["k"], "v");
    }

    #[test]
    fn required_fields_and_invalid_json_are_rejected() {
        let err = parse_values(&values(&[("expression", "x")]), None).unwrap_err();
        assert_eq!(err.to_string(), "Left operand is required");
        let err = parse_values(
            &values(&[
                ("left_operand", "l"),
                ("expression", "x"),
                ("properties", "{"),
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().starts_with("Properties is not valid JSON"));
        let err = parse_values(
            &values(&[("id", "b"), ("left_operand", "l"), ("expression", "x")]),
            Some("a"),
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "Id cannot be changed");
    }

    #[test]
    fn test_input_accepts_json_or_text_operands() {
        let test = parse_test(&values(&[
            ("operator", "eq"),
            ("right_operand", "42"),
            ("params", r#"{"a": true}"#),
        ]))
        .unwrap();
        assert_eq!(test.right_operand, Value::from(42));
        assert_eq!(test.params["a"], Value::Bool(true));

        let test = parse_test(&values(&[("operator", "eq"), ("right_operand", "plain")])).unwrap();
        assert_eq!(test.right_operand, Value::String("plain".to_string()));
        assert!(test.params.is_empty());

        assert!(parse_test(&values(&[("right_operand", "x")])).is_err());
    }
}
