use serde::{de::Error, Deserialize, Deserializer, Serialize};
use serde_json::{from_value, Value};

use super::JavaVersion;
use crate::utils::{ARCHITECTURE, OS};

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "value")]
pub enum DistributionSelection {
    #[serde(rename = "automatic")]
    Automatic(String), // (String) is useless, but required for deserialization
    #[serde(rename = "custom")]
    Custom(String),
    #[serde(rename = "manual")]
    Manual(JavaDistribution),
}

impl Default for DistributionSelection {
    fn default() -> Self {
        DistributionSelection::Automatic(String::new())
    }
}

/// The identifier of a Java vendor as the API names it, such as `temurin`. It is part of folder
/// names, hence the narrow set of characters.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String")]
pub struct JavaDistribution(String);

impl JavaDistribution {
    pub fn get_name(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for JavaDistribution {
    type Error = &'static str;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        let is_allowed = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-';
        if name.is_empty() || name.len() > 64 || name.starts_with('-') || !name.bytes().all(is_allowed) {
            return Err("Invalid Java distribution ID");
        }
        Ok(Self(name))
    }
}

/// A Java archive the API approved for one vendor, Java version and platform.
#[derive(Clone, Deserialize)]
pub struct JavaPackage {
    pub vendor: JavaDistribution,
    pub os: String,
    pub arch: String,
    pub archive: String,
    pub major: u32,
    #[serde(deserialize_with = "parse_version")]
    pub version: JavaVersion,
    pub url: String,
    pub sha256: String,
}

fn parse_version<'de, D: Deserializer<'de>>(input: D) -> Result<JavaVersion, D::Error> {
    JavaVersion::parse(&String::deserialize(input)?)
        .ok_or_else(|| D::Error::custom("Invalid Java version"))
}

impl JavaPackage {
    /// Reads the packages of a launch manifest, leaving out any this launcher cannot read.
    pub fn list<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<Self>, D::Error> {
        let Value::Array(packages) = Value::deserialize(input)? else {
            return Ok(Vec::new());
        };
        Ok(packages
            .into_iter()
            .filter_map(|package| from_value(package).ok())
            .collect())
    }

    /// Whether this machine can install the package as Java `major`.
    pub fn fits(&self, major: u32) -> bool {
        let is_for_this_platform = OS.get_simple_name().ok() == Some(self.os.as_str())
            && ARCHITECTURE.get_simple_name().ok() == Some(self.arch.as_str())
            && OS.get_archive_type().ok() == Some(self.archive.as_str());
        let is_download = self.url.starts_with("http://") || self.url.starts_with("https://");
        // The checksum names the folder the package is installed to.
        let is_checksum =
            self.sha256.len() == 64 && self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit());

        self.major == major
            && self.version.major() == major
            && is_for_this_platform
            && is_download
            && is_checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn manual_distribution_must_be_a_safe_cache_name() {
        let selection = |vendor: &str| json!({ "type": "manual", "value": vendor });
        for name in ["", "../java", "/java", "a\\b", "-jdk", "a_b", "Jdk", "café", &"a".repeat(65)] {
            assert!(from_value::<DistributionSelection>(selection(name)).is_err(), "{name}");
        }
        for name in ["temurin", "graalvm", "zulu", "vendor-42", "1--", &"a".repeat(64)] {
            let input = selection(name);
            let parsed: DistributionSelection = from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), input);
        }
    }
}
