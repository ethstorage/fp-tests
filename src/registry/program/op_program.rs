//! Contains the implementation of [ProgramCommand] for `op-program`

use std::path::PathBuf;

use super::Program;
use crate::registry::program::{ProgramHostInputs, ProgramHostSource};
use color_eyre::Result;

/// The `op-program` fault proof program.
pub(crate) struct OpProgram {
    pub(crate) binary: PathBuf,
    pub(crate) server_mode: bool,
}

impl OpProgram {
    /// Create a new `OpProgram` instance.
    pub(crate) fn new(binary: PathBuf, server_mode: bool) -> Self {
        Self {
            binary,
            server_mode,
        }
    }
}

/// The `op-program` fault proof program.
impl Program for OpProgram {
    fn host_cmd(&self, inputs: &ProgramHostInputs) -> Result<Vec<String>> {
        let mut cmd = vec![
            self.binary.display().to_string(),
            "--l1.head".to_string(),
            inputs.fixture_inputs.l1_head.to_string(),
            "--l2.head".to_string(),
            inputs.fixture_inputs.l2_head.to_string(),
            "--l2.outputroot".to_string(),
            inputs.fixture_inputs.l2_output_root.to_string(),
            "--l2.claim".to_string(),
            inputs.fixture_inputs.l2_claim.to_string(),
            "--l2.blocknumber".to_string(),
            inputs.fixture_inputs.l2_block_number.to_string(),
            "--rollup.config".to_string(),
            inputs.rollup_cfg_path.display().to_string(),
            "--l2.genesis".to_string(),
            inputs.genesis_path.display().to_string(),
        ];

        // Set up the server mode flag.
        if self.server_mode {
            cmd.push("--server".to_string());
        }

        // Set up the data source flags.
        match inputs.source.clone() {
            ProgramHostSource::Disk { path } => {
                cmd.extend(vec!["--datadir".to_string(), path.display().to_string()]);
            }
            ProgramHostSource::Rpc {
                l1,
                l1_beacon,
                l2,
                path,
            } => {
                cmd.extend(vec![
                    "--l1".to_string(),
                    l1,
                    "--l1.beacon".to_string(),
                    l1_beacon,
                    "--l2".to_string(),
                    l2,
                    "--datadir".to_string(),
                    path.display().to_string(),
                ]);
            }
        }

        Ok(cmd)
    }
}
