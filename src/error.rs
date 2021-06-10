use thiserror::Error;

#[derive(Error, Debug)]
pub enum LauncherError {
    #[error("Invalid version profile: {0}")]
    InvalidVersionProfile(String),
    #[error("Invalid launcher manifest: {0}")]
    InvalidLauncherManifest(String),
    #[error("Unknown template parameter: {0}")]
    UnknownTemplateParameter(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Invalid java script: {0}")]
    InvalidJavaScript(String),
}