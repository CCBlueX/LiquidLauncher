use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

use super::JavaVersion;
use crate::utils::{Architecture, ARCHITECTURE, OS};
use crate::HTTP_CLIENT;

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

pub struct JavaPackage {
    pub version: JavaVersion,
    pub url: String,
    pub sha256: String,
}

impl JavaDistribution {
    pub async fn package(&self, major: &u32) -> Result<Option<JavaPackage>> {
        let package = match self {
            Self::Temurin => temurin(*major).await?,
            Self::Zulu => zulu(*major).await?,
            Self::GraalVM => graal(*major).await?,
        };
        if let Some(build) = &package {
            ensure!(
                build.version.major() == *major,
                "Vendor returned the wrong Java major"
            );
        }
        Ok(package)
    }

    pub async fn has_build(&self, jre_version: &u32) -> Result<bool> {
        Ok(self.package(jre_version).await?.is_some())
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
            JavaDistribution::Zulu => true,    // Community builds available for all LTS versions
        }
    }
}

async fn temurin(major: u32) -> Result<Option<JavaPackage>> {
    #[derive(Deserialize)]
    struct Asset {
        binary: Binary,
        version: Version,
    }
    #[derive(Deserialize)]
    struct Binary {
        package: Package,
    }
    #[derive(Deserialize)]
    struct Package {
        link: String,
        checksum: String,
    }
    #[derive(Deserialize)]
    struct Version {
        openjdk_version: String,
    }

    let assets: Vec<Asset> = HTTP_CLIENT
        .get(format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={}&os={}&image_type=jre&vendor=eclipse",
            ARCHITECTURE.get_simple_name()?,
            OS.get_adoptium_name()?,
        ))
        .send().await?.error_for_status()?.json().await?;
    assets
        .into_iter()
        .next()
        .map(|asset| {
            Ok(JavaPackage {
                version: JavaVersion::parse(&asset.version.openjdk_version)
                    .context("Invalid Temurin version")?,
                url: asset.binary.package.link,
                sha256: asset.binary.package.checksum,
            })
        })
        .transpose()
}

async fn zulu(major: u32) -> Result<Option<JavaPackage>> {
    #[derive(Deserialize)]
    struct Package {
        java_version: Vec<u32>,
        openjdk_build_number: Option<u32>,
        download_url: String,
        sha256_hash: String,
    }

    let packages: Vec<Package> = HTTP_CLIENT
        .get(format!(
            "https://api.azul.com/metadata/v1/zulu/packages/?java_version={major}&os={}&arch={}&archive_type={}&java_package_type=jre&availability_types=CA&release_status=ga&javafx_bundled=false&latest=true&page_size=1&include_fields=sha256_hash,openjdk_build_number",
            OS.get_zulu_name()?,
            ARCHITECTURE.get_zulu_name()?,
            OS.get_archive_type()?,
        ))
        .send().await?.error_for_status()?.json().await?;
    packages
        .into_iter()
        .next()
        .map(|package| {
            Ok(JavaPackage {
                version: JavaVersion::new(package.java_version, package.openjdk_build_number)
                    .context("Invalid Zulu version")?,
                url: package.download_url,
                sha256: package.sha256_hash,
            })
        })
        .transpose()
}

async fn graal(major: u32) -> Result<Option<JavaPackage>> {
    #[derive(Deserialize)]
    struct Artifacts {
        items: Vec<Artifact>,
    }
    #[derive(Deserialize)]
    struct Artifact {
        id: String,
        checksum: String,
        metadata: Vec<Metadata>,
    }
    #[derive(Deserialize)]
    struct Metadata {
        key: String,
        value: String,
    }

    let arch = match *ARCHITECTURE {
        Architecture::X64 => "amd64",
        Architecture::AARCH64 => "aarch64",
        _ => return Ok(None),
    };
    let os = OS.get_graal_name()?;
    let base = "https://gds.oracle.com/api/20220101/artifacts";
    // Select only GFTC releases: later 17 and 21 patches require a separate Oracle license.
    let catalog: Artifacts = HTTP_CLIENT
        .get(format!(
            "{base}?productId=D53FAE8052773FFAE0530F15000AA6C6&licenseId=FD40B563DC17366EE0531518000ADC4D&metadata=isBase:True&metadata=edition:ee&metadata=os:{os}&metadata=arch:{arch}&status=PUBLISHED&displayName=Oracle%20GraalVM%20for%20JDK%20{major}%20release&sortBy=timeCreated&sortOrder=DESC&limit=1&responseFields=id&responseFields=checksum&responseFields=metadata"
        ))
        .send().await?.error_for_status()?.json().await?;
    catalog
        .items
        .into_iter()
        .next()
        .map(|artifact| {
            let version = artifact
                .metadata
                .iter()
                .find(|field| field.key == "version")
                .and_then(|field| JavaVersion::parse(&field.value))
                .context("Missing GraalVM release version")?;
            Ok(JavaPackage {
                version,
                url: format!("{base}/{}/content", artifact.id),
                sha256: artifact.checksum,
            })
        })
        .transpose()
}
