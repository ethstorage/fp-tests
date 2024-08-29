//! Contains the [Platform] trait, which defines the interface for a fault proof virtual machine.

use super::program::Program;
use crate::fixture::ProgramHostInputs;
use async_trait::async_trait;
use color_eyre::{eyre::bail, Result};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path, str::FromStr};

pub(crate) mod cannon;
pub(crate) mod native;

/// The minimal interface for a fault proof virtual machine binary.
#[async_trait]
pub(crate) trait Platform<PROG: Program> {
    /// Load a program into the FPVM's state format.
    ///
    /// ## Takes
    /// - `elf_path` - The path to the ELF file to load.
    ///
    /// ## Returns
    /// - `Result<()>` - Ok if successful, Err otherwise.
    async fn load_elf(&self, elf_path: &Path, out: &Path) -> Result<()>;

    /// Runs the loaded program on the FPVM.
    ///
    /// ## Takes
    /// - `program_inputs` - The inputs to the program.
    /// - `program` - The program command specification.
    ///
    /// ## Returns
    /// - `Result<StatusCode>` - Ok if successful, Err otherwise.
    async fn run(
        &self,
        program_inputs: &ProgramHostInputs,
        program: &PROG,
        workdir: &Path,
    ) -> Result<u8>;
}

/// Supported platform kinds.
#[derive(Default, Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PlatformKind {
    /// Native platform
    #[default]
    Native,
    /// `cannon`
    Cannon,
    /// `asterisc`
    Asterisc,
}

// impl PlatformKind {
//     /// Returns the [Platform] implementation for the given kind.
//     pub(crate) fn get_platform<PROG>(
//         &self,
//         binary: Option<PathBuf>,
//     ) -> Result<Box<dyn Platform<PROG>>>
//     where
//         PROG: Program + Send + Sync,
//     {
//         match self {
//             Self::Native => Ok(Box::new(native::Native)),
//             Self::Cannon => {
//                 let plat = cannon::Cannon::new(
//                     binary.ok_or(eyre!("Binary required for `cannon` platform"))?,
//                 );
//                 Ok(Box::new(plat))
//             }
//             Self::Asterisc => unimplemented!(),
//         }
//     }
// }

impl FromStr for PlatformKind {
    type Err = color_eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "native" => Ok(Self::Native),
            "cannon" => Ok(Self::Cannon),
            "asterisc" => Ok(Self::Asterisc),
            _ => bail!("Unknown program kind: {}", s),
        }
    }
}

impl From<String> for PlatformKind {
    fn from(s: String) -> Self {
        s.parse().unwrap_or_else(|e| panic!("{e}"))
    }
}

impl Display for PlatformKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "native"),
            Self::Cannon => write!(f, "cannon"),
            Self::Asterisc => write!(f, "asterisc"),
        }
    }
}