//! Contains the implementation of the [Platform] trait for the Cannon virtual machine.

use super::Platform;
use crate::{fixture::ProgramHostInputs, registry::program::Program, util::run_cmd};
use async_trait::async_trait;
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// The Cannon virtual machine.
pub(crate) struct Cannon {
    /// The path to the Cannon binary.
    binary: PathBuf,
}

impl Cannon {
    /// Create a new `Cannon` instance.
    pub(crate) fn new(binary: PathBuf) -> Self {
        Self { binary }
    }
}

#[async_trait]
impl<PROG> Platform<PROG> for Cannon
where
    PROG: Program + Send + Sync,
{
    async fn load_elf(&self, elf_path: &Path, out: &Path) -> Result<()> {
        let mut cmd = Command::new(self.binary.display().to_string());
        let result = run_cmd(
            cmd.arg("load-elf")
                .arg("--path")
                .arg(elf_path)
                .arg("--out")
                .arg(out),
        )
        .await?;

        ensure!(
            result.success(),
            "Failed to load ELF file into Cannon: {}",
            result
        );

        Ok(())
    }

    async fn run(&self, inputs: &ProgramHostInputs, program: &PROG, workdir: &Path) -> Result<u8> {
        let mut cmd = Command::new(self.binary.display().to_string());
        let host_args = program.host_cmd(inputs)?;
        let result = run_cmd(
            cmd.arg("run")
                .arg("--info-at")
                .arg("%10000000")
                .arg("--proof-at")
                .arg("never")
                .arg("--input")
                .arg("state.json")
                .arg("--")
                .args(host_args)
                .current_dir(workdir),
        )
        .await?;

        Ok(result.code().ok_or(eyre!("Missing exit code"))? as u8)
    }
}
