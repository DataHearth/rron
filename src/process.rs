use std::{path::PathBuf, str::FromStr, time};

use chrono::{TimeDelta, Utc};
use hifitime::Duration;
use log::{error, info};
use tokio::task::JoinHandle;

use crate::{
    command::trigger_cmd,
    config::Command,
    errors::{CommandError, ProcessError},
};

pub fn crontab(
    name: String,
    crontab: String,
    tz: chrono_tz::Tz,
    before: Option<Command>,
    exec: Command,
    after: Option<Command>,
    log_file: Option<PathBuf>,
) -> Result<JoinHandle<()>, ProcessError> {
    let (before, exec, after) = get_commands(before, exec, after)?;
    let cr = cron::Schedule::from_str(&crontab).map_err(|e| ProcessError::CrontabParse(e))?;

    info!("{}: starting processes...", name);

    let handle = tokio::spawn(async move {
        for next in cr.upcoming(tz) {
            let now = Utc::now().with_timezone(&tz);
            info!("{name}: next trigger: {}", next.format("%d-%m-%Y %H:%M:%S"));

            let duration = next.with_timezone(&tz).signed_duration_since(now);

            if (next - now) < TimeDelta::zero() {
                continue;
            }

            tokio::time::sleep(
                duration
                    .to_std()
                    .expect(&format!("{name}: failed to get duration")),
            )
            .await;

            info!("{name}: executing pre-commands...");
            if let Err(e) = execute(&before, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            };

            info!("{name}: executing commands...");
            if let Err(e) = execute(&exec, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            };

            info!("{name}: executing post-commands...");
            if let Err(e) = execute(&after, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            }

            info!("{name}: every processes executed");
        }
    });

    Ok(handle)
}

pub fn basic(
    name: String,
    duration: String,
    before: Option<Command>,
    exec: Command,
    after: Option<Command>,
    log_file: Option<PathBuf>,
) -> Result<JoinHandle<()>, ProcessError> {
    let (before, exec, after) = get_commands(before, exec, after)?;
    let duration = Duration::from_str(&duration).map_err(|e| ProcessError::DurationParse(e))?;
    if duration.is_negative() {
        return Err(ProcessError::NegativeDuration);
    }
    let secs = duration.to_seconds();

    info!("{}: starting processes...", name);

    let handle = tokio::spawn(async move {
        loop {
            info!("{name}: next trigger in: {}", duration);

            tokio::time::sleep(time::Duration::from_secs_f64(secs)).await;

            info!("{name}: executing pre-commands...");
            if let Err(e) = execute(&before, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            };

            info!("{name}: executing commands...");
            if let Err(e) = execute(&exec, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            };

            info!("{name}: executing post-commands...");
            if let Err(e) = execute(&after, &name, &log_file) {
                error!("{name}: Command failed - {e}");
                continue;
            }

            info!("{name}: every processes executed");
        }
    });

    Ok(handle)
}

fn get_commands(
    before: Option<Command>,
    exec: Command,
    after: Option<Command>,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), ProcessError> {
    let before = if let Some(b) = before {
        let mut out = vec![];
        for (i, v) in b.into_iter().enumerate() {
            if v.is_empty() {
                return Err(ProcessError::EmptyCommand(String::from("before"), i));
            }

            out.push(v);
        }

        out
    } else {
        vec![]
    };
    let exec = {
        let mut out = vec![];
        for (i, x) in exec.into_iter().enumerate() {
            if x.is_empty() {
                return Err(ProcessError::EmptyCommand(String::from("exec"), i));
            }
            out.push(x);
        }

        out
    };
    let after = if let Some(a) = after {
        let mut out = vec![];
        for (i, v) in a.into_iter().enumerate() {
            if v.is_empty() {
                return Err(ProcessError::EmptyCommand(String::from("after"), i));
            }

            out.push(v);
        }

        out
    } else {
        vec![]
    };

    Ok((before, exec, after))
}

fn execute(
    cmds: &[String],
    job_name: &str,
    log_file: &Option<PathBuf>,
) -> Result<(), CommandError> {
    for cmd in cmds {
        trigger_cmd(cmd, job_name, log_file)?
    }

    Ok(())
}
