//! Contains the [Program] trait, which defines the interface for a fault proof program.

use crate::fixture::FixtureInputs;
use color_eyre::{eyre::bail, Result};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf, str::FromStr, sync::Arc};

pub(crate) mod op_program;

/// The minimal interface for a fault proof program host binary.
pub(crate) trait Program {
    /// Returns the arguments for the host program binary with the given inputs.
    ///
    /// ## Takes
    /// - `inputs` - The inputs to the program.
    ///
    /// ## Returns
    /// - `Result<Vec<String>>` - Ok if successful, Err otherwise.
    fn host_cmd(&self, inputs: &ProgramHostInputs) -> Result<Vec<String>>;
}

/// Supported program kinds.
#[derive(Default, Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProgramKind {
    /// `op-program` (native)
    #[default]
    OpProgramNative,
    /// `op-program` (mips / cannon)
    OpProgramMips,
    /// `op-program` (riscv / asterisc)
    OpProgramRiscv,
    /// `kona` (native)
    KonaNative,
    /// `kona` (riscv / asterisc)
    KonaRiscv,
}

impl ProgramKind {
    pub(crate) fn get_program(&self, bin_path: PathBuf) -> Arc<dyn Program + Send + Sync> {
        match self {
            Self::OpProgramNative => Arc::new(op_program::OpProgram::new(bin_path, false)),
            Self::OpProgramMips | Self::OpProgramRiscv => {
                Arc::new(op_program::OpProgram::new(bin_path, true))
            }
            _ => todo!(),
        }
    }
}

impl FromStr for ProgramKind {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "op-program-native" => Ok(Self::OpProgramNative),
            "op-program-mips" => Ok(Self::OpProgramMips),
            "op-program-riscv" => Ok(Self::OpProgramRiscv),
            "kona-native" => Ok(Self::KonaNative),
            "kona-riscv" => Ok(Self::KonaNative),
            _ => bail!("Unknown program kind: {}", s),
        }
    }
}

impl From<String> for ProgramKind {
    fn from(s: String) -> Self {
        s.parse().unwrap_or_else(|e| panic!("{e}"))
    }
}

impl Display for ProgramKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpProgramNative => write!(f, "op-program-native"),
            Self::OpProgramMips => write!(f, "op-program-mips"),
            Self::OpProgramRiscv => write!(f, "op-program-riscv"),
            Self::KonaNative => write!(f, "kona-native"),
            Self::KonaRiscv => write!(f, "kona-riscv"),
        }
    }
}

/// The inputs to the program host binary.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProgramHostInputs {
    /// The basic inputs to the program host.
    pub(crate) fixture_inputs: FixtureInputs,
    /// The path to the `rollup.json` file.
    pub(crate) rollup_cfg_path: PathBuf,
    /// The path to the `genesis.json` file.
    pub(crate) genesis_path: PathBuf,
    /// The data sources for the fixture.
    pub(crate) source: ProgramHostSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProgramHostSource {
    /// Disk-backed preimage server.
    Disk { path: PathBuf },
    /// RPC-backed preimage server.
    Rpc {
        /// The L1 RPC endpoint
        l1: String,
        /// The L1 beacon RPC endpoint
        l1_beacon: String,
        /// The L2 RPC endpoint
        l2: String,
        /// The witness database path
        path: PathBuf,
    },
}

impl Default for ProgramHostSource {
    fn default() -> Self {
        Self::Disk {
            path: Default::default(),
        }
    }
}
