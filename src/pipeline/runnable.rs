//! Contains the test runner for `fpt`.

use crate::{
    fixture::FixtureMetadata,
    registry::{
        platform::PlatformKind,
        program::{ProgramHostInputs, ProgramKind},
        FPPDefinition, PlatformAndPrograms,
    },
};
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use std::{fs, sync::Arc};
use tempfile::tempdir;
use tokio::process::Command;

/// An individual test case runner.
#[derive(Clone)]
pub(crate) struct RunnableTest {
    /// The test fixture metadata.
    pub(crate) fixture_meta: Arc<FixtureMetadata>,
    /// The inputs for the test case.
    pub(crate) inputs: Arc<ProgramHostInputs>,
    /// The platform to run the test on.
    pub(crate) platform_kind: PlatformKind,
    /// The platform definition.
    pub(crate) platform_definition: Arc<PlatformAndPrograms>,
    /// The program to run.
    pub(crate) program_kind: ProgramKind,
    /// The program definition.
    pub(crate) program_definition: Arc<FPPDefinition>,
}

impl RunnableTest {
    /// Create a new [RunnableTest].
    pub(crate) fn new(
        fixture_meta: Arc<FixtureMetadata>,
        inputs: Arc<ProgramHostInputs>,
        platform: PlatformKind,
        platform_definition: Arc<PlatformAndPrograms>,
        program: ProgramKind,
        program_definition: Arc<FPPDefinition>,
    ) -> Self {
        Self {
            fixture_meta,
            inputs,
            platform_kind: platform,
            platform_definition,
            program_kind: program,
            program_definition,
        }
    }

    /// Run the test case and return whether or not it passed.
    pub(crate) async fn run(&self) -> Result<bool> {
        // Create a temporary directory for the test case.
        let workdir = tempdir()?;

        // Grab the client artifact for the program.
        let client_artifact = self
            .program_definition
            .build
            .get_artifact("client")
            .ok_or(eyre!("Failed to get client artifact"))?;

        let platform = self.platform_definition.vm_kind.get_platform(
            self.platform_definition
                .vm
                .build
                .as_ref()
                .map(|b| b.get_artifact("vm"))
                .flatten(),
        )?;
        let program = self.program_kind.get_program(
            self.program_definition
                .build
                .get_artifact("host")
                .ok_or(eyre!("No host artifact"))?,
        );

        // Load the binary into the platform's state format.
        platform
            .load_elf(client_artifact.as_path(), workdir.path())
            .await?;

        // Run the program on the platform.
        let result = platform
            .run(self.inputs.as_ref(), program, workdir.path())
            .await?;

        Ok(result == self.fixture_meta.expected_status)
    }

    /// Decompresses the files within the test fixture.
    pub(crate) async fn decompress_fixture(&self) -> Result<()> {
        // Grab the fixture directory.
        let fixture_dir = self
            .inputs
            .genesis_path
            .parent()
            .ok_or(eyre!("Fixture at top-level directory"))?;

        // Decompress the genesis file
        let decompress_status = Command::new("zstd")
            .arg("-d")
            .arg(fixture_dir.join("genesis.json.zst"))
            .current_dir(fixture_dir)
            .output()
            .await?;
        ensure!(
            decompress_status.status.success(),
            "Failed to decompress genesis file"
        );

        // Decompress witness database
        let decompress_status = Command::new("tar")
            .arg("--zstd")
            .arg("-xvf")
            .arg(fixture_dir.join("witness-db.tar.zst"))
            .current_dir(fixture_dir)
            .output()
            .await?;
        ensure!(
            decompress_status.status.success(),
            "Failed to decompress witness database"
        );

        Ok(())
    }

    /// Clean up the decompressed fixture files.
    pub(crate) async fn teardown(&self) -> Result<()> {
        // Grab the fixture directory.
        let fixture_dir = self
            .inputs
            .genesis_path
            .parent()
            .ok_or(eyre!("Fixture at top-level directory"))?;

        // Remove the decompressed files.
        fs::remove_file(fixture_dir.join("genesis.json"))?;
        fs::remove_dir_all(fixture_dir.join("witness-db"))?;

        Ok(())
    }
}
