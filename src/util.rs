//! Utilities for the `fpt` binary.

use color_eyre::Result;
use std::{io, process::ExitStatus};
use tokio::{process::Command, try_join};

/// Runs a command in a child process and streams the output to stdout.
///
/// ## Takes
/// - `cmd` - The command to run.
///
/// ## Returns
/// - `Result<ExitStatus>` - Ok if successful, Err otherwise.
pub(crate) async fn run_cmd(mut cmd: &mut Command) -> Result<ExitStatus> {
    cmd = cmd.stdout(io::stdout()).stderr(io::stderr());

    let mut child = cmd.spawn()?;
    let proc_handle = tokio::spawn(async move { child.wait().await });

    let (proc_res,) = try_join!(proc_handle)?;
    proc_res.map_err(Into::into)
}
