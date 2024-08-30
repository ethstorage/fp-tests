//! Contains the test runner pipeline.

use crate::{
    cli::TestConfig,
    fixture::TestFixture,
    registry::{
        program::{ProgramHostInputs, ProgramHostSource},
        PlatformAndPrograms,
    },
};
use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Result};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use runnable::RunnableTest;
use std::{
    fs,
    sync::Arc,
    time::{self, Duration},
};
use tokio::{
    sync::{Mutex, Semaphore},
    task::JoinSet,
};
use tracing::info;

mod runnable;

/// The [TestPipeline] is a pipelined test runner, with [Self::setup], [Self::run], and [Self::teardown] stages.
pub(crate) struct TestPipeline<'a> {
    /// The test configuration.
    cfg: &'a TestConfig,
    /// The matrix of platforms and programs to run tests on.
    matrix: Vec<PlatformAndPrograms>,
    /// The tests to run.
    tests: Option<Vec<RunnableTest>>,
}

impl<'a> TestPipeline<'a> {
    pub(crate) fn new(cfg: &'a TestConfig, matrix: Vec<PlatformAndPrograms>) -> Self {
        Self {
            cfg,
            matrix,
            tests: None,
        }
    }

    /// Sets up the test pipeline.
    ///
    /// ## Tasks
    /// 1. Build the active platforms and programs.
    /// 2. Gather the tests that will be ran from the active matrix.
    /// 3. Decompress the compressed artifacts within the active fixture folders.
    pub(crate) async fn setup(mut self) -> Result<Self> {
        // Attempt to build all platforms and programs in the matrix.
        self.try_build_matrix().await?;

        // Gather the tests that will be ran from the active matrix.
        self.tests = Some(self.gather_tests()?);

        // Decompress the artifacts within the active fixture folders.
        self.decompress_fixtures().await?;

        Ok(self)
    }

    /// Runs the tests against the active matrix.
    ///
    /// ## Tasks
    /// 1. Schedule the tests to run in parallel in a worker pool.
    pub(crate) async fn run(self) -> Result<Self> {
        let tests = self.tests.clone().ok_or(eyre!("No tests to run"))?;
        let num_tests = tests.len();

        // Inform the cli of the number of tests to run.
        println!(
            "\n\nRunning {} tests across {} platforms...",
            num_tests.blue(),
            self.matrix.len().blue()
        );

        let multi_progress = Arc::new(Mutex::new(MultiProgress::new()));
        let semaphore = Arc::new(Semaphore::new(self.cfg.workers));
        let mut join_set = JoinSet::new();

        // Execute the tests in a parallel worker pool.
        for case in tests {
            let semaphore = semaphore.clone();
            let multi_progress = multi_progress.clone();

            join_set.spawn(async move {
                // Aquire a permit on the semaphore. Once the permit is aquired, we can begin
                // running the test case.
                let _permit = semaphore.acquire().await?;

                // Set up the progress bar.
                let pb = multi_progress.lock().await.add(ProgressBar::new_spinner());
                pb.set_style(
                    ProgressStyle::with_template("{prefix:.bold} {spinner} {wide_msg}")?
                        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
                );
                pb.set_prefix(format!(
                    "{}::{}::{}",
                    case.platform_kind.magenta(),
                    case.program_kind.cyan(),
                    case.fixture_meta.name.blue()
                ));
                pb.enable_steady_tick(Duration::from_millis(50));
                pb.set_message("Executing test...");

                let start_time = time::Instant::now();
                let pass = case.run().await?;

                // Notify the user that the test has completed.
                pb.finish_with_message(format!(
                    "{} {} Test took {} {} Status: {}",
                    "Done".green().bold(),
                    "|".black(),
                    HumanDuration(start_time.elapsed()).magenta(),
                    "|".black(),
                    if pass {
                        "PASS".green().bold().to_string()
                    } else {
                        "FAIL".red().bold().italic().to_string()
                    }
                ));

                Ok::<_, color_eyre::Report>(pass)
            });
        }

        // Join all test tasks.
        let mut num_passed = 0;
        while let Some(result) = join_set.join_next().await {
            num_passed += result?? as usize;
        }
        println!(
            "{} - {} tests {}, {} tests {}.\n",
            "Completed".bold(),
            num_passed.to_string().blue().bold(),
            "passed".green().bold(),
            (num_tests - num_passed).to_string().blue().bold(),
            "failed".red().bold()
        );

        Ok(self)
    }

