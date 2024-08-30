//! Contains the [TestCaseGenerator], which facilitates the creation of test cases from the reference program.

use crate::{
    cli::GenerateConfig,
    fixture::{FixtureInputs, FixtureMetadata, TestFixture},
    registry::{
        platform::{native::Native, Platform},
        program::{op_program::OpProgram, ProgramHostInputs, ProgramHostSource, ProgramKind},
        FP_REGISTRY,
    },
};
use alloy_primitives::{B256, U64};
use alloy_provider::{network::Ethereum, Provider, ReqwestProvider};
use alloy_rpc_types::BlockTransactionsKind;
use alloy_transport_http::reqwest::Url;
use color_eyre::{
    eyre::{ensure, eyre},
    Result,
};
use std::{fs, path::PathBuf, sync::Arc};
use tempfile::{tempdir, TempDir};
use tokio::process::Command;
use tracing::info;

/// The name of the chain configuration artifact on the kurtosis devnet.
pub(crate) const CHAIN_CONFIG_ARTIFACT: &str = "op-genesis-configs";

/// The name of the witness database directory.
pub(crate) const WITNESS_DB_DIR_NAME: &str = "witness-db";

/// The test case generator for `fpt`.
pub(crate) struct TestCaseGenerator<'a> {
    /// The [GenerateConfig] for the generator.
    cfg: &'a GenerateConfig,
    /// The working directory during the generation process.
    workdir: TempDir,
}

impl<'a> TestCaseGenerator<'a> {
    /// Create a new [TestCaseGenerator] with the provided [GenerateConfig].
    pub(crate) fn new(cfg: &'a GenerateConfig) -> Result<Self> {
        Ok(Self {
            cfg,
            workdir: tempdir()?,
        })
    }

    /// Generate a test case from the reference program.
    pub(crate) async fn generate(&self) -> Result<()> {
        // Download the chain configuration.
        self.download_chain_config().await?;

        // Fetch the inputs for the test case.
        let inputs = self.gather_inputs().await?;

        // Run the reference program.
        let result = self.run_reference_program(&inputs).await?;

        // Flush the test fixture and metadata to disk.
        self.flush_fixture(inputs, result).await?;

        Ok(())
    }

    /// Downlaods the chain configuration from the devnet.
    async fn download_chain_config(&self) -> Result<()> {
        info!(target: "test-gen", "Downloading chain configuration from the devnet...");
        let status = Command::new("kurtosis")
            .arg("files")
            .arg("download")
            .arg("devnet")
            .arg(CHAIN_CONFIG_ARTIFACT)
            .current_dir(self.workdir.path())
            .status()
            .await?;

        ensure!(
            status.success(),
            "Failed to download chain configuration from the devnet. Is Kurtosis running?"
        );

        info!(target: "test-gen", "Successfully downloaded chain configuration.");
        Ok(())
    }

    /// Gather the [ProgramHostInputs].
    async fn gather_inputs(&self) -> Result<ProgramHostInputs> {
        let GenerateConfig {
            l2_block,
            l2_claim,
            l1_head,
            l2_output_root,
            l2_head,
            l2_chain_id,
            ..
        } = self.cfg;
        info!(target: "test-gen", "Fetching configuration for block #{}...", l2_block);

        let l1_rpc = ReqwestProvider::<Ethereum>::new_http(Url::parse(self.cfg.l1_rpc.as_ref())?);
        let l2_node_rpc =
            ReqwestProvider::<Ethereum>::new_http(Url::parse(self.cfg.l2_node_rpc.as_ref())?);
        let l2_rpc = ReqwestProvider::<Ethereum>::new_http(Url::parse(self.cfg.l2_rpc.as_ref())?);

        let l2_claim = if let Some(l2_claim) = l2_claim {
            *l2_claim
        } else {
            info!(target: "test-gen", "Fetching L2 claim...");
            let output_at_block = l2_node_rpc
                .raw_request::<[U64; 1], OutputAtBlockResponse>(
                    "optimism_outputAtBlock".into(),
                    [U64::from(*l2_block)],
                )
                .await?;
            output_at_block.output_root
        };

        let l2_output_root = if let Some(l2_output_root) = l2_output_root {
            *l2_output_root
        } else {
            info!(target: "test-gen", "Fetching starting L2 output root...");
            let output_at_block = l2_node_rpc
                .raw_request::<[U64; 1], OutputAtBlockResponse>(
                    "optimism_outputAtBlock".into(),
                    [U64::from(*l2_block - 1)],
                )
                .await?;
            output_at_block.output_root
        };

        let l2_head = if let Some(l2_head) = l2_head {
            *l2_head
        } else {
            info!(target: "test-gen", "Fetching L2 head...");
            let l2_head = l2_rpc
                .get_block((*l2_block - 1).into(), BlockTransactionsKind::Hashes)
                .await?
                .ok_or(eyre!("Failed to fetch block."))?;
            l2_head.header.hash
        };

        let l2_chain_id = if let Some(l2_chain_id) = l2_chain_id {
            *l2_chain_id
        } else {
            info!(target: "test-gen", "Fetching L2 chain ID...");
            l2_rpc.get_chain_id().await?
        };

        let l1_head = if let Some(l1_head) = l1_head {
            *l1_head
        } else {
            info!(target: "test-gen", "Fetching L1 head...");
            // First, fetch the output root response for the starting L2 output root.
            let output_at_block = l2_node_rpc
                .raw_request::<[U64; 1], OutputAtBlockResponse>(
                    "optimism_outputAtBlock".into(),
                    [U64::from(*l2_block)],
                )
                .await?;
            // Use an L1 head hash 25 blocks ahead of the L1 origin block of the
            // L1 origin of the disputed block.
            let l1_head = l1_rpc
                .get_block(
                    (output_at_block.block_ref.l1origin.number + 25).into(),
                    BlockTransactionsKind::Hashes,
                )
                .await?
                .ok_or(eyre!("Failed to fetch block."))?;
            l1_head.header.hash
        };

        // Fetch chain configuration paths.
        let rollup_cfg_path = self
            .workdir
            .path()
            .join(CHAIN_CONFIG_ARTIFACT)
            .join("rollup.json");
        let genesis_path = self
            .workdir
            .path()
            .join(CHAIN_CONFIG_ARTIFACT)
            .join("genesis.json");

        Ok(ProgramHostInputs {
            fixture_inputs: FixtureInputs {
                l2_block_number: *l2_block,
                l1_head,
                l2_claim,
                l2_output_root,
                l2_head,
                l2_chain_id,
            },
            rollup_cfg_path,
            genesis_path,
            source: ProgramHostSource::Rpc {
                l1: self.cfg.l1_rpc.clone(),
                l1_beacon: self.cfg.l1_beacon_rpc.clone(),
                l2: self.cfg.l2_rpc.clone(),
                path: WITNESS_DB_DIR_NAME.into(),
            },
        })
    }

