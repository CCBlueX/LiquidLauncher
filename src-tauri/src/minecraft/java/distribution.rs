use anyhow::{bail, Result};
use crate::utils::{ARCHITECTURE, OS};
use crate::HTTP_CLIENT;
use reqwest::StatusCode;
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

impl JavaDistribution {
    /// `None` when the vendor has no build for the running OS and architecture
    pub async fn get_url(&self, jre_version: &u32) -> Result<Option<String>> {
        let os_arch = ARCHITECTURE.get_simple_name()?;
        let archive_type = OS.get_archive_type()?;

        let url = match self {
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
            // Azul's file names carry the exact build, so only its metadata API knows the URL
            JavaDistribution::Zulu => {
                #[derive(Deserialize)]
                struct AzulPackage {
                    download_url: String,
                }

                let packages: Vec<AzulPackage> = HTTP_CLIENT
                    .get(format!(
                        "https://api.azul.com/metadata/v1/zulu/packages/?java_version={}&os={}&arch={}&archive_type={}&java_package_type=jre&availability_types=CA&release_status=ga&javafx_bundled=false&latest=true&page_size=1",
                        jre_version,
                        OS.get_zulu_name()?,
                        ARCHITECTURE.get_zulu_name()?,
                        archive_type
                    ))
                    .header("accept", "application/json")
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;

                return Ok(packages
                    .into_iter()
                    .next()
                    .map(|package| package.download_url));
            }
        };

        let response = HTTP_CLIENT.head(&url).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        response.error_for_status()?;
        Ok(Some(url))
    }

    pub async fn has_build(&self, jre_version: &u32) -> Result<bool> {
        Ok(self.get_url(jre_version).await?.is_some())
    }

    pub fn get_name(&self) -> &str {
        match self {
            JavaDistribution::Temurin => "temurin",
            JavaDistribution::GraalVM => "graalvm",
            JavaDistribution::Zulu => "zulu",
        }
    }

    pub fn supports_version(&self, version: u32) -> bool {
        match self {
            JavaDistribution::Temurin => true, // Supports 8, 11, 17, 21
            JavaDistribution::GraalVM => version >= 17, // Only supports 17+
            JavaDistribution::Zulu => true,             // Community builds available for all LTS versions
        }
    }
}
