/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2026 CCBlueX
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

use std::path::PathBuf;

use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum InstanceState {
    #[default]
    Running,
    Blocked,
    Starting,
    Started,
    Attaching,
    Attached,
    Detaching,
}

impl InstanceState {
    pub fn is_busy(self) -> bool {
        matches!(self, Self::Starting | Self::Attaching)
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: Option<u32>,
    pub pid: Option<u32>,
    pub version: Option<String>,
    pub lb_version: Option<String>,
    pub directory: Option<PathBuf>,
    pub profile: Option<String>,
    pub username: Option<String>,
    pub state: InstanceState,
    pub unsupported: Option<String>,
}
