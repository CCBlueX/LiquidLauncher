use std::collections::HashSet;

use anyhow::Result;
use os_info::Info;
use regex::Regex;

use crate::minecraft::version::{Rule, RuleAction};
use crate::utils::OS;

pub fn check_condition(rules: &Vec<Rule>, features: &HashSet<String>, os_info: &Info) -> Result<bool> {
    if rules.is_empty() {
        return Ok(true);
    }

    let mut allow = false;

    for rule in rules {
        let mut rule_applies = true;

        if let Some(os_requirement) = &rule.os {
            if os_requirement.name.as_ref().map_or(false, |x| x != OS.get_simple_name()) {
                rule_applies = false;
            }
            if let Some(arch) = &os_requirement.arch {
                if !arch.is(&os_info.bitness())? {
                    rule_applies = false;
                }
            }
            if let Some(version_regex) = &os_requirement.version {
                if !Regex::new(version_regex)?.is_match(&os_info.version().to_string()) {
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