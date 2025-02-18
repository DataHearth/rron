mod command;
mod config;
mod errors;

use chrono::{TimeDelta, Utc};
use chrono_tz::Tz;
use clap::{Parser, Subcommand};
use command::trigger_cmd;
use config::Configuration;
use env_logger::Builder;
use log::{error, info};
use std::io::Write;
use std::{path::PathBuf, process::exit, str::FromStr};

#[derive(Parser, Debug)]
struct Cli {
    /// Path to the configuration file containing jobs
    #[arg(short, long, default_value=default_cfg_path().into_os_string())]
    config: PathBuf,

    #[command(subcommand)]
    cmds: Commands,

    #[arg(short, long, default_value_t=String::from("info"))]
    log_level: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start jobs
    Run,

    /// Print the current configuration
    Print,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let cfg = match Configuration::read_from_file(&cli.config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read configuration file: {e}");
            exit(1)
        }
    };

    let tz: Tz = match cfg.tz.clone().unwrap_or("UTC".to_string()).parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to parse timezone: {e}");
            exit(1)
        }
    };

    Builder::new()
        .format(move |buf, record| {
            writeln!(
                buf,
                "{} {} - {}",
                Utc::now().with_timezone(&tz).format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(
            None,
            log::LevelFilter::from_str(&cli.log_level).unwrap_or_else(|_| {
                eprintln!("Unable to validate logger level. Defaulting to INFO");
                log::LevelFilter::Info
            }),
        )
        .init();

    info!("Timezone in use: {tz}");
    match cli.cmds {
        Commands::Run => {
            let mut handles = vec![];

            if cfg.jobs.is_empty() {
                error!("No jobs found for the current configuration");
                exit(1)
            }

            for (i, job) in cfg.jobs.into_iter().enumerate() {
                let cr = match cron::Schedule::from_str(job.crontab.as_str()) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{}: failed to parse crontab for job n°{i}: {e}", job.name);
                        exit(1)
                    }
                };

                info!("{}: starting processes...", job.name);
                let handle = tokio::spawn(async move {
                    let job_name = &job.name;

                    for next in cr.upcoming(tz) {
                        let now = Utc::now().with_timezone(&tz);
                        info!(
                            "{job_name}: next trigger: {}",
                            next.format("%d-%m-%Y %H:%M:%S")
                        );

                        let duration = next.with_timezone(&tz).signed_duration_since(now);

                        if (next - now) < TimeDelta::zero() {
                            continue;
                        }

                        tokio::time::sleep(
                            duration
                                .to_std()
                                .expect(&format!("{job_name}: failed to get duration")),
                        )
                        .await;

                        if let Some(before) = &job.before {
                            for c in before.clone().into_iter() {
                                if c.is_empty() {
                                    error!("{job_name}: empty command");
                                    break;
                                }

                                if let Err(e) = trigger_cmd(&c, job_name, &job.logs) {
                                    error!("{job_name}: Command failed - {e}");
                                    continue;
                                };

                                info!("{job_name}: pre-crontab command(s) executed with success");
                            }
                        }

                        for c in job.exec.clone().into_iter() {
                            if c.is_empty() {
                                error!("{job_name}: empty command");
                                break;
                            }

                            if let Err(e) = trigger_cmd(&c, job_name, &job.logs) {
                                error!("{job_name}: Command failed - {e}");
                                continue;
                            };

                            info!("{job_name}: crontab command(s) executed with success");
                        }

                        if let Some(after) = &job.after {
                            for c in after.clone().into_iter() {
                                if c.is_empty() {
                                    error!("{job_name}: empty command");
                                    break;
                                }

                                if let Err(e) = trigger_cmd(&c, job_name, &job.logs) {
                                    error!("{job_name}: Command failed - {e}");
                                    continue;
                                };

                                info!("{job_name}: post-crontab command(s) executed with success");
                            }
                        }

                        info!("{job_name}: every process executed");
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await.expect("Panic in crontab thread");
            }
        }
        Commands::Print => println!("{cfg}"),
    }
}

fn default_cfg_path() -> PathBuf {
    let mut home = match home::home_dir() {
        Some(v) => v,
        None => {
            eprintln!("Failed to get user home directory");
            exit(1)
        }
    };
    home.push(".rron.toml");

    return home;
}
