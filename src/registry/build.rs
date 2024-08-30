//! The builder for the [FPRegistry]'s contents.

use super::{BuildInstructions, COMPONENTS_DIR};
use color_eyre::eyre::{ensure, eyre, Result};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tokio::process::Command;
use tracing::debug;

impl BuildInstructions {
    /// Returns a specific artifact by name.
    pub(crate) fn get_artifact(&self, name: &str) -> Option<PathBuf> {
        self.artifacts.get(name).map(|path| {
            PathBuf::from(COMPONENTS_DIR)
                .join(self.repo.clone())
                .join(self.workdir.clone())
                .join(path)
        })
    }

    /// Builds the binary artifact(s) from the cloned GitHub repository.
    pub(crate) async fn try_build(&self) -> Result<()> {
        // Clone the repository.
        self.sync_repo().await?;

        // Navigate to the work directory and build the binaries.
        let commands = self.cmd.split(" && ").collect::<Vec<_>>();
        for command_str in commands {
            let args = command_str.split_whitespace().collect::<Vec<_>>();
            let build_output = Command::new(args.first().ok_or(eyre!("Command is empty"))?)
                .args(args.get(1..).ok_or(eyre!("No arguments"))?.iter())
                .current_dir(
                    PathBuf::from(COMPONENTS_DIR)
                        .join(self.repo.clone())
                        .join(self.workdir.clone()),
                )
                .output()
                .await?;

            if !build_output.status.success() {
                io::stdout().write_all(&build_output.stdout).unwrap();
                io::stderr().write_all(&build_output.stderr).unwrap();
            }

            // Check if the build was successful.
            ensure!(
                build_output.status.success(),
                "Failed to build repository: {repo}",
                repo = self.repo
            );
        }

        Ok(())
    }

    /// Clones the GitHub repository for the specified revision.
    async fn sync_repo(&self) -> Result<()> {
        if PathBuf::from(COMPONENTS_DIR)
            .join(self.repo.clone())
            .exists()
        {
            debug!(
                target: "build",
                "Repository {repo} already exists, skipping clone.",
                repo = self.repo
            );

            let fetch_output = Command::new("git")
                .arg("fetch")
                .arg("origin")
                .current_dir(PathBuf::from(COMPONENTS_DIR).join(self.repo.clone()))
                .output()
                .await?;
            ensure!(
                fetch_output.status.success(),
                "Failed to fetch upstream: {repo}",
                repo = self.repo
            );

            let checkout_output = Command::new("git")
                .arg("checkout")
                .arg(self.rev.clone())
                .current_dir(PathBuf::from(COMPONENTS_DIR).join(self.repo.clone()))
                .output()
                .await?;
            ensure!(
                checkout_output.status.success(),
                "Failed to checkout revision: {rev}",
                rev = self.rev
            );

            return Ok(());
        }

        debug!(target: "build", "Cloning repository: {}", self.repo);

        // Clone the repository.
        let clone_output = Command::new("git")
            .arg("clone")
            .arg("-b")
            .arg(self.rev.clone())
            .arg(format!("https://github.com/{}", self.repo))
            .arg(PathBuf::from(COMPONENTS_DIR).join(self.repo.clone()))
            .output()
            .await?;

        // Check if the clone was successful.
        ensure!(
            clone_output.status.success(),
            "Failed to clone repository: {repo}",
            repo = self.repo
        );

        debug!(
            target: "build",
            "Cloned {repo} at {rev} successfully.",
            repo = self.repo,
            rev = self.rev
        );

        Ok(())
    }
}
