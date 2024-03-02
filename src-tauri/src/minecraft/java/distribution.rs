use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum JavaDistribution {
    #[serde(alias = "temurin")]
    Temurin,
    #[serde(alias = "graalvm")]
    GraalVM,
    #[serde(alias = "openjdk")]
    OpenJDK
}

impl Default for JavaDistribution {
    fn default() -> Self {
        // Termurin supports any version of java
        JavaDistribution::Temurin
    }
}

impl JavaDistribution {
    pub fn get_url(&self, jre_version: &str, os_name: &str, os_arch: &str) -> String {
        match self {
            JavaDistribution::Temurin => {
                format!(
                    "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
                    jre_version, os_name, os_arch
                )
            }
            JavaDistribution::GraalVM => {
                format!(
                    "https://download.oracle.com/graalvm/{}/latest/graalvm-jdk-{}_{}-{}_bin.tar.gz",
                    jre_version, jre_version, os_name, os_arch
                )
            }
            JavaDistribution::OpenJDK => {
                // use microsoft openjdk
                // https://aka.ms/download-jdk/microsoft-jdk-21-linux-x64.tar.gz

                format!(
                    "https://aka.ms/download-jdk/microsoft-jdk-{}_{}-{}.tar.gz",
                    jre_version, os_name, os_arch
                )
            }
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            JavaDistribution::Temurin => "temurin",
            JavaDistribution::GraalVM => "graalvm",
            JavaDistribution::OpenJDK => "openjdk"
        }
    }

}
