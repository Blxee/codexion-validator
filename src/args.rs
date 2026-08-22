use std::{error::Error, fmt::Display, num::ParseIntError, time::Duration};

#[derive(Clone, Copy, Debug)]
pub struct RawArgs<'a> {
    pub number_of_coders: &'a str,
    pub time_to_burnout: &'a str,
    pub time_to_compile: &'a str,
    pub time_to_debug: &'a str,
    pub time_to_refactor: &'a str,
    pub number_of_compiles_required: &'a str,
    pub dongle_cooldown: &'a str,
    pub scheduler: &'a str,
}

#[derive(Clone, Copy)]
pub struct ProcessedArgs {
    pub number_of_coders: u32,
    pub time_to_burnout: Duration,
    pub time_to_compile: Duration,
    pub time_to_debug: Duration,
    pub time_to_refactor: Duration,
    pub number_of_compiles_required: u32,
    pub dongle_cooldown: Duration,
    pub scheduler: Scheduler,
}

#[derive(Debug, Clone, Copy)]
pub enum Scheduler {
    FIFO,
    EDF,
}

#[derive(Debug)]
pub enum ArgsError {
    InvalidNumber {
        argument: String,
        source: ParseIntError,
    },
    InvalidScheduler,
}

impl Error for ArgsError {}

impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::InvalidNumber { argument, source } => {
                write!(f, "Error: invalid number for argument {argument}: {source}")
            }
            ArgsError::InvalidScheduler => write!(f, "Error: invalid scheduler"),
        }
    }
}

impl<'a> TryInto<ProcessedArgs> for RawArgs<'a> {
    type Error = ArgsError;

    fn try_into(self) -> Result<ProcessedArgs, Self::Error> {
        Ok(ProcessedArgs {
            number_of_coders: self.number_of_coders.parse().map_err(|source| {
                ArgsError::InvalidNumber {
                    argument: "number_of_coders".to_string(),
                    source,
                }
            })?,
            time_to_burnout: Duration::from_millis(self.time_to_burnout.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_burnout".to_string(),
                    source,
                },
            )?),
            time_to_compile: Duration::from_millis(self.time_to_compile.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_compile".to_string(),
                    source,
                },
            )?),
            time_to_debug: Duration::from_millis(self.time_to_debug.parse().map_err(|source| {
                ArgsError::InvalidNumber {
                    argument: "time_to_debug".to_string(),
                    source,
                }
            })?),
            time_to_refactor: Duration::from_millis(self.time_to_refactor.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_refactor".to_string(),
                    source,
                },
            )?),
            number_of_compiles_required: self.number_of_compiles_required.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "number_of_compiles_required".to_string(),
                    source,
                },
            )?,
            dongle_cooldown: Duration::from_millis(self.dongle_cooldown.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "dongle_cooldown".to_string(),
                    source,
                },
            )?),
            scheduler: match self.scheduler {
                "fifo" => Scheduler::FIFO,
                "edf" => Scheduler::EDF,
                _ => return Err(ArgsError::InvalidScheduler),
            },
        })
    }
}

impl<'a> RawArgs<'a> {
    pub fn to_vec(self) -> Vec<&'a str> {
        vec![
            self.number_of_coders,
            self.time_to_burnout,
            self.time_to_compile,
            self.time_to_debug,
            self.time_to_refactor,
            self.number_of_compiles_required,
            self.dongle_cooldown,
            self.scheduler,
        ]
    }
}
