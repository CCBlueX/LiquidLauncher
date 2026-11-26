/*
 * This file is part of LiquidLauncher (https://github.com/CCBlueX/LiquidLauncher)
 *
 * Copyright (c) 2015 - 2026 CCBlueX
 *
 * LiquidLauncher is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * LiquidLauncher is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with LiquidLauncher. If not, see <https://www.gnu.org/licenses/>.
 */

//! LiquidBounce saves a Microsoft sign-in the way the Java MinecraftAuth
//! library does, the launcher the way the Rust `minecraft-auth` crate does.
//!
//! Xbox Live and Minecraft tokens are left out, both sides fetch them again
//! from the MSA token.

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use minecraft_auth::java::MinecraftProfile;
use minecraft_auth::msa::{MsaApplicationConfig, MsaEnvironment, MsaToken};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey};
use p256::SecretKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct RustState {
    application_config: MsaApplicationConfig,
    device_type: String,
    device_key_pair: RustKeyPair,
    device_id: Uuid,
    msa_token: MsaToken,
    minecraft_profile: Option<MinecraftProfile>,
}

#[derive(Serialize, Deserialize)]
struct RustKeyPair {
    /// Base64 PKCS#8 DER
    private_key_pkcs8: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaState {
    #[serde(rename = "_saveVersion", default = "save_version")]
    save_version: u32,
    msa_application_config: JavaConfig,
    device_type: String,
    device_key_pair: JavaKeyPair,
    device_id: Uuid,
    msa_token: JavaToken,
    #[serde(skip_serializing_if = "Option::is_none")]
    minecraft_profile: Option<JavaProfile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaConfig {
    #[serde(rename = "_saveVersion", default = "save_version")]
    save_version: u32,
    client_id: String,
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect_uri: Option<String>,
    #[serde(default, with = "JavaEnvironment")]
    environment: MsaEnvironment,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "MsaEnvironment", rename_all = "SCREAMING_SNAKE_CASE")]
enum JavaEnvironment {
    Live,
    MicrosoftOnlineCommon,
    MicrosoftOnlineConsumers,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaKeyPair {
    algorithm: String,
    /// Base64 X.509 DER
    public_key: String,
    /// Base64 PKCS#8 DER
    private_key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaToken {
    #[serde(rename = "_saveVersion", default = "save_version")]
    save_version: u32,
    expire_time_ms: i64,
    access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaProfile {
    #[serde(rename = "_saveVersion", default = "save_version")]
    save_version: u32,
    id: Uuid,
    name: String,
}

fn save_version() -> u32 {
    1
}

/// The launcher's `state` as LiquidBounce's `authManager`.
pub fn to_java(state: &Value) -> Result<Value> {
    let rust = RustState::deserialize(state).context("Unexpected Microsoft account state")?;
    let config = rust.application_config;
    let token = rust.msa_token;

    let java = JavaState {
        save_version: save_version(),
        msa_application_config: JavaConfig {
            save_version: save_version(),
            client_id: config.client_id,
            scope: config.scope,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            environment: config.environment,
        },
        device_type: rust.device_type,
        device_key_pair: java_key_pair(&rust.device_key_pair)?,
        device_id: rust.device_id,
        msa_token: JavaToken {
            save_version: save_version(),
            expire_time_ms: token.expire_time_ms,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
        },
        minecraft_profile: rust.minecraft_profile.map(|profile| JavaProfile {
            save_version: save_version(),
            id: profile.id,
            name: profile.name,
        }),
    };
    Ok(serde_json::to_value(java)?)
}

/// LiquidBounce's `authManager` as the launcher's `state`.
pub fn to_rust(auth_manager: &Value) -> Result<Value> {
    let java =
        JavaState::deserialize(auth_manager).context("Unexpected LiquidBounce authManager")?;
    let config = java.msa_application_config;
    let token = java.msa_token;

    let rust = RustState {
        application_config: MsaApplicationConfig {
            client_id: config.client_id,
            scope: config.scope,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            environment: config.environment,
        },
        device_type: java.device_type,
        device_key_pair: rust_key_pair(&java.device_key_pair)?,
        device_id: java.device_id,
        msa_token: MsaToken {
            expire_time_ms: token.expire_time_ms,
            access_token: token.access_token,
            refresh_token: token.refresh_token,
        },
        minecraft_profile: java.minecraft_profile.map(|profile| MinecraftProfile {
            id: profile.id,
            name: profile.name,
        }),
    };
    Ok(serde_json::to_value(rust)?)
}

fn java_key_pair(key_pair: &RustKeyPair) -> Result<JavaKeyPair> {
    let secret = secret_key(&key_pair.private_key_pkcs8)?;
    let public_key = secret
        .public_key()
        .to_public_key_der()
        .context("Failed to encode the device public key")?;

    Ok(JavaKeyPair {
        algorithm: "EC".to_string(),
        public_key: STANDARD.encode(public_key.as_bytes()),
        private_key: pkcs8_base64(&secret)?,
    })
}

fn rust_key_pair(key_pair: &JavaKeyPair) -> Result<RustKeyPair> {
    if key_pair.algorithm != "EC" {
        bail!("Unsupported device key algorithm {}", key_pair.algorithm);
    }

    let secret = secret_key(&key_pair.private_key)?;
    Ok(RustKeyPair {
        private_key_pkcs8: pkcs8_base64(&secret)?,
    })
}

fn secret_key(encoded: &str) -> Result<SecretKey> {
    let der = STANDARD
        .decode(encoded)
        .context("Device private key is not base64")?;
    SecretKey::from_pkcs8_der(&der).context("Device private key is not a P-256 PKCS#8 key")
}

fn pkcs8_base64(secret: &SecretKey) -> Result<String> {
    let der = secret
        .to_pkcs8_der()
        .context("Failed to encode the device private key")?;
    Ok(STANDARD.encode(der.as_bytes()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use minecraft_auth::java::JavaAuthManager;
    use serde_json::json;

    /// `Client::new()` wants system CA certificates, a build sandbox has none.
    pub(crate) fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .tls_certs_only([])
            .build()
            .unwrap()
    }

    /// As Java MinecraftAuth 5 writes it, with a key pair from Java's
    /// `KeyPairGenerator.getInstance("EC")`.
    pub(crate) fn java_sample() -> Value {
        json!({
            "_saveVersion": 1,
            "msaApplicationConfig": {
                "_saveVersion": 1,
                "clientId": "00000000402b5328",
                "scope": "service::user.auth.xboxlive.com::MBI_SSL",
                "environment": "LIVE"
            },
            "deviceType": "Win32",
            "deviceKeyPair": {
                "algorithm": "EC",
                "publicKey": JAVA_PUBLIC_KEY,
                "privateKey": JAVA_PRIVATE_KEY
            },
            "deviceId": "8b0f7a64-3f3b-4c47-9a0f-6a2f2f7f4e11",
            "msaToken": {
                "_saveVersion": 1,
                "expireTimeMs": 1790000000000i64,
                "accessToken": "msa-access",
                "refreshToken": "M.refresh"
            },
            "xblDeviceToken": { "_saveVersion": 1, "token": "dropped" },
            "minecraftToken": { "_saveVersion": 1, "token": "dropped" },
            "minecraftProfile": {
                "_saveVersion": 1,
                "id": "069a79f4-44e9-4726-a5be-fca90e38aaf5",
                "name": "Notch"
            }
        })
    }

    const JAVA_PUBLIC_KEY: &str = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE8Yo/uTGlbCJbYwgOf90fjjdiupPpawwdRIRscwfZyXoBjAI7LEAwUiZduU52p9lPlN6voAVXfulSlrCt79yZFQ==";
    const JAVA_PRIVATE_KEY: &str = "MEECAQAwEwYHKoZIzj0CAQYIKoZIzj0DAQcEJzAlAgEBBCDAIv2K7fibjb+nA3yvUbmXe/t/pYfPVqZaPBfF+Tf71w==";

    #[test]
    fn reads_java_auth_manager() {
        let rust = to_rust(&java_sample()).unwrap();

        assert_eq!(rust["application_config"]["client_id"], "00000000402b5328");
        assert_eq!(rust["application_config"]["environment"], "Live");
        assert_eq!(rust["device_type"], "Win32");
        assert_eq!(rust["device_id"], "8b0f7a64-3f3b-4c47-9a0f-6a2f2f7f4e11");
        assert_eq!(rust["msa_token"]["refresh_token"], "M.refresh");
        assert_eq!(rust["minecraft_profile"]["name"], "Notch");
        assert!(rust.get("minecraft_token").is_none());

        JavaAuthManager::from_json(client(), &rust).unwrap();
    }

    #[test]
    fn round_trip_keeps_everything_carried_over() {
        let mut expected = java_sample();
        let mut back = to_java(&to_rust(&expected).unwrap()).unwrap();

        // p256 puts the public key into the PKCS#8 bytes too, Java does not,
        // so the private keys are compared as keys and not as text.
        let take_private_key = |java: &mut Value| {
            let encoded = java["deviceKeyPair"]["privateKey"].take();
            secret_key(encoded.as_str().unwrap()).unwrap()
        };
        assert_eq!(take_private_key(&mut back), take_private_key(&mut expected));

        let expected_fields = expected.as_object_mut().unwrap();
        expected_fields.remove("xblDeviceToken");
        expected_fields.remove("minecraftToken");
        assert_eq!(back, expected);
    }

    #[tokio::test]
    async fn rust_state_survives_java_and_back() {
        let manager = JavaAuthManager::builder(client()).login_msa_token(MsaToken {
            expire_time_ms: 1790000000000,
            access_token: "msa-access".to_string(),
            refresh_token: None,
        });
        let rust = manager.to_json().await.unwrap();

        let java = to_java(&rust).unwrap();
        assert_eq!(java["deviceKeyPair"]["algorithm"], "EC");
        assert!(java["msaToken"].get("refreshToken").is_none());
        assert!(java["msaApplicationConfig"].get("clientSecret").is_none());

        let back = to_rust(&java).unwrap();
        for key in [
            "application_config",
            "device_type",
            "device_key_pair",
            "device_id",
            "msa_token",
        ] {
            assert_eq!(back[key], rust[key], "{key}");
        }
    }

    #[test]
    fn environments_use_java_enum_names() {
        let mut java = java_sample();
        for (java_name, rust_name) in [
            ("MICROSOFT_ONLINE_COMMON", "MicrosoftOnlineCommon"),
            ("MICROSOFT_ONLINE_CONSUMERS", "MicrosoftOnlineConsumers"),
        ] {
            java["msaApplicationConfig"]["environment"] = json!(java_name);
            let rust = to_rust(&java).unwrap();
            assert_eq!(rust["application_config"]["environment"], rust_name);
            let back = to_java(&rust).unwrap();
            assert_eq!(back["msaApplicationConfig"]["environment"], java_name);
        }
    }
}