    /// Cleans up the artifacts created during the test run.
    ///
    /// ## Tasks
    /// 1. Remove all uncompressed artifacts from the active fixture folders.
    pub(crate) async fn teardown(mut self) -> Result<()> {
        let tests = self.tests.take().ok_or(eyre!("No tests to run"))?;

        let unique_fixtures = tests
            .iter()
            .unique_by(|t| t.inputs.genesis_path.as_path())
            .cloned()
            .collect::<Vec<_>>();

        let progress_bar = {
            let bar = ProgressBar::new(unique_fixtures.len() as u64);
            bar.enable_steady_tick(Duration::from_millis(50));
            bar.set_message("Cleaning up decompressed fixture artifacts...");
            bar.set_style(ProgressStyle::default_bar().template("{msg} {wide_bar} {pos}/{len}")?);
            Arc::new(Mutex::new(bar))
        };

        let semaphore = Arc::new(Semaphore::new(self.cfg.workers));
        let mut join_set = JoinSet::new();

        for test in unique_fixtures.into_iter() {
            let semaphore = semaphore.clone();
            let progress_bar = progress_bar.clone();

            join_set.spawn(async move {
                // Aquire a permit on the semaphore. Once the permit is aquired, we can begin
                // deleting the test fixture artifacts.
                let _permit = semaphore.acquire().await?;

                // Decompress the fixture.
                test.teardown().await?;

                // Notify the cli that the fixture has been decompressed.
                progress_bar.lock().await.inc(1);

                Ok::<_, color_eyre::Report>(())
            });
        }

        while let Some(result) = join_set.join_next().await {
            result??;
        }

        progress_bar
            .lock()
            .await
            .finish_with_message("Deleted decompressed fixture artifacts");

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

    /// Gathers the [RunnableTest]s to execute.
    fn gather_tests(&self) -> Result<Vec<RunnableTest>> {
        // TODO: Custom tests dir.
        let test_files = fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/tests"))?;
        let glob = glob::Pattern::new(self.cfg.test.as_ref().unwrap_or(&"*".to_string()).as_str())?;

        let enabled_fixtures = test_files
            .into_iter()
            .filter_map(|entry| {
                let entry = glob
                    .matches(entry.as_ref().ok()?.file_name().to_str()?)
                    .then_some(entry)?;

                let fixture_path = entry.ok()?.path();
                let fixture = toml::from_str::<TestFixture>(
                    &fs::read_to_string(fixture_path.join("fixture.toml")).ok()?,
                )
                .ok()?;
                Some((fixture_path, fixture))
            })
            .collect::<Vec<_>>();

        // Create the test case runners for enabled tests.
        let mut tests = Vec::new();
        for platform in self.matrix.iter() {
            for (program_kind, program_def) in platform.programs.iter() {
                for (fixture_path, fixture) in enabled_fixtures.iter() {
                    let inputs = ProgramHostInputs {
                        fixture_inputs: fixture.inputs.clone(),
                        rollup_cfg_path: fixture_path.join("rollup.json"),
                        genesis_path: fixture_path.join("genesis.json"),
                        source: ProgramHostSource::Disk {
                            path: fixture_path.join("witness-db"),
                        },
                    };

                    // TODO: Lift the arc's, terrible code I wrote at 2am.
                    tests.push(RunnableTest::new(
                        Arc::new(fixture.metadata.clone()),
                        Arc::new(inputs),
                        platform.vm_kind,
                        Arc::new(platform.clone()),
                        *program_kind,
                        Arc::new(program_def.clone()),
                    ));
                }
            }
        }

        Ok(tests)
    }

    /// Decompresses the fixtures within the test directory.
    async fn decompress_fixtures(&self) -> Result<()> {
        let tests = self.tests.as_ref().ok_or(eyre!("No tests to run"))?;
        let unique_fixtures = tests
            .iter()
            .unique_by(|t| t.inputs.genesis_path.as_path())
            .cloned()
            .collect::<Vec<_>>();

        let progress_bar = {
            let bar = ProgressBar::new(unique_fixtures.len() as u64);
            bar.enable_steady_tick(Duration::from_millis(50));
            bar.set_message("Decompressing active fixtures...");
            bar.set_style(ProgressStyle::default_bar().template("{msg} {wide_bar} {pos}/{len}")?);
            Arc::new(Mutex::new(bar))
        };

        let semaphore = Arc::new(Semaphore::new(self.cfg.workers));
        let mut join_set = JoinSet::new();

        for test in unique_fixtures.into_iter() {
            let semaphore = semaphore.clone();
            let progress_bar = progress_bar.clone();

            join_set.spawn(async move {
                // Aquire a permit on the semaphore. Once the permit is aquired, we can begin
                // decompressing the test fixture.
                let _permit = semaphore.acquire().await?;

                // Decompress the fixture.
                test.decompress_fixture().await?;

                // Notify the cli that the fixture has been decompressed.
                progress_bar.lock().await.inc(1);

                Ok::<_, color_eyre::Report>(())
            });
        }

        while let Some(result) = join_set.join_next().await {
            result??;
        }

        progress_bar
            .lock()
            .await
            .finish_with_message("Decompressed fixtures");
        Ok(())
    }
}
