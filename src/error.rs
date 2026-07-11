use thiserror::Error;

#[derive(Error, Debug)]
pub enum StaircaseError {
    #[error("Git command failed: {command}\nStdout: {stdout}\nStderr: {stderr}")]
    GitCommandFailed {
        command: String,
        stdout: String,
        stderr: String,
    },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Staircase not found: {0}")]
    NotFound(String),
    #[error("Ambiguous staircase: {0}")]
    Ambiguous(String),
    #[error("Invalid staircase structure: {0}")]
    InvalidStructure(String),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StaircaseError>;
