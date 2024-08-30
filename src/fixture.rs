//! Contains the definition for the test fixture format.

use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TestFixture {
    /// The fixture metadata.
    #[serde(flatten)]
    pub(crate) metadata: FixtureMetadata,
    /// The inputs to the fault proof program.
    pub(crate) inputs: FixtureInputs,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct FixtureMetadata {
    /// The name of the test fixture.
    pub(crate) name: String,
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
