//! Contains the implementation of the [Platform] trait for the Cannon virtual machine.

use super::Platform;
use crate::registry::program::{Program, ProgramHostInputs};
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Result};
use std::{io::Write, path::Path, sync::Arc};
use tokio::process::Command;
use tracing::debug;

/// The native platform.
pub(crate) struct Native;

#[async_trait]
impl Platform for Native {
    async fn load_elf(&self, _: &Path, _: &Path) -> Result<()> {
        debug!(target: "native-platform", "Native platform; No need to load ELF file");
        Ok(())
    }

    async fn run(
        &self,
        inputs: &ProgramHostInputs,
        program: Arc<dyn Program + Send + Sync>,
        workdir: &Path,
    ) -> Result<u8> {
        let host_cmd = program.host_cmd(inputs)?;

        // On the native platform, the host program is ran verbatim.
        let result = Command::new(&host_cmd.get(0).ok_or(eyre!("Missing host binary"))?)
            .args(
                host_cmd
                    .get(1..)
                    .ok_or(eyre!("Missing host binary arguments"))?,
            )
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
