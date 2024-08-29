//! Contains the [Program] trait, which defines the interface for a fault proof program.

use crate::fixture::ProgramHostInputs;
use color_eyre::{eyre::bail, Result};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

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
