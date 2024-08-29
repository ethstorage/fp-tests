//! Contains the implementation of the [Platform] trait for the Cannon virtual machine.

use super::Platform;
use crate::{fixture::ProgramHostInputs, registry::program::Program, util::run_cmd};
use async_trait::async_trait;
use color_eyre::{eyre::eyre, Result};
use std::path::Path;
use tokio::process::Command;
use tracing::debug;

/// The native platform.
pub(crate) struct Native;

#[async_trait]
impl<PROG> Platform<PROG> for Native
where
    PROG: Program + Send + Sync,
{
    async fn load_elf(&self, _: &Path, _: &Path) -> Result<()> {
        debug!(target: "native-platform", "Native platform; No need to load ELF file");
        Ok(())
    }

    async fn run(&self, inputs: &ProgramHostInputs, program: &PROG, workdir: &Path) -> Result<u8> {
        let host_cmd = program.host_cmd(inputs)?;

        // On the native platform, the host program is ran verbatim.
        let mut cmd = Command::new(&host_cmd.get(0).ok_or(eyre!("Missing host binary"))?);
        let result = run_cmd(
            cmd.args(
                host_cmd
                    .get(1..)
                    .ok_or(eyre!("Missing host binary arguments"))?,
            )
            .current_dir(workdir),
        )
        .await?;

        Ok(result.code().ok_or(eyre!("Missing exit code"))? as u8)
    }
}
