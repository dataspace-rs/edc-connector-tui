use std::str::FromStr;

use anyhow::bail;
use enum_ordinalize::Ordinalize;
use strum::{EnumString, VariantNames};

use crate::config::ConnectorApiVersion;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    Participants,
    DataspaceProfiles,
    CelExpressions,
    CachedDocuments,
    DcpScopes,
    SchemaValidators,
}

impl FromStr for Nav {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connectors" => Ok(Nav::ConnectorsList),
            "assets" => Ok(Nav::AssetsList),
            "policies" => Ok(Nav::PoliciesList),
            "participants" => Ok(Nav::Participants),
            "profiles" => Ok(Nav::DataspaceProfiles),
            "cel" => Ok(Nav::CelExpressions),
            "documents" | "cache" => Ok(Nav::CachedDocuments),
            "scopes" => Ok(Nav::DcpScopes),
            "validators" | "schemas" => Ok(Nav::SchemaValidators),
            _ => bail!("Command {} not recognized", s),
        }
    }
}

/// The two sets of menu entries: day-to-day operations on a participant and the global
/// (admin only) resources of a v5 connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(usize)]
pub enum Workspace {
    #[default]
    Operations,
    Admin,
}

impl Workspace {
    pub fn name(&self) -> &'static str {
        match self {
            Workspace::Operations => "Operations",
            Workspace::Admin => "Admin",
        }
    }

    pub fn other(&self) -> Workspace {
        match self {
            Workspace::Operations => Workspace::Admin,
            Workspace::Admin => Workspace::Operations,
        }
    }

    /// Whether the workspace makes sense for a connector speaking `version`.
    /// `None` (no connector selected) allows both.
    pub fn is_available_for(&self, version: Option<ConnectorApiVersion>) -> bool {
        match self {
            Workspace::Operations => true,
            Workspace::Admin => version.is_none_or(|v| v.supports_admin()),
        }
    }
}

impl FromStr for Workspace {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Workspace::Admin),
            "ops" | "operations" => Ok(Workspace::Operations),
            _ => bail!("Unknown workspace {}", s),
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
    Participants,
    DataspaceProfiles,
    CelExpressions,
    CachedDocuments,
    DcpScopes,
    SchemaValidators,
}

