mod command;
mod config;
mod errors;
mod process;

use chrono::Utc;
use chrono_tz::Tz;
use clap::{Parser, Subcommand};
use config::Configuration;
use env_logger::Builder;
use log::{error, info};
use std::io::Write;
use std::{path::PathBuf, process::exit, str::FromStr};

#[derive(Parser, Debug)]
struct Cli {
    /// Path to the configuration file containing jobs
    #[arg(short, long)]
    config: Option<PathBuf>,

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

    let cfg = if let Some(cfg_path) = &cli.config {
        match Configuration::read_from_file(&cfg_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to read configuration file: {e}");
                exit(1)
            }
        }
    } else {
        let mut usr_cfg = match home::home_dir() {
            Some(v) => v,
            None => {
                eprintln!("Failed to get user home directory");
                exit(1)
            }
        };
        usr_cfg.push(".config/rron.toml");
        let etc_cfg = PathBuf::from("/etc/rron.toml");

        if usr_cfg.exists() {
            match Configuration::read_from_file(&usr_cfg) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to read configuration file: {e}");
                    exit(1)
                }
            }
        } else if etc_cfg.exists() {
            match Configuration::read_from_file(&etc_cfg) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to read configuration file: {e}");
                    exit(1)
                }
            }
        } else {
            eprintln!("No configuration file found. Exiting...");
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

    match cli.cmds {
        Commands::Run => {
            let mut handles = vec![];

            info!("Timezone in use: {tz}");

            if cfg.jobs.is_empty() {
                error!("No jobs found for the current configuration");
                exit(1)
            }

            cfg.jobs.into_iter().for_each(|job| {
                if !job.enable {
                    info!("{}: job disabled. Skipping...", job.name);
                    return;
                }

                let handle = match job.duration {
                    config::ProcessType::Basic(v) => crate::process::basic(
                        job.name.clone(),
                        v,
                        job.before,
                        job.exec,
                        job.after,
                        job.logs,
                    ),
                    config::ProcessType::Crontab(v) => crate::process::crontab(
                        job.name.clone(),
                        v,
                        tz,
                        job.before,
                        job.exec,
                        job.after,
                        job.logs,
                    ),
                };

                match handle {
                    Ok(v) => handles.push(v),
                    Err(e) => {
                        error!("{}: failed to start job: {e}", job.name);
                        exit(1)
                    }
                };
            });

            for handle in handles {
                handle.await.expect("Panic in crontab thread");
            }
        }
        Commands::Print => println!("{cfg}"),
    }
}
