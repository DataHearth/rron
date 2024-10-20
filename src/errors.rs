use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("failed to read configuration file: {}", .0)]
    Io(#[from] io::Error),

    #[error("failed to convert file content: {}", .0)]
    Parse(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("failed to write {out_buf}: {error}")]
    LogsBufferWrite { out_buf: String, error: io::Error },
    #[error("failed to execute command ({cmd}): {error}")]
    CmdError { cmd: String, error: io::Error },
    #[error("non zero status ({})", .0)]
    CmdFailed(String),
}
