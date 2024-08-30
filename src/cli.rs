//! CLI definition for `fpt`.

use crate::{
    generator::TestCaseGenerator,
    pipeline::TestPipeline,
    registry::{platform::PlatformKind, program::ProgramKind, FP_REGISTRY},
};
use alloy_primitives::B256;
use clap::{ArgAction, Args, Parser, Subcommand};
use cli_table::{Cell, Style, Table};
use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Result};
use tracing::Level;

/// The CLI options for `fpt`.
#[derive(Parser, Debug, Clone)]
pub(crate) struct Cli {
    /// Verbosity level (0-2)
    #[arg(long, short, action = ArgAction::Count)]
    pub v: u8,
    /// The subcommand to run.
    #[clap(subcommand)]
    pub subcommand: CliSubcommand,
}

impl Cli {
    /// Parses the CLI arguments and runs the application.
    pub(crate) async fn run(self) -> Result<()> {
        let registry = FP_REGISTRY;
        match self.subcommand {
            CliSubcommand::Generate(cfg) => {
                TestCaseGenerator::new(&cfg)?.generate().await?;
            }
            CliSubcommand::Test(cfg) => {
                let matrix = registry.resolve_matrix(Some(&cfg));
                TestPipeline::new(&cfg, matrix)
                    .setup()
                    .await?
                    .run()
                    .await?
                    .teardown()
                    .await?
            }
            CliSubcommand::Matrix => {
                let matrix = registry.resolve_matrix(None);

                let mut table_contents = Vec::with_capacity(matrix.len());
                matrix.iter().for_each(|pair| {
                    let programs = pair
                        .programs
                        .iter()
                        .map(|(prog, prog_def)| {
                            let mut name = prog.magenta().to_string();
                            if prog_def.default {
                                name = format!("{name} (default)");
                            }
                            name
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                        .to_string();

                    let mut vm_name = pair.vm_kind.to_string().green().to_string();
                    if pair.vm.default {
                        vm_name = format!("{vm_name} (default)");
                    }
                    table_contents.push(vec![vm_name.cell(), programs.cell()]);
                });

                let table = table_contents
                    .table()
                    .title(vec!["Platform".cell(), "Programs".cell()])
                    .bold(true);
                cli_table::print_stdout(table)?;
            }
        }
        Ok(())
    }

    /// Initializes the tracing subscriber
    ///
    /// # Arguments
    /// * `verbosity_level` - The verbosity level (0-2)
    ///
    /// # Returns
    /// * `Result<()>` - Ok if successful, Err otherwise.
    pub(crate) fn init_tracing_subscriber(self) -> Result<Self> {
        color_eyre::install()?;

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(match self.v {
                0 => Level::INFO,
                1 => Level::DEBUG,
                _ => Level::TRACE,
            })
            .finish();

        tracing::subscriber::set_global_default(subscriber).map_err(|e| eyre!(e))?;

        Ok(self)
    }
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum CliSubcommand {
    /// Lists the available FPVMs and FPPs.
    Matrix,
    /// Runs a set of tests.
    Test(TestConfig),
    /// Generate a new test case.
    Generate(GenerateConfig),
}

#[derive(Args, Debug, Clone)]
pub(crate) struct TestConfig {
    /// The test to run (glob pattern supported)
    #[clap(short, long)]
    pub(crate) test: Option<String>,
    /// The FPVM to run the tests on (multiple deliniated by commas)
    #[clap(short, long)]
    pub(crate) vm: Option<Vec<PlatformKind>>,
    /// The FPP to run the tests on (multiple deliniated by commas)
    #[clap(short, long)]
    pub(crate) program: Option<Vec<ProgramKind>>,
    /// The partition of tests to run
    #[clap(long)]
    pub(crate) partition: Option<String>,
    /// The number of active workers (default = 4).
    #[clap(long, default_value = "4")]
    pub(crate) workers: usize,
}

#[derive(Args, Debug, Clone)]
pub(crate) struct GenerateConfig {
    /// The name of the test case
    #[clap(short, long)]
    pub(crate) name: String,
    /// The L1 RPC
    #[clap(long, env = "L1_RPC")]
    pub(crate) l1_rpc: String,
    /// The L1 beacon RPC
    #[clap(long, env = "L1_BEACON_RPC")]
    pub(crate) l1_beacon_rpc: String,
    /// The L2 rollup node RPC
    #[clap(long, env = "L2_NODE_RPC")]
    pub(crate) l2_node_rpc: String,
    /// The L2 RPC
    #[clap(long, env = "L2_RPC")]
    pub(crate) l2_rpc: String,
    /// The L2 block number that the test case is for.
    #[clap(long, env = "L2_BLOCK")]
    pub(crate) l2_block: u64,
    /// The L2 claim.
    #[clap(long, env = "L2_CLAIM")]
    pub(crate) l2_claim: Option<B256>,
    /// The starting L2 output root.
    #[clap(long, env = "L2_OUTPUT_ROOT")]
    pub(crate) l2_output_root: Option<B256>,
    /// The starting L2 head hash.
    #[clap(long, env = "L2_HEAD")]
    pub(crate) l2_head: Option<B256>,
    /// The L1 block at the creation of the dispute.
    #[clap(long, env = "L1_HEAD")]
    pub(crate) l1_head: Option<B256>,
    /// The L2 chain ID.
    #[clap(long, env = "L2_CHAIN_ID")]
    pub(crate) l2_chain_id: Option<u64>,
}
