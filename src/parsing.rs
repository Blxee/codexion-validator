use std::{error::Error, fmt::Display, time::Duration};

use crate::{Args, ProgramOutput};

#[derive(Debug)]
pub struct CodexionOutput {
    args: Args,
    events: Vec<Event>,
    duration: Duration,
}

#[derive(Debug)]
struct Event {
    timestamp: Duration,
    coder_id: i32,
    action: Action,
}

#[derive(Debug)]
enum Action {
    DongleTaken,
    Compile,
    Debug,
    Refactor,
    BurnOut,
}

#[derive(Debug)]
pub enum ParsingError {
    WrongTimeStampFormat,
}

impl Error for ParsingError {}

impl TryFrom<&(Args, ProgramOutput)> for CodexionOutput {
    type Error = ParsingError;

    fn try_from((args, output): &(Args, ProgramOutput)) -> Result<Self, Self::Error> {
        let mut events = Vec::new();

        for line in output.stdout.lines() {
            let spans = line.splitn(3, ' ').collect::<Vec<&str>>();
            let [timestamp, coder_id, action] = spans.as_slice() else {
                return Err(ParsingError::WrongTimeStampFormat);
            };

            let timestamp = Duration::from_millis(
                timestamp
                    .strip_suffix(':')
                    .ok_or(ParsingError::WrongTimeStampFormat)?
                    .parse()
                    .map_err(|_| ParsingError::WrongTimeStampFormat)?,
            );

            let coder_id = coder_id
                .parse()
                .map_err(|_| ParsingError::WrongTimeStampFormat)?;

            let action = match *action {
                "has taken a dongle" => Action::DongleTaken,
                "is compiling" => Action::Compile,
                "is debugging" => Action::Debug,
                "is refactoring" => Action::Refactor,
                "burned out" => Action::BurnOut,
                _ => return Err(ParsingError::WrongTimeStampFormat),
            };

            events.push(Event {
                timestamp,
                coder_id,
                action,
            });
        }

        Ok(CodexionOutput {
            args: *args,
            events,
            duration: output.duration,
        })
    }
}

impl Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
