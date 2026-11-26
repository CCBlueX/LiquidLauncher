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

use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "FATAL" => Self::Fatal,
            "ERROR" => Self::Error,
            "WARN" => Self::Warn,
            "INFO" => Self::Info,
            "DEBUG" => Self::Debug,
            "TRACE" => Self::Trace,
            _ => return None,
        })
    }
}

/// A line of the client log. `text` is the line as it was printed.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LogLine {
    /// The first line of a log4j event.
    Event {
        text: String,
        time: String,
        thread: String,
        level: Level,
        logger: Option<String>,
        message: String,
    },
    Launcher {
        text: String,
        level: Level,
    },
    /// Any other output, mostly the lines following an event, such as a stack trace.
    Text {
        text: String,
        is_frame: bool,
    },
}

impl LogLine {
    pub fn parse(text: &str) -> Self {
        // Vanilla `[01:09:54] [Render thread/ERROR]: msg`, Forge `... [Client thread/INFO] [FML]: msg`
        // and Fabric `... [main/INFO] (FabricLoader) msg`.
        static EVENT: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\[(\d{2}:\d{2}:\d{2})\] \[([^\]]+)/([A-Z]+)\](?:: | \[([^\]]+)\]: | \(([^)]+)\) )(.*)$")
                .unwrap()
        });

        EVENT
            .captures(text)
            .and_then(|caps| {
                Some(Self::Event {
                    text: text.to_owned(),
                    time: caps[1].to_owned(),
                    thread: caps[2].to_owned(),
                    level: Level::named(&caps[3])?,
                    logger: caps.get(4).or(caps.get(5)).map(|logger| logger.as_str().to_owned()),
                    message: caps[6].to_owned(),
                })
            })
            .unwrap_or_else(|| {
                let trimmed = text.trim_start();
                Self::Text {
                    text: text.to_owned(),
                    is_frame: trimmed.len() < text.len()
                        && (trimmed.starts_with("at ") || trimmed.starts_with("... ")),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(text: &str) -> (String, Level, Option<String>, String) {
        match LogLine::parse(text) {
            LogLine::Event { thread, level, logger, message, .. } => (thread, level, logger, message),
            line => panic!("not an event: {line:?}"),
        }
    }

    #[test]
    fn reads_events() {
        assert_eq!(
            event("[01:09:53] [Render thread/INFO]: [Indigo] Different rendering plugin detected"),
            ("Render thread".into(), Level::Info, None, "[Indigo] Different rendering plugin detected".into())
        );
        assert_eq!(
            event("[12:00:00] [Client thread/WARN] [FML]: Mod list changed"),
            ("Client thread".into(), Level::Warn, Some("FML".into()), "Mod list changed".into())
        );
        assert_eq!(
            event("[12:00:00] [main/ERROR] (FabricLoader/Mixin) Mixin apply failed"),
            ("main".into(), Level::Error, Some("FabricLoader/Mixin".into()), "Mixin apply failed".into())
        );
    }

    #[test]
    fn reads_other_lines_as_text() {
        let is_frame = |text| match LogLine::parse(text) {
            LogLine::Text { is_frame, .. } => is_frame,
            line => panic!("not text: {line:?}"),
        };
        assert!(is_frame("\tat knot//net.minecraft.client.Minecraft.<init>(Minecraft.java:740)"));
        assert!(is_frame("\t\t... 11 more"));
        assert!(!is_frame("Caused by: java.lang.UnsatisfiedLinkError: Unable to load library 'flite':"));
        assert!(!is_frame("at the end of a sentence"));
        assert!(!is_frame("[12:00:00] [main/NOTICE]: not a log4j level"));
    }
}
