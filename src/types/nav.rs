use std::str::FromStr;

use anyhow::bail;
use enum_ordinalize::Ordinalize;
use strum::{EnumString, VariantNames};

use crate::config::ConnectorApiVersion;

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
    DataPlanes,
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

#[derive(Debug, Default, Ordinalize, Clone, PartialEq, Eq, EnumString, VariantNames)]
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
    DataPlanes,
}

impl Menu {
    pub fn name(&self) -> &'static str {
        <Menu as VariantNames>::VARIANTS[self.ordinal()]
    }

    /// Whether this menu entry makes sense for a connector speaking `version`.
    /// `None` (no connector selected) shows every entry.
    pub fn is_available_for(&self, version: Option<ConnectorApiVersion>) -> bool {
        match self {
            Menu::Edrs => version.is_none_or(|v| v.supports_edrs()),
            _ => true,
        }
    }

    /// All menu entries available for `version`, in declaration order.
    pub fn available_for(version: Option<ConnectorApiVersion>) -> Vec<Menu> {
        <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| m.is_available_for(version))
            .cloned()
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
            Nav::DataPlanes => Menu::DataPlanes,
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
            Menu::DataPlanes => Nav::DataPlanes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSIONS: [Option<ConnectorApiVersion>; 4] = [
        None,
        Some(ConnectorApiVersion::V3),
        Some(ConnectorApiVersion::V4),
        Some(ConnectorApiVersion::V5),
    ];

    #[test]
    fn edrs_available_only_without_connector_or_for_v3() {
        assert!(Menu::Edrs.is_available_for(None));
        assert!(Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V3)));
        assert!(!Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V4)));
        assert!(!Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V5)));
    }

    #[test]
    fn other_menus_are_always_available() {
        for menu in <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| **m != Menu::Edrs)
        {
            for version in VERSIONS {
                assert!(menu.is_available_for(version), "{menu:?} for {version:?}");
            }
        }
    }

    #[test]
    fn available_for_v4_omits_only_edrs_and_keeps_order() {
        let expected: Vec<Menu> = <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| **m != Menu::Edrs)
            .cloned()
            .collect();
        assert_eq!(Menu::available_for(Some(ConnectorApiVersion::V4)), expected);
        assert_eq!(Menu::available_for(Some(ConnectorApiVersion::V5)), expected);
        assert_eq!(
            Menu::available_for(Some(ConnectorApiVersion::V3)),
            <Menu as Ordinalize>::VARIANTS.to_vec()
        );
        assert_eq!(
            Menu::available_for(None),
            <Menu as Ordinalize>::VARIANTS.to_vec()
        );
    }

    #[test]
    fn name_matches_variant_name() {
        assert_eq!(Menu::Edrs.name(), "Edrs");
        assert_eq!(Menu::DataPlanes.name(), "DataPlanes");
    }
}
