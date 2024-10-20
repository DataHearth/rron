use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::Command,
};

use crate::errors::CommandError;

pub fn trigger_cmd(
    cmd: &str,
    job_name: &str,
    log_file: &Option<PathBuf>,
) -> Result<(), CommandError> {
    let mut cmd_split = cmd.split(" ");
    let out = Command::new(
        cmd_split
            .next()
            .expect(&format!("{}: should not be empty", job_name)),
    )
    .args(cmd_split)
    .output()
    .map_err(|e| CommandError::CmdError {
        job_name: job_name.into(),
        error: e,
    })?;

    let mut buf: Box<dyn Write> = if let Some(lf) = &log_file {
        let file = if lf.exists() {
            fs::File::options().write(true).append(true).open(lf)
        } else {
            fs::File::create(lf)
        };
        match file {
            Ok(v) => Box::new(v),
            Err(e) => {
                eprintln!(
                    "{}: Failed to open or create log file. Fallback to stdout: {e}",
                    job_name
                );
                Box::new(io::stdout())
            }
        }
    } else {
        Box::new(io::stdout())
    };

    buf.write_all(&out.stdout)
        .map_err(|e| CommandError::LogsBufferWrite {
            job_name: job_name.into(),
            out_buf: "stdout".into(),
            error: e,
        })?;
    buf.write_all(&out.stderr)
        .map_err(|e| CommandError::LogsBufferWrite {
            job_name: job_name.into(),
            out_buf: "stderr".into(),
            error: e,
        })?;

    if !out.status.success() {
        return Err(CommandError::CmdFailed {
            job_name: job_name.into(),
            cmd: truncate(cmd, 15),
        });
    }

    Ok(())
}

fn truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_string(),
        Some((idx, _)) => s[..idx].to_string(),
    }
}
