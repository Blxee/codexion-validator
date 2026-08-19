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
pub struct OutputParsingError {
    line: String,
    number: usize,
    kind: ParsingErrorKind,
}

#[derive(Debug)]
pub enum ParsingErrorKind {
    InvalidFormat { fields: usize },
    MissingColon,
    InavlidTimestamp,
    InvalidCoderID,
    InvalidAction,
}

impl Error for OutputParsingError {}

impl TryFrom<&(Args, ProgramOutput)> for CodexionOutput {
    type Error = OutputParsingError;

    fn try_from((args, output): &(Args, ProgramOutput)) -> Result<Self, Self::Error> {
        let mut events = Vec::new();

        for (number, line) in output.stdout.lines().enumerate() {
            let spans = line.splitn(3, ' ').collect::<Vec<&str>>();
            let [timestamp, coder_id, action] = spans.as_slice() else {
                return Err(OutputParsingError {
                    line: line.to_string(),
                    number,
                    kind: ParsingErrorKind::InvalidFormat {
                        fields: spans.len(),
                    },
                });
            };

            let timestamp = Duration::from_millis(
                timestamp
                    .strip_suffix(':')
                    .ok_or(OutputParsingError {
                        line: line.to_string(),
                        number,
                        kind: ParsingErrorKind::MissingColon,
                    })?
                    .parse()
                    .map_err(|_| OutputParsingError {
                        line: line.to_string(),
                        number,
                        kind: ParsingErrorKind::InavlidTimestamp,
                    })?,
            );

            let coder_id = coder_id.parse().map_err(|_| OutputParsingError {
                line: line.to_string(),
                number,
                kind: ParsingErrorKind::InvalidCoderID,
            })?;

            if coder_id < 0 {
                return Err(OutputParsingError {
                    line: line.to_string(),
                    number,
                    kind: ParsingErrorKind::InvalidCoderID,
                });
            }

            let action = match *action {
                "has taken a dongle" => Action::DongleTaken,
                "is compiling" => Action::Compile,
                "is debugging" => Action::Debug,
                "is refactoring" => Action::Refactor,
                "burned out" => Action::BurnOut,
                _ => {
                    return Err(OutputParsingError {
                        line: line.to_string(),
                        number,
                        kind: ParsingErrorKind::InvalidAction,
                    });
                }
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

impl Display for OutputParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
                ParsingErrorKind::InvalidFormat { fields } =>
                    format!("expected 3 parts (timestamp: coder_id action) but found {fields}"),
                ParsingErrorKind::MissingColon => "missing colon ':' after timestamp".to_string(),
                ParsingErrorKind::InavlidTimestamp =>
                    "could not parse timestamp into a valid unsigned long".to_string(),
                ParsingErrorKind::InvalidCoderID =>
                    "could not parse coder id into a valid positive integer".to_string(),
                ParsingErrorKind::InvalidAction =>
                    "invalid action format, expected either 'has taken a dongle', 'is compiling', 'is debugging', 'is refactoring' or 'burned out'".to_string(),
            };
        write!(
            f,
            "[Error at line {}]: '{}'\n{}",
            self.number, self.line, kind,
        )
    }
}
