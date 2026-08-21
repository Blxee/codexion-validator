use std::{error::Error, fmt::Display, time::Duration};

use crate::ProgramOutput;

#[derive(Debug)]
pub struct CodexionOutput {
    events: Vec<Event>,
    duration: Duration,
}

#[derive(Debug)]
pub struct Event {
    pub timestamp: Duration,
    pub coder_id: i32,
    pub action: Action,
    pub line: String,
    pub line_number: usize,
}

#[derive(Debug)]
pub enum Action {
    DongleTaken,
    Compile,
    Debug,
    Refactor,
    BurnOut,
}

#[derive(Debug)]
pub struct OutputParsingError {
    line: String,
    line_number: usize,
    kind: ParsingErrorKind,
}

#[derive(Debug)]
pub enum ParsingErrorKind {
    InvalidFormat { fields: usize },
    MissingColon,
    InvalidTimestamp,
    InvalidCoderID,
    InvalidAction,
}

impl Error for OutputParsingError {}

impl TryFrom<&ProgramOutput> for CodexionOutput {
    type Error = OutputParsingError;

    fn try_from(output: &ProgramOutput) -> Result<Self, Self::Error> {
        let mut events = Vec::new();

        for (number, line) in output.stdout.lines().enumerate() {
            let mut event: Event = line.try_into().map_err(|mut err: OutputParsingError| {
                err.line_number = number;
                err
            })?;
            event.line_number = number;
            events.push(event);
        }

        Ok(CodexionOutput {
            events,
            duration: output.duration,
        })
    }
}

impl TryFrom<&str> for Event {
    type Error = OutputParsingError;

    fn try_from(line: &str) -> Result<Self, Self::Error> {
        let spans = line.splitn(3, ' ').collect::<Vec<&str>>();
        let [timestamp, coder_id, action] = spans.as_slice() else {
            return Err(OutputParsingError {
                line: line.to_string(),
                line_number: 0,
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
                    line_number: 0,
                    kind: ParsingErrorKind::MissingColon,
                })?
                .parse()
                .map_err(|_| OutputParsingError {
                    line: line.to_string(),
                    line_number: 0,
                    kind: ParsingErrorKind::InvalidTimestamp,
                })?,
        );

        let coder_id = coder_id.parse().map_err(|_| OutputParsingError {
            line: line.to_string(),
            line_number: 0,
            kind: ParsingErrorKind::InvalidCoderID,
        })?;

        if coder_id <= 0 {
            return Err(OutputParsingError {
                line: line.to_string(),
                line_number: 0,
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
                    line_number: 0,
                    kind: ParsingErrorKind::InvalidAction,
                });
            }
        };

        Ok(Event {
            timestamp,
            coder_id,
            action,
            line: line.to_string(),
            line_number: 0,
        })
    }
}

impl Display for OutputParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
                ParsingErrorKind::InvalidFormat { fields } =>
                    format!("expected 3 parts (timestamp: coder_id action) but found {fields}"),
                ParsingErrorKind::MissingColon => "missing colon ':' after timestamp".to_string(),
                ParsingErrorKind::InvalidTimestamp =>
                    "could not parse timestamp into a valid unsigned long".to_string(),
                ParsingErrorKind::InvalidCoderID =>
                    "could not parse coder id into a valid positive integer".to_string(),
                ParsingErrorKind::InvalidAction =>
                    "invalid action format, expected either 'has taken a dongle', 'is compiling', 'is debugging', 'is refactoring' or 'burned out'".to_string(),
            };
        write!(
            f,
            "[Error at line {}]: '{}'\n{}",
            self.line_number, self.line, kind,
        )
    }
}
