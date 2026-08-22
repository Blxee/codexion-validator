use std::{error::Error, fmt::Display, time::Duration};

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
pub struct InvalidScheduler();

impl Error for InvalidScheduler {}

impl Display for InvalidScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inavlid scheduler")
    }
}

impl<'a> TryInto<ProcessedArgs> for RawArgs<'a> {
    type Error = Box<dyn Error>;

    fn try_into(self) -> Result<ProcessedArgs, Self::Error> {
        Ok(ProcessedArgs {
            number_of_coders: self.number_of_coders.parse()?,
            time_to_burnout: Duration::from_millis(self.time_to_burnout.parse()?),
            time_to_compile: Duration::from_millis(self.time_to_compile.parse()?),
            time_to_debug: Duration::from_millis(self.time_to_debug.parse()?),
            time_to_refactor: Duration::from_millis(self.time_to_refactor.parse()?),
            number_of_compiles_required: self.number_of_compiles_required.parse()?,
            dongle_cooldown: Duration::from_millis(self.dongle_cooldown.parse()?),
            scheduler: match self.scheduler {
                "fifo" => Scheduler::FIFO,
                "edf" => Scheduler::EDF,
                _ => return Err(Box::new(InvalidScheduler())),
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
