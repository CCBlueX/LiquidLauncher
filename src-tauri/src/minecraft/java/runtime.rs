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
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::oneshot::Receiver;
use tracing::debug;
pub struct JavaRuntime(PathBuf);

impl JavaRuntime {
    pub fn new(path: PathBuf) -> JavaRuntime {
        JavaRuntime(path)
    }

    pub async fn execute(&self, arguments: Vec<String>, game_dir: &Path) -> Result<Child> {
        if !self.0.exists() {
            bail!("Java runtime not found at: {}", self.0.display());
        }

        debug!("Executing Java runtime: {}", self.0.display());

        let mut command = Command::new(&self.0);
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
        let mut stdout = running_task.stdout.take().unwrap();
        let mut stderr = running_task.stderr.take().unwrap();

        let mut stdout_buf = vec![0; 1024];
        let mut stderr_buf = vec![0; 1024];

        tokio::pin!(terminator);

        loop {
            tokio::select! {
                read_len = stdout.read(&mut stdout_buf) => {
                    let _ = on_stdout(&data, &stdout_buf[..read_len?]);
                },
                read_len = stderr.read(&mut stderr_buf) => {
                    let _ = on_stderr(&data, &stderr_buf[..read_len?]);
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
