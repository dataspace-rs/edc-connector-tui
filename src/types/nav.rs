use std::str::FromStr;

use anyhow::bail;
use enum_ordinalize::Ordinalize;
use strum::{EnumString, VariantNames};

#[derive(Debug, Clone, Default)]
pub enum Nav {
    #[default]
    ConnectorsList,
    AssetsList,
    PoliciesList,
    ContractDefinitionsList,
    ContractNegotiations,
    ContractAgreements,
    TransferProcesses,
    Edrs,
}

impl FromStr for Nav {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connectors" => Ok(Nav::ConnectorsList),
            "assets" => Ok(Nav::AssetsList),
            "policies" => Ok(Nav::PoliciesList),
            _ => bail!("Command {} not recognized", s),
        }
    }
}

#[derive(Debug, Default, Ordinalize, Clone, EnumString, VariantNames)]
#[repr(usize)]
pub enum Menu {
    #[default]
    Connectors,
    Assets,
    Policies,
    ContractDefinitions,
    ContractNegotiations,
    ContractAgreements,
    TransferProcesses,
    Edrs,
}

impl Menu {
    pub fn names() -> Vec<String> {
        <Menu as VariantNames>::VARIANTS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}

impl From<Nav> for Menu {
    fn from(val: Nav) -> Self {
        match val {
            Nav::ConnectorsList => Menu::Connectors,
            Nav::AssetsList => Menu::Assets,
            Nav::PoliciesList => Menu::Policies,
            Nav::ContractDefinitionsList => Menu::ContractDefinitions,
            Nav::ContractNegotiations => Menu::ContractNegotiations,
            Nav::TransferProcesses => Menu::TransferProcesses,
            Nav::ContractAgreements => Menu::ContractAgreements,
            Nav::Edrs => Menu::Edrs,
        }
    }
}

impl From<Menu> for Nav {
    fn from(val: Menu) -> Self {
        match val {
            Menu::Connectors => Nav::ConnectorsList,
            Menu::Assets => Nav::AssetsList,
            Menu::Policies => Nav::PoliciesList,
            Menu::ContractDefinitions => Nav::ContractDefinitionsList,
            Menu::ContractNegotiations => Nav::ContractNegotiations,
            Menu::TransferProcesses => Nav::TransferProcesses,
            Menu::ContractAgreements => Nav::ContractAgreements,
            Menu::Edrs => Nav::Edrs,
        }
    }
}
