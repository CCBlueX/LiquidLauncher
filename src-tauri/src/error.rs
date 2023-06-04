use thiserror::Error;

#[derive(Error, Debug)]
pub enum LauncherError {
    #[error("Invalid version profile: {0}")]
    InvalidVersionProfile(String),
    #[error("Unknown template parameter: {0}")]
    UnknownTemplateParameter(String),
}

