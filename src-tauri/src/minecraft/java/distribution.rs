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

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub enum JavaDistribution {
    #[serde(rename = "temurin")]
    Temurin,
    #[serde(rename = "graalvm")]
    GraalVM,
    #[serde(rename = "zulu")]
    Zulu,
}

impl Default for JavaDistribution {
    fn default() -> Self {
        // Temurin supports any version of java
        JavaDistribution::Temurin
    }
}

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
    pub fn list<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<Self>, D::Error> {
        let Value::Array(packages) = Value::deserialize(input)? else {
            return Ok(Vec::new());
        };
        Ok(packages
            .into_iter()
            .filter_map(|item| from_value(item).ok())
            .collect())
    }

    pub fn fits(&self, major: u32) -> bool {
        self.major == major
            && self.version.major() == major
            && OS.get_simple_name().ok() == Some(self.os.as_str())
            && ARCHITECTURE.get_simple_name().ok() == Some(self.arch.as_str())
            && OS.get_archive_type().ok() == Some(self.archive.as_str())
            && self.url.starts_with("https://")
            && self.sha256.len() == 64
            && self.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

impl JavaDistribution {
    pub fn get_name(&self) -> &str {
        match self {
            JavaDistribution::Temurin => "temurin",
            JavaDistribution::GraalVM => "graalvm",
            JavaDistribution::Zulu => "zulu",
        }
    }
}
