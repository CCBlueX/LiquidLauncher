use std::collections::HashSet;

use anyhow::Result;
use regex::Regex;

use crate::minecraft::version::{Rule, RuleAction};
use crate::utils::{OS, ARCHITECTURE, OS_VERSION};

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