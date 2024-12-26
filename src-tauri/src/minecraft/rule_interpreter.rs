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

use std::collections::HashSet;

use anyhow::Result;
use regex::Regex;

use crate::minecraft::version::{Rule, RuleAction};
use crate::utils::{ARCHITECTURE, OS, OS_VERSION};

pub fn check_condition(rules: &Vec<Rule>, features: &HashSet<String>) -> Result<bool> {
    if rules.is_empty() {
        return Ok(true);
    }

    let os_name = OS.get_simple_name()?;
    let os_version = &*OS_VERSION.clone();

    let mut allow = false;

    for rule in rules {
        let mut rule_applies = true;

        if let Some(os_requirement) = &rule.os {
            if os_requirement.name.as_ref().map_or(false, |x| x != os_name) {
                rule_applies = false;
            }
            if let Some(arch) = &os_requirement.arch {
                if *arch != ARCHITECTURE {
                    rule_applies = false;
                }
            }
            if let Some(version_regex) = &os_requirement.version {
                if !Regex::new(version_regex)?.is_match(&os_version) {
                    rule_applies = false;
                }
            }
        }
        if let Some(filtered_features) = &rule.features {
            for (feature_name, feature_supported) in filtered_features {
                if features.contains(feature_name) != *feature_supported {
                    rule_applies = false;
                }
            }
        }

        if rule_applies {
            match rule.action {
                RuleAction::Allow => allow = true,
                RuleAction::Disallow => allow = false,
            }
        }
    }

    Ok(allow)
}