impl Menu {
    pub fn name(&self) -> &'static str {
        <Menu as VariantNames>::VARIANTS[self.ordinal()]
    }

    pub fn workspace(&self) -> Workspace {
        match self {
            Menu::Connectors
            | Menu::Assets
            | Menu::Policies
            | Menu::ContractDefinitions
            | Menu::ContractNegotiations
            | Menu::ContractAgreements
            | Menu::TransferProcesses
            | Menu::Edrs
            | Menu::DataPlanes => Workspace::Operations,
            Menu::Participants
            | Menu::DataspaceProfiles
            | Menu::CelExpressions
            | Menu::CachedDocuments
            | Menu::DcpScopes
            | Menu::SchemaValidators => Workspace::Admin,
        }
    }

    /// The entry selected when a workspace is opened for the first time.
    pub fn home(workspace: Workspace) -> Menu {
        match workspace {
            Workspace::Operations => Menu::Connectors,
            Workspace::Admin => Menu::Participants,
        }
    }

    /// Whether this menu entry makes sense for a connector speaking `version`.
    /// `None` (no connector selected) shows every entry.
    pub fn is_available_for(&self, version: Option<ConnectorApiVersion>) -> bool {
        match self {
            Menu::Edrs => version.is_none_or(|v| v.supports_edrs()),
            _ => self.workspace().is_available_for(version),
        }
    }

    /// The entries of `workspace` available for `version`, in declaration order.
    pub fn available_for(version: Option<ConnectorApiVersion>, workspace: Workspace) -> Vec<Menu> {
        <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| m.workspace() == workspace && m.is_available_for(version))
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
            Nav::Participants => Menu::Participants,
            Nav::DataspaceProfiles => Menu::DataspaceProfiles,
            Nav::CelExpressions => Menu::CelExpressions,
            Nav::CachedDocuments => Menu::CachedDocuments,
            Nav::DcpScopes => Menu::DcpScopes,
            Nav::SchemaValidators => Menu::SchemaValidators,
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
            Menu::Participants => Nav::Participants,
            Menu::DataspaceProfiles => Nav::DataspaceProfiles,
            Menu::CelExpressions => Nav::CelExpressions,
            Menu::CachedDocuments => Nav::CachedDocuments,
            Menu::DcpScopes => Nav::DcpScopes,
            Menu::SchemaValidators => Nav::SchemaValidators,
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

    fn ops() -> Vec<Menu> {
        <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| m.workspace() == Workspace::Operations)
            .cloned()
            .collect()
    }

    fn admin() -> Vec<Menu> {
        <Menu as Ordinalize>::VARIANTS
            .iter()
            .filter(|m| m.workspace() == Workspace::Admin)
            .cloned()
            .collect()
    }

    #[test]
    fn edrs_available_only_without_connector_or_for_v3() {
        assert!(Menu::Edrs.is_available_for(None));
        assert!(Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V3)));
        assert!(!Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V4)));
        assert!(!Menu::Edrs.is_available_for(Some(ConnectorApiVersion::V5)));
    }

    #[test]
    fn other_ops_menus_are_always_available() {
        for menu in ops().iter().filter(|m| **m != Menu::Edrs) {
            for version in VERSIONS {
                assert!(menu.is_available_for(version), "{menu:?} for {version:?}");
            }
        }
    }

    #[test]
    fn admin_menus_available_only_without_connector_or_for_v5() {
        for menu in admin() {
            assert!(menu.is_available_for(None), "{menu:?}");
            assert!(!menu.is_available_for(Some(ConnectorApiVersion::V3)));
            assert!(!menu.is_available_for(Some(ConnectorApiVersion::V4)));
            assert!(menu.is_available_for(Some(ConnectorApiVersion::V5)));
        }
        assert!(Workspace::Admin.is_available_for(None));
        assert!(!Workspace::Admin.is_available_for(Some(ConnectorApiVersion::V4)));
        assert!(Workspace::Admin.is_available_for(Some(ConnectorApiVersion::V5)));
        for version in VERSIONS {
            assert!(Workspace::Operations.is_available_for(version));
        }
    }

    #[test]
    fn available_for_filters_by_workspace_and_keeps_order() {
        let ops_without_edrs: Vec<Menu> = ops().into_iter().filter(|m| *m != Menu::Edrs).collect();
        assert_eq!(
            Menu::available_for(Some(ConnectorApiVersion::V4), Workspace::Operations),
            ops_without_edrs
        );
        assert_eq!(
            Menu::available_for(Some(ConnectorApiVersion::V5), Workspace::Operations),
            ops_without_edrs
        );
        assert_eq!(
            Menu::available_for(Some(ConnectorApiVersion::V3), Workspace::Operations),
            ops()
        );
        assert_eq!(Menu::available_for(None, Workspace::Operations), ops());
        assert_eq!(Menu::available_for(None, Workspace::Admin), admin());
        assert_eq!(
            Menu::available_for(Some(ConnectorApiVersion::V5), Workspace::Admin),
            admin()
        );
        assert!(Menu::available_for(Some(ConnectorApiVersion::V3), Workspace::Admin).is_empty());
    }

    #[test]
    fn every_menu_round_trips_through_nav() {
        for menu in <Menu as Ordinalize>::VARIANTS {
            let nav: Nav = menu.clone().into();
            assert_eq!(Menu::from(nav), *menu);
        }
        assert_eq!(Menu::home(Workspace::Admin).workspace(), Workspace::Admin);
        assert_eq!(
            Menu::home(Workspace::Operations).workspace(),
            Workspace::Operations
        );
    }

    #[test]
    fn launch_bar_commands_parse() {
        assert_eq!("profiles".parse::<Nav>().unwrap(), Nav::DataspaceProfiles);
        assert_eq!("cache".parse::<Nav>().unwrap(), Nav::CachedDocuments);
        assert_eq!("schemas".parse::<Nav>().unwrap(), Nav::SchemaValidators);
        assert!("nope".parse::<Nav>().is_err());
        assert_eq!("admin".parse::<Workspace>().unwrap(), Workspace::Admin);
        assert_eq!("ops".parse::<Workspace>().unwrap(), Workspace::Operations);
        assert!("nope".parse::<Workspace>().is_err());
    }

    #[test]
    fn name_matches_variant_name() {
        assert_eq!(Menu::Edrs.name(), "Edrs");
        assert_eq!(Menu::DataPlanes.name(), "DataPlanes");
        assert_eq!(Menu::DcpScopes.name(), "DcpScopes");
        assert_eq!(Workspace::Admin.other(), Workspace::Operations);
    }
}
