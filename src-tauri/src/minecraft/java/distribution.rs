use anyhow::bail;
use crate::utils::{ARCHITECTURE, OS};
use serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize, Clone)]
pub enum JavaDistribution {
    #[serde(rename = "temurin")]
    Temurin,
    #[serde(rename = "graalvm")]
    GraalVM,
}

impl Default for JavaDistribution {
    fn default() -> Self {
        // Temurin supports any version of java
        JavaDistribution::Temurin
    }
}

impl JavaDistribution {
    pub fn get_url(&self, jre_version: &u32) -> anyhow::Result<String> {
        let os_arch = ARCHITECTURE.get_simple_name()?;
        let archive_type = OS.get_archive_type()?;

        Ok(match self {
            JavaDistribution::Temurin => {
                let os_name = OS.get_adoptium_name()?;
                format!(
                    "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
                    jre_version, os_name, os_arch
                )
            }
            JavaDistribution::GraalVM => {
                let os_name = OS.get_graal_name()?;

                if jre_version > &17 {
                    format!(
                        "https://download.oracle.com/graalvm/{}/latest/graalvm-jdk-{}_{}-{}_bin.{}",
                        jre_version, jre_version, os_name, os_arch, archive_type
                    )
                } else if jre_version == &17 {
                    // Use archive link for 17.0.12
                    format!(
                        "https://download.oracle.com/graalvm/17/archive/graalvm-jdk-17.0.12_{}-{}_bin.{}",
                        os_name, os_arch, archive_type
                    )
                } else {
                    bail!("GraalVM only supports Java 17+")
                }
            }
        })
    }

    pub fn get_name(&self) -> &str {
        match self {
            JavaDistribution::Temurin => "temurin",
            JavaDistribution::GraalVM => "graalvm",
        }
    }

    pub fn supports_version(&self, version: u32) -> bool {
        match self {
            JavaDistribution::Temurin => true, // Supports 8, 11, 17, 21
            JavaDistribution::GraalVM => version >= 17, // Only supports 17+
        }
    }
}
