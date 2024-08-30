//! Contains the implementation of the [Platform] trait for the Cannon virtual machine.

use super::Platform;
use crate::registry::program::{Program, ProgramHostInputs};
use async_trait::async_trait;
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
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
impl Platform for Cannon {
    async fn load_elf(&self, elf_path: &Path, out: &Path) -> Result<()> {
        let result = Command::new(self.binary.display().to_string())
            .arg("load-elf")
            .arg("--path")
            .arg(elf_path)
            .arg("--out")
            .arg(out)
            .output()
            .await?;

        ensure!(
            result.status.success(),
            "Failed to load ELF file into Cannon: {}",
            result.status
        );

        Ok(())
    }

    async fn run(
        &self,
        inputs: &ProgramHostInputs,
        program: Arc<dyn Program + Send + Sync>,
        workdir: &Path,
    ) -> Result<u8> {
        let host_args = program.host_cmd(inputs)?;
        dbg!(&self.binary, &host_args);
        let result = Command::new(self.binary.display().to_string())
            .arg("run")
            .arg("--info-at")
            .arg("%10000000")
            .arg("--proof-at")
            .arg("never")
            .arg("--input")
            .arg("state.json")
            .arg("--")
            .args(host_args)
            .current_dir(workdir)
            .output()
            .await?;

        // Dump logs if the command failed.
        if !result.status.success() {
            std::io::stdout().write_all(&result.stdout)?;
            std::io::stderr().write_all(&result.stderr)?;
        }

        Ok(result.status.code().ok_or(eyre!("Missing exit code"))? as u8)
    }
}
