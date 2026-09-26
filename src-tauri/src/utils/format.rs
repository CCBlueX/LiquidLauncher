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

use std::sync::LazyLock;

use chrono::{Datelike, NaiveDateTime, Utc};
use regex::Regex;

/// `Sep 26`, with the year when it is not this one.
pub fn short_date(date: NaiveDateTime) -> String {
    if date.year() == Utc::now().year() {
        date.format("%b %-d").to_string()
    } else {
        date.format("%b %-d, %Y").to_string()
    }
}

/// The error and its causes as one line, without the request URL.
pub fn error_line(error: &anyhow::Error) -> String {
    static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" for url \([^)]*\)").unwrap());
    URL.replace_all(&format!("{error:#}"), "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_the_url_out_of_errors() {
        let error = anyhow::anyhow!(
            "error sending request for url (http://127.0.0.1:1/api/v3/marketplace?page=1)"
        )
        .context("unable to browse marketplace");
        assert_eq!(
            error_line(&error),
            "unable to browse marketplace: error sending request"
        );
    }
}
