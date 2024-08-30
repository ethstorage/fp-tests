//! Contains the registry type, which holds metadata about the available FPVMs and FPPs.

use crate::cli::TestConfig;
use platform::PlatformKind;
use program::ProgramKind;
use serde::{Deserialize, Serialize};
use std::{cell::LazyCell, collections::HashMap, path::PathBuf};

pub(crate) mod build;
pub(crate) mod platform;
pub(crate) mod program;

/// The directory containing the components.
pub(crate) const COMPONENTS_DIR: &'static str = concat!(env!("HOME"), "/.fpt/components");

/// The FP Registry.
pub(crate) const FP_REGISTRY: LazyCell<FPRegistry> = LazyCell::new(|| {
    const REGISTRY_SER: &str = include_str!("../../registry.toml");
    toml::from_str(REGISTRY_SER).expect("Failed to parse registry")
});

/// The FP Registry holds metadata about the available FPVMs and FPPs.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FPRegistry {
    /// The fault proof virtual machines available in the registry.
    pub(crate) platform: HashMap<PlatformKind, PlatformDefinition>,
    /// The fault proof programs available in the registry.
    pub(crate) program: HashMap<ProgramKind, FPPDefinition>,
}

impl FPRegistry {
    /// Returns the matrix of compatibility between the available FPVMs and FPPs.
    ///
    /// ## Takes
    /// - `cfg` - The test configuration. If `None`, all possible configurations are returned.
    ///
    /// ## Returns
    /// - `Vec<DefPair>` - The matrix of FPVMs and FPPs compatible with the [TestConfig].
    pub(crate) fn resolve_matrix<'a>(
        &'a self,
        cfg: Option<&TestConfig>,
    ) -> Vec<PlatformAndPrograms> {
        let mut matrix = Vec::new();

        let selected_platforms = if let Some(cfg) = cfg {
            if let Some(vm) = cfg.vm.as_ref() {
                self.platform
                    .iter()
                    .filter(|(kind, _)| vm.contains(kind))
                    .collect::<HashMap<_, _>>()
            } else {
                self.platform
                    .iter()
                    .filter(|(_, def)| def.default)
                    .collect::<HashMap<_, _>>()
            }
        } else {
            self.platform.iter().collect::<HashMap<_, _>>()
        };

        for (vm_kind, vm_def) in selected_platforms {
            let compat = self
                .program
                .iter()
                .filter_map(|(prog_name, prog_def)| {
                    ((cfg
                        .map(|cfg| cfg.program.as_ref().map(|p| p.contains(prog_name)))
                        .flatten()
                        .unwrap_or_default()
                        || prog_def.default)
                        && prog_def.platform_compat.contains(&vm_kind))
                    .then_some((prog_name.clone(), prog_def.clone()))
                })
                .collect::<HashMap<_, _>>();

            matrix.push(PlatformAndPrograms {
                vm: vm_def.clone(),
                vm_kind: vm_kind.clone(),
                programs: compat,
            });
        }
        matrix
    }
}

/// The platform definition holds metadata about a platform that runs the fault proof programs.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PlatformDefinition {
    /// Whether or not to run the platform by default.
    #[serde(default)]
    pub(crate) default: bool,
    /// The instructions to build the platform locally.
    pub(crate) build: Option<BuildInstructions>,
}

/// The FPP definition holds metadata about a fault proof program.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FPPDefinition {
    /// Whether or not to run the FPP by default.
    #[serde(default)]
    pub(crate) default: bool,
    /// The compatibility of the FPP, with respect to the available platform.
    pub(crate) platform_compat: Vec<PlatformKind>,
    /// The instructions to build the FPP locally.
    pub(crate) build: BuildInstructions,
}

/// Build instructions for a binary within a GitHub repository.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct BuildInstructions {
    /// The org/reponame of the github repository to build.
    pub(crate) repo: String,
    /// The revision or tag to build.
    pub(crate) rev: String,
    /// The workdir of the build.
    pub(crate) workdir: PathBuf,
    /// The build command to run.
    pub(crate) cmd: String,
    /// The binary path, relative to the workdir.
    pub(crate) artifacts: HashMap<String, PathBuf>,
}

/// A pair of a platform and its compatible programs.
#[derive(Debug, Clone)]
pub(crate) struct PlatformAndPrograms {
    /// The platform to execute the programs on.
    pub(crate) vm: PlatformDefinition,
    /// THe kind of the fault proof virtual machine.
    pub(crate) vm_kind: PlatformKind,
    /// The fault proof programs and their names.
    pub(crate) programs: HashMap<ProgramKind, FPPDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_fp_registry() {
        let ser = toml::to_string(&*FP_REGISTRY).unwrap();
        let de: FPRegistry = toml::from_str(&ser).unwrap();
        assert_eq!(*FP_REGISTRY, de);
    }
}
