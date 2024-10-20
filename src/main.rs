mod command;
mod config;
mod errors;

use std::{path::PathBuf, process::exit, str::FromStr};

use chrono::{TimeDelta, Utc};
use chrono_tz::Tz;
use clap::{Parser, Subcommand};
use command::trigger_cmd;
use config::Configuration;

#[derive(Parser, Debug)]
struct Cli {
    /// Path to the configuration file containing jobs
    #[arg(short, long, default_value=default_cfg_path().into_os_string())]
    config: PathBuf,

    #[command(subcommand)]
    cmds: Commands,
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
            eprintln!("{e}");
            exit(1)
        }
    };

    match cli.cmds {
        Commands::Run => {
            let mut handles = vec![];

            if cfg.jobs.is_empty() {
                eprintln!("No jobs found with current configuration")
            }

            for job in cfg.jobs {
                let cr = match cron::Schedule::from_str(job.crontab.as_str()) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{}: failed to parse crontab for job 1: {e}", job.name);
                        exit(1)
                    }
                };

                let tz: Tz = match cfg.tz.clone().unwrap_or("UTC".to_string()).parse() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{}: failed to parse timezone: {e}", job.name);
                        exit(1)
                    }
                };

                println!(
                    "{} - {}: starting processes...",
                    Utc::now().with_timezone(&tz).format("%Y-%m-%d %H:%M:%S %Z"),
                    job.name
                );
                let handle = tokio::spawn(async move {
                    for next in cr.upcoming(tz) {
                        let now = Utc::now().with_timezone(&tz);
                        println!("{}: next trigger: {next}", job.name);

                        let duration = next.with_timezone(&tz).signed_duration_since(now);

                        if (next - now) < TimeDelta::zero() {
                            continue;
                        }

                        tokio::time::sleep(
                            duration
                                .to_std()
                                .expect(&format!("{}: failed to get duration", job.name)),
                        )
                        .await;

                        if let Some(before) = &job.before {
                            for c in before.clone().into_iter() {
                                if c.is_empty() {
                                    eprintln!("{}: empty command", job.name);
                                    break;
                                }

                                if let Err(e) = trigger_cmd(&c, &job.name, &job.logs) {
                                    eprintln!("{e}");
                                    continue;
                                };

                                println!("{}: before command(s) executed with success", job.name);
                            }
                        }

                        for c in job.exec.clone().into_iter() {
                            if c.is_empty() {
                                eprintln!("{}: empty command", job.name);
                                break;
                            }

                            if let Err(e) = trigger_cmd(&c, &job.name, &job.logs) {
                                eprintln!("{e}");
                                continue;
                            };

                            println!("{}: main command(s) executed with success", job.name);
                        }

                        if let Some(after) = &job.after {
                            for c in after.clone().into_iter() {
                                if c.is_empty() {
                                    eprintln!("{}: empty command", job.name);
                                    break;
                                }

                                if let Err(e) = trigger_cmd(&c, &job.name, &job.logs) {
                                    eprintln!("{e}");
                                    continue;
                                };

                                println!("{}: after command(s) executed with success", job.name);
                            }
                        }

                        println!("{}: processes executed", job.name);
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await.expect("Panic in task");
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
