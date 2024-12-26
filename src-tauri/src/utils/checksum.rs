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

use anyhow::Result;
use sha1::{Digest, Sha1};
use std::path::PathBuf;

pub fn sha1sum(path: &PathBuf) -> Result<String> {
    // get sha1 of library file and check if it matches
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash = hasher.finalize();
    let hex_hash = base16ct::lower::encode_string(&hash);

    Ok(hex_hash)
}