    /// Runs the reference program with the given [FixtureInputs].
    async fn run_reference_program(&self, inputs: &ProgramHostInputs) -> Result<u8> {
        // Fetch the reference program definition from the registry.
        let registry = FP_REGISTRY;
        let ref_program_def = registry.program.get(&ProgramKind::default()).ok_or(eyre!(
            "Failed to find program definition for reference program."
        ))?;

        // Try to build the reference program, if the artifact is not already present.
        ref_program_def.build.try_build().await?;
        let program_bin = ref_program_def
            .build
            .get_artifact("host")
            .ok_or(eyre!("Artifact not found"))?;

        // Run the program.
        let native_program = Arc::new(OpProgram::new(program_bin, false));
        let result = Native
            .run(&inputs, native_program, self.workdir.path())
            .await?;
        info!(target: "test-gen", "Successfully executed reference program on the native platform. Exit status: {result}");

        Ok(result)
    }

    /// Flushes the [TestFixture] and metadata to disk.
    async fn flush_fixture(&self, inputs: ProgramHostInputs, result: u8) -> Result<()> {
        let fixture_path = PathBuf::from("./tests").join(self.cfg.name.clone());
        fs::create_dir_all(&fixture_path)?;

        // Write the test fixture to disk.
        let fixture = TestFixture {
            metadata: FixtureMetadata {
                name: self.cfg.name.clone(),
                expected_status: result,
            },
            inputs: inputs.fixture_inputs,
        };
        fs::write(
            fixture_path.join("fixture.toml").as_path(),
            toml::to_string_pretty(&fixture)?,
        )?;
        info!(target: "test-gen", "Wrote test fixture to disk.");

        // Gzip the witness directory
        info!(target: "test-gen", "Compressing witness database...");
        let status = Command::new("tar")
            .arg("--zstd")
            .arg("-cf")
            .arg(format!("{}.tar.zst", WITNESS_DB_DIR_NAME))
            .arg(WITNESS_DB_DIR_NAME)
            .current_dir(self.workdir.path().display().to_string())
            .status()
            .await?;
        ensure!(status.success(), "Failed to compress witness database.");
        info!(target: "test-gen", "Compressed witness database successfully.");

        // Copy the witness DB archive into the fixture.
        fs::copy(
            self.workdir
                .path()
                .join(format!("{}.tar.zst", WITNESS_DB_DIR_NAME)),
            fixture_path.join(format!("{}.tar.zst", WITNESS_DB_DIR_NAME)),
        )?;
        info!(target: "test-gen", "Copied witness database archive into test fixture.");

        // Copy the genesis and rollup configuration files into the fixture.
        fs::copy(
            self.workdir
                .path()
                .join(CHAIN_CONFIG_ARTIFACT)
                .join("genesis.json"),
            fixture_path.join("genesis.json"),
        )?;
        fs::copy(
            self.workdir
                .path()
                .join(CHAIN_CONFIG_ARTIFACT)
                .join("rollup.json"),
            fixture_path.join("rollup.json"),
        )?;
        info!(target: "test-gen", "Copied chain configuration files into test fixture.");

        // Compress the genesis JSON file.
        let status = Command::new("zstd")
            .arg("genesis.json")
            .current_dir(&fixture_path)
            .status()
            .await?;
        ensure!(status.success(), "Failed to compress genesis.json.");
        info!(target: "test-gen", "Compressed genesis.json successfully.");

        // Remove the uncompressed genesis JSON file.
        fs::remove_file(fixture_path.join("genesis.json").as_path())?;

        Ok(())
    }
}

/// Partial response for the `optimism_outputAtBlock` RPC.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputAtBlockResponse {
    pub(crate) output_root: B256,
    pub(crate) block_ref: MinL2BlockRef,
}

/// Partial response for the `optimism_outputAtBlock` RPC.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinL2BlockRef {
    pub(crate) l1origin: MinL1BlockRef,
}

/// Partial response for the `optimism_outputAtBlock` RPC.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinL1BlockRef {
    pub(crate) number: u64,
}
