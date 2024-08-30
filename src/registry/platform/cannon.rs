//! Contains the implementation of the [Platform] trait for the Cannon virtual machine.

use super::Platform;
use crate::registry::program::{Program, ProgramHostInputs};
use async_trait::async_trait;
use color_eyre::{eyre::ensure, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
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
    async fn load_elf(&self, elf_path: &Path, workdir: &Path) -> Result<()> {
        let result = Command::new(self.binary.display().to_string())
            .arg("load-elf")
            .arg("--path")
            .arg(elf_path)
            .arg("--out")
            .arg(workdir.join("state.json"))
            .arg("--meta")
            .arg(workdir.join("meta.json"))
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
        Command::new(self.binary.display().to_string())
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

        // Read `out.json`
        let output = serde_json::from_slice::<PartialCannonOutput>(
            fs::read(workdir.join("out.json"))?.as_slice(),
        )?;
        ensure!(output.exited, "Program did not exit");

        Ok(output.exit)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartialCannonOutput {
    /// Whether or not the program has exited.
    exited: bool,
    /// The exit code of the program.
    exit: u8,
}
