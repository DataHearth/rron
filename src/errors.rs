use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Failed to read configuration file: {}", .0)]
    Io(#[from] io::Error),

    #[error("Failed to convert file content: {}", .0)]
    Parse(#[from] toml::de::Error),
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("{job_name}: failed to write {out_buf}: {error}")]
    LogsBufferWrite {
        job_name: String,
        out_buf: String,
        error: io::Error,
    },
    #[error("{job_name}: process error: {error}")]
    CmdError { job_name: String, error: io::Error },
}

#[derive(Error, Debug)]
pub enum Error {}
