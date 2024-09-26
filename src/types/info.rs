use std::collections::BTreeMap;

#[derive(Default, Debug)]
pub struct InfoSheet {
    info: BTreeMap<String, String>,
    key_bindings: BTreeMap<String, String>,
}

impl InfoSheet {
    pub fn info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.info.insert(key.into(), value.into());
        self
    }

    pub fn key_binding(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.key_bindings.insert(key.into(), value.into());
        self
    }

    pub fn iter_info(&self) -> impl Iterator<Item = (&String, &String)> {
        self.info.iter()
    }

    pub fn iter_key_bindings(&self) -> impl Iterator<Item = (&String, &String)> {
        self.key_bindings.iter()
    }

    pub fn merge(&self, other: InfoSheet) -> InfoSheet {
        let info = self
            .iter_info()
            .chain(other.iter_info())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<BTreeMap<String, String>>();

        let key_bindings = self
            .iter_key_bindings()
            .chain(other.iter_key_bindings())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<BTreeMap<String, String>>();

        InfoSheet { info, key_bindings }
    }
}
