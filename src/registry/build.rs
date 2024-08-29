//! The builder for the [FPRegistry]'s contents.

use super::{BuildInstructions, COMPONENTS_DIR};
use crate::util::run_cmd;
use color_eyre::eyre::{ensure, eyre, Result};
use std::path::PathBuf;
use tokio::process::Command;
use tracing::info;

impl BuildInstructions {
    /// Returns a specific artifact by name.
    pub(crate) fn get_artifact(&self, name: &str) -> Result<PathBuf> {
        self.artifacts
            .get(name)
            .ok_or_else(|| eyre!("Artifact not found"))
            .and_then(|path| {
                let path = PathBuf::from(COMPONENTS_DIR)
                    .join(self.repo.clone())
                    .join(self.workdir.clone())
                    .join(path);
                Ok(path)
            })
    }

    /// Builds the binary artifact(s) from the cloned GitHub repository.
    pub(crate) async fn try_build(&self) -> Result<()> {
        // Clone the repository.
        self.clone_repo().await?;

        // Navigate to the work directory and build the binaries.
        let commands = self.cmd.split(" && ").collect::<Vec<_>>();
        for command_str in commands {
            let args = command_str.split_whitespace().collect::<Vec<_>>();
            let mut cmd = Command::new(args.get(0).ok_or(eyre!("Command is empty"))?);
            let build_output = run_cmd(
                cmd.args(args.get(1..).ok_or(eyre!("No arguments"))?.iter())
                    .current_dir(
                        PathBuf::from(COMPONENTS_DIR)
                            .join(self.repo.clone())
                            .join(self.workdir.clone()),
                    ),
            )
            .await?;

            // Check if the build was successful.
            ensure!(
                build_output.success(),
                "Failed to build repository: {repo}",
                repo = self.repo
            );
        }

        Ok(())
    }

    /// Clones the GitHub repository for the specified revision.
    async fn clone_repo(&self) -> Result<()> {
        if PathBuf::from(COMPONENTS_DIR)
            .join(self.repo.clone())
            .exists()
        {
            info!(
                target: "build",
                "Repository {repo} already exists, skipping clone.",
                repo = self.repo
            );
            return Ok(());
        }

        info!(target: "build", "Cloning repository: {}", self.repo);

        // Clone the repository.
        let mut cmd = Command::new("git");
        let clone_output = run_cmd(
            cmd.arg("clone")
                .arg("-b")
                .arg(self.rev.clone())
                .arg(format!("https://github.com/{}", self.repo))
                .arg(PathBuf::from(COMPONENTS_DIR).join(self.repo.clone())),
        )
        .await?;

        // Check if the clone was successful.
        ensure!(
            clone_output.success(),
            "Failed to clone repository: {repo}",
            repo = self.repo
        );

        info!(
            target: "build",
            "Cloned {repo} at {rev} successfully.",
            repo = self.repo,
            rev = self.rev
        );

        Ok(())
    }
}
