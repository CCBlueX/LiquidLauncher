/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2024 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::{bail, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::oneshot::Receiver;
use tokio::{pin, select};
use tracing::debug;

pub struct JavaRuntime {
    path: PathBuf,
    /// Kept open by the game, see `lease` in `jre_downloader`.
    lease: Option<File>,
}

impl JavaRuntime {
    pub fn new(path: PathBuf) -> JavaRuntime {
        Self { path, lease: None }
    }

    pub(super) fn leased(path: PathBuf, lease: File) -> Self {
        Self {
            path,
            lease: Some(lease),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn execute(&self, arguments: Vec<String>, game_dir: &Path) -> Result<Child> {
        if !fs::try_exists(&self.path).await? {
            bail!("Java runtime not found at: {}", self.path.display());
        }

        debug!("Executing Java runtime: {}", self.path.display());

        let mut command = Command::new(&self.path);
        if let Some(lease) = &self.lease {
            // As its standard input the game inherits the lease, and holds it after the launcher quits.
            command.stdin(Stdio::from(lease.try_clone()?));
        }
        command.current_dir(game_dir);
        command.args(arguments);

        command.stderr(Stdio::piped()).stdout(Stdio::piped());

        let child = command.spawn()?;
        Ok(child)
    }

    pub async fn handle_io<D: Send + Sync>(
        &self,
        running_task: &mut Child,
        on_stdout: fn(&D, &[u8]) -> Result<()>,
        on_stderr: fn(&D, &[u8]) -> Result<()>,
        terminator: Receiver<()>,
        data: &D,
    ) -> Result<()> {
        let mut stdout = BufReader::new(running_task.stdout.take().unwrap());
        let mut stderr = BufReader::new(running_task.stderr.take().unwrap());

        // Kept across iterations: `read_until` leaves a line read in part in them when cancelled.
        let mut stdout_line = Vec::new();
        let mut stderr_line = Vec::new();

        pin!(terminator);

        loop {
            select! {
                read_len = stdout.read_until(b'\n', &mut stdout_line) => {
                    if read_len? > 0 {
                        let _ = on_stdout(&data, &stdout_line);
                    }
                    stdout_line.clear();
                },
                read_len = stderr.read_until(b'\n', &mut stderr_line) => {
                    if read_len? > 0 {
                        let _ = on_stderr(&data, &stderr_line);
                    }
                    stderr_line.clear();
                },
                _ = &mut terminator => {
                    running_task.kill().await?;
                    break;
                },
                exit_status = running_task.wait() => {
                    let code = exit_status?.code().unwrap_or(7900); // 7900 = unwrap failed error code

                    debug!("Process exited with code: {}", code);
                    if code != 0 && code != -1073740791 { // -1073740791 = happens when the process is killed forcefully, we don't want to bail in this case
                        bail!("Process exited with non-zero exit code: {}.", code);
                    }
                    break;
                },
            }
        }
        Ok(())
    }
}
