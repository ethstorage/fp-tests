//! Contains the definition for the test fixture format.

use alloy_primitives::B256;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TestFixture {
    /// The name of the test fixture.
    pub(crate) name: String,
    /// The inputs to the fault proof program.
    pub(crate) inputs: FixtureInputs,
    /// The expected status byte of the program execution.
    pub(crate) expected_status: u8,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FixtureInputs {
    /// The L1 head hash, containing the data required to derive the L2 chain at the height of the `l2_claim`.
    pub(crate) l1_head: B256,
    /// The block number of the L2 claim.
    pub(crate) l2_block_number: u64,
    /// The L2 claim.
    pub(crate) l2_claim: B256,
    /// The starting, trusted L2 output root.
    pub(crate) l2_output_root: B256,
    /// The L2 head hash, corresponding to the `l2_output_root`.
    pub(crate) l2_head: B256,
    /// The L2 chain ID.
    pub(crate) l2_chain_id: u64,
}

/// The inputs to the program host binary.
pub(crate) struct ProgramHostInputs {
    /// The basic inputs to the program host.
    pub(crate) fixture_inputs: FixtureInputs,
    /// The path to the `rollup.json` file.
    pub(crate) rollup_cfg_path: PathBuf,
    /// The path to the `genesis.json` file.
    pub(crate) genesis_path: PathBuf,
    /// The data sources for the fixture.
    pub(crate) source: FixtureSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureSource {
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

impl Default for FixtureSource {
    fn default() -> Self {
        Self::Disk {
            path: Default::default(),
        }
    }
}
