use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
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
    pub fn get_url(
        &self, 
        jre_version: &u32, 
        os_name: &str, 
        os_arch: &str,
        archive_type: &str
    ) -> String {
        match self {
            JavaDistribution::Temurin => {
                format!(
                    "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
                    jre_version, os_name, os_arch
                )
            }
            JavaDistribution::GraalVM => {
                format!(
                    "https://download.oracle.com/graalvm/{}/latest/graalvm-jdk-{}_{}-{}_bin.{}",
                    jre_version, jre_version, os_name, os_arch, archive_type
                )
            }
            JavaDistribution::OpenJDK => {
                // use microsoft openjdk
                // https://aka.ms/download-jdk/microsoft-jdk-21-linux-x64.tar.gz

                format!(
                    "https://aka.ms/download-jdk/microsoft-jdk-{}_{}-{}.{}",
                    jre_version, os_name, os_arch, archive_type
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
