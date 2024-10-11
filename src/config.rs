use std::{
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::Deserialize;

use crate::errors::ConfigurationError;

#[derive(Deserialize)]
pub struct Configuration {
    /// Timezone configuration for jobs
    pub tz: Option<String>,

    /// List of jobs to run
    pub jobs: Vec<Job>,
}

#[derive(Deserialize)]
pub struct Job {
    /// Job name
    pub name: String,
    /// crontab expression
    pub crontab: String,
    /// Commands to execute. Could be a single command or multiple at once
    pub exec: Command,
    /// Commands to execute before the main process
    pub before: Option<Command>,
    /// Commands to execute after the main process
    pub after: Option<Command>,
    /// Logs file path
    pub logs: Option<PathBuf>,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Command {
    Single(String),
    Multiple(Vec<String>),
}

impl Configuration {
    pub fn read_from_file(path: &Path) -> Result<Self, ConfigurationError> {
        let f = fs::read_to_string(path)?;

        Ok(Configuration::from_str(&f)?)
    }
}

impl FromStr for Configuration {
    type Err = ConfigurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str(s)?)
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tz = if let Some(tz) = &self.tz { tz } else { "UTC" };
        writeln!(f, "Timezone: {tz}")?;
        writeln!(f, "Jobs:")?;

        for j in self.jobs.iter() {
            writeln!(f, "\t- name: {}", j.name)?;
            writeln!(f, "\t  crontab: {}", j.crontab)?;
            if let Some(lf) = &j.logs {
                writeln!(f, "\t  logs file: {:?}", lf)?;
            }
            writeln!(f, "\t  commands: {}", j.exec.custom_display("\t\t"))?;
            if let Some(b) = &j.before {
                writeln!(f, "\t  before: {}", b.custom_display("\t\t"))?;
            }
            if let Some(a) = &j.after {
                writeln!(f, "\t  after: {}", a.custom_display("\t\t"))?;
            }
            writeln!(f, "")?;
        }

        writeln!(f)
    }
}

impl Command {
    fn custom_display(&self, init_str: &str) -> String {
        match self {
            Command::Single(v) => format!("\n{init_str}- {v}"),
            Command::Multiple(v) => {
                let mut end = String::new();

                for c in v {
                    end.push_str(&format!("\n{init_str}- {c}"));
                }
                end
            }
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Single(v) => write!(f, "{}", v),
            Command::Multiple(v) => {
                writeln!(f)?;
                for c in v {
                    writeln!(f, "- {}", c)?;
                }
                writeln!(f)
            }
        }
    }
}

impl IntoIterator for Command {
    type Item = String;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Command::Single(s) => vec![s].into_iter(),
            Command::Multiple(m) => m.into_iter(),
        }
    }
}
