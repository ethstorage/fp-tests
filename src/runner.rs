//! Contains the test runner for `fpt`.

use crate::{
    cli::TestConfig,
    fixture::{ProgramHostInputs, TestFixture},
    registry::{platform::Platform, program::Program, PlatformAndPrograms},
};
use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Result};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    time::{self, Duration},
};
use tempfile::tempdir;
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinSet,
};
use tracing::info;

/// The test runner for `fpt`.
pub(crate) struct TestRunner<'a> {
    /// The [TestConfig] for the runner.
    pub(crate) cfg: &'a TestConfig,
    /// The matrix of platforms and programs to run.
    pub(crate) matrix: Vec<PlatformAndPrograms<'a>>,
}

impl<'a> TestRunner<'a> {
    pub(crate) fn new(cfg: &'a TestConfig, matrix: Vec<PlatformAndPrograms<'a>>) -> Self {
        Self { cfg, matrix }
    }

    /// Run the configured tests on the matrix of platforms and programs.
    pub(crate) async fn run(&self) -> Result<()> {
        // Attempt to build all platforms and programs in the matrix.
        self.try_build_matrix().await?;

        // Gather test vectors to execute.
        let tests = self.gather_tests()?;

        // Inform the user of the number of tests to run.
        let num_tests = tests.len() * self.matrix.iter().map(|p| p.programs.len()).sum::<usize>();
        println!(
            "{} {} tests across {} platforms...",
            "Starting".green().bold(),
            num_tests.blue(),
            self.matrix.len().blue()
        );

        // Shared progress indicator
        let multi_progress = Arc::new(Mutex::new(MultiProgress::new()));

        // Execute the tests in a parallel worker pool.
        let semaphore = Arc::new(Semaphore::new(self.cfg.workers));
        let mut join_set = JoinSet::new();
        for test in tests {
            self.matrix.iter().for_each(|platform| {
                // Extract the platform kind for the test.
                let platform_kind = platform.vm_kind;

                platform
                    .programs
                    .iter()
                    .for_each(|(program_kind, program)| {
                        let semaphore = semaphore.clone();
                        let multi_progress = multi_progress.clone();

                        let test = test.clone();
                        let program_build = program.build.clone();
                        let program_kind = *program_kind;

                        join_set.spawn(async move {
                            // Aquire a permit on the semaphore. Once the permit is aquired, we can begin
                            // running the test case.
                            let _permit = semaphore.acquire().await?;

                            // Set up the progress bar.
                            let pb = multi_progress.lock().await.add(ProgressBar::new_spinner());
                            pb.set_style(
                                ProgressStyle::with_template(
                                    "{prefix:.bold} {spinner} {wide_msg}",
                                )?
                                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
                            );
                            pb.set_prefix(format!(
                                "{}::{}::{}",
                                platform_kind.magenta(),
                                program_kind.cyan(),
                                test.name.blue()
                            ));
                            pb.enable_steady_tick(Duration::from_millis(50));
                            pb.set_message("Executing test...");

                            // Run the test on the platform.
                            let start_time = time::Instant::now();
                            // TestCaseRun::new(
                            //     &test.inputs,
                            //     Default::default(), // TODO: Client artifact.
                            //     todo!(),
                            //     todo!(),
                            // )
                            // .run()
                            // .await?;

                            pb.finish_with_message(format!(
                                "Done {} 🕐 {} {} Status: {}",
                                "|".black(),
                                HumanDuration(start_time.elapsed()).green(),
                                "|".black(),
                                "PASS".green() // TODO: Dynamic status.
                            ));

                            Ok::<_, color_eyre::Report>(())
                        });
                    });
            });
        }

        // Join all test tasks.
        // TODO: Test summary.
        while let Some(result) = join_set.join_next().await {
            result??;
        }

        Ok(())
    }

    /// Attempts to build all platforms and programs in the matrix.
    async fn try_build_matrix(&self) -> Result<()> {
        for platform in self.matrix.iter() {
            for (program_name, program) in platform.programs.iter() {
                info!(target: "test-runner", "Building program: {}", program_name);
                program.build.try_build().await?;
            }

            if let Some(vm_build) = platform.vm.build.as_ref() {
                info!(target: "test-runner", "Building platform: {}", platform.vm_kind);
                vm_build.try_build().await?;
            }
        }
        Ok(())
    }

    /// Gathers the [TestFixture]s to execute.
    ///
    /// TODO: Lazy loading of witness data!
    fn gather_tests(&self) -> Result<Vec<Arc<TestFixture>>> {
        // TODO: Custom tests dir.
        let test_files = fs::read_dir("./tests")?;

        if let Some(tests) = &self.cfg.test {
            let glob = glob::Pattern::new(tests).expect("Invalid glob pattern");
            let tests = test_files
                .filter_map(|f| {
                    let file = f.ok()?;
                    let file_name = file.file_name();

                    if glob.matches(file_name.to_str()?) {
                        serde_json::from_str(&fs::read_to_string(&file.path()).ok()?)
                            .ok()
                            .map(Arc::new)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            Ok(tests)
        } else {
            let tests = test_files
                .map(|f| {
                    serde_json::from_str(&fs::read_to_string(&f?.path())?)
                        .map(Arc::new)
                        .map_err(|e| eyre!("Failed to parse test file: {e}"))
                })
                .collect::<Result<Vec<_>>>();
            tests
        }
    }
}

/// An individual test case runner.
struct TestCaseRun<'a, PLAT: Platform<PROG>, PROG: Program> {
    /// The inputs for the test case.
    pub(crate) inputs: &'a ProgramHostInputs,
    /// The path to the client program.
    pub(crate) client_artifact: PathBuf,
    /// The platform to run the test on.
    pub(crate) platform: &'a PLAT,
    /// The program to run the test on.
    pub(crate) program: &'a PROG,
}

impl<'a, VM, P> TestCaseRun<'a, VM, P>
where
    VM: Platform<P>,
    P: Program + Send + Sync,
{
    /// Create a new [TestCaseRun].
    pub(crate) fn new(
        inputs: &'a ProgramHostInputs,
        client_artifact: PathBuf,
        platform: &'a VM,
        program: &'a P,
    ) -> Self {
        Self {
            inputs,
            client_artifact,
            platform,
            program,
        }
    }

    /// Run the test case.
    pub(crate) async fn run(&self) -> Result<u8> {
        // Create a temporary directory for the test case.
        let workdir = tempdir()?;

        // Load the binary into the platform's state format.
        self.platform
            .load_elf(self.client_artifact.as_path(), workdir.path())
            .await?;

        // Run the program on the platform.
        // TODO: Set workdir.
        // TODO: Validate output.
        self.platform
            .run(self.inputs, self.program, workdir.path())
            .await
    }
}
