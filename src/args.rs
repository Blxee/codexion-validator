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
    TooManyArguments(usize),
    TooFewArguments(usize),
    InvalidNumber {
        argument: &'static str,
        source: ParseIntError,
    },
    InvalidScheduler,
}

impl Error for ArgsError {}

impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgsError::TooManyArguments(n) => {
                write!(f, "Error: too many arguments, expected 8 but found {n}")
            }
            ArgsError::TooFewArguments(n) => {
                write!(f, "Error: too few arguments, expected 8 but found {n}")
            }
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
                    argument: "number_of_coders",
                    source,
                }
            })?,
            time_to_burnout: Duration::from_millis(self.time_to_burnout.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_burnout",
                    source,
                },
            )?),
            time_to_compile: Duration::from_millis(self.time_to_compile.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_compile",
                    source,
                },
            )?),
            time_to_debug: Duration::from_millis(self.time_to_debug.parse().map_err(|source| {
                ArgsError::InvalidNumber {
                    argument: "time_to_debug",
                    source,
                }
            })?),
            time_to_refactor: Duration::from_millis(self.time_to_refactor.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "time_to_refactor",
                    source,
                },
            )?),
            number_of_compiles_required: self.number_of_compiles_required.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "number_of_compiles_required",
                    source,
                },
            )?,
            dongle_cooldown: Duration::from_millis(self.dongle_cooldown.parse().map_err(
                |source| ArgsError::InvalidNumber {
                    argument: "dongle_cooldown",
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

impl<'a> TryFrom<&'a str> for RawArgs<'a> {
    type Error = ArgsError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let args: Vec<_> = value.split_whitespace().collect();

        match args.len() {
            n @ ..=7 => Err(ArgsError::TooFewArguments(n)),
            n @ 9.. => Err(ArgsError::TooManyArguments(n)),
            8 => Ok(Self {
                number_of_coders: args[0],
                time_to_burnout: args[1],
                time_to_compile: args[2],
                time_to_debug: args[3],
                time_to_refactor: args[4],
                number_of_compiles_required: args[5],
                dongle_cooldown: args[6],
                scheduler: args[7],
            }),
        }
    }
}

impl<'a> Into<String> for RawArgs<'a> {
    fn into(self) -> String {
        [
            self.number_of_coders,
            self.time_to_burnout,
            self.time_to_compile,
            self.time_to_debug,
            self.time_to_refactor,
            self.number_of_compiles_required,
            self.dongle_cooldown,
            self.scheduler,
        ]
        .join(" ")
    }
}
