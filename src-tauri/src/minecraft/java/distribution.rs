use serde::{Deserialize, Serialize};
use crate::utils::{ARCHITECTURE, OS};

#[derive(Deserialize, Serialize, Clone)]
pub enum JavaDistribution {
    #[serde(rename = "temurin")]
    Temurin,
    #[serde(rename = "graalvm")]
    GraalVM,
    #[serde(rename = "openjdk")]
    OpenJDK
}

impl Default for JavaDistribution {
    fn default() -> Self {
        // Termurin supports any version of java
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

                // https://download.oracle.com/graalvm/21/latest/graalvm-jdk-21_windows-x64_bin.zip
                // https://download.oracle.com/graalvm/21/latest/graalvm-jdk-21_linux-x64_bin.tar.gz
                // https://download.oracle.com/graalvm/21/latest/graalvm-jdk-21_macos-x64_bin.tar.gz
                format!(
                    "https://download.oracle.com/graalvm/{}/latest/graalvm-jdk-{}_{}-{}_bin.{}",
                    jre_version, jre_version, os_name, os_arch, archive_type
                )
            }
            JavaDistribution::OpenJDK => {
                // use microsoft openjdk
                // https://aka.ms/download-jdk/microsoft-jdk-21-linux-x64.tar.gz
                let os_name = OS.get_graal_name()?;

                format!(
                    "https://aka.ms/download-jdk/microsoft-jdk-{}_{}-{}.{}",
                    jre_version, os_name, os_arch, archive_type
                )
            }
        })
    }

    pub fn get_name(&self) -> &str {
        match self {
            JavaDistribution::Temurin => "temurin",
            JavaDistribution::GraalVM => "graalvm",
            JavaDistribution::OpenJDK => "openjdk"
        }
    }

}
