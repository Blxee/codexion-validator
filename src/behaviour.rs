use std::{error::Error, fmt::Display, time::Duration};

use crate::{
    CodexionInput,
    parsing::{Action, Event},
};

struct CodexionState {
    args: CodexionInput,
    coders: Vec<Coder>,
    dongles: Vec<Dongle>,
    last_timestamp: Duration,
}

struct Coder {
    id: i32,
}

struct Dongle {
    id: i32,
}

#[derive(Debug)]
struct BehaviourError {
    line: String,
    line_number: usize,
    kind: BehaviourErrorKind,
}

#[derive(Debug)]
enum BehaviourErrorKind {
    UnsycronizedTimestamps,
    InvalidCoderId { max_id: i32, found: i32 },
}

impl Error for BehaviourError {}

impl CodexionState {
    fn from(args: CodexionInput) -> Self {
        let mut coders = Vec::with_capacity(args.number_of_coders as usize);
        let mut dongles = Vec::with_capacity(args.number_of_coders as usize);

        for id in 0..args.number_of_coders {
            coders.push(Coder { id });
            dongles.push(Dongle { id });
        }

        Self {
            args,
            coders,
            dongles,
            last_timestamp: Duration::ZERO,
        }
    }

    pub fn update(
        &mut self,
        Event {
            timestamp,
            coder_id,
            action,
            line,
            line_number,
        }: Event,
    ) -> Result<(), BehaviourError> {
        if timestamp < self.last_timestamp {
            return Err(BehaviourError {
                line,
                line_number,
                kind: BehaviourErrorKind::UnsycronizedTimestamps,
            });
        }
        self.last_timestamp = timestamp;

        if coder_id >= self.args.number_of_coders {
            return Err(BehaviourError {
                line,
                line_number,
                kind: BehaviourErrorKind::InvalidCoderId {
                    max_id: self.args.number_of_coders,
                    found: coder_id,
                },
            });
        }

        match action {
            Action::DongleTaken => {
                if let Err(kind) = self.validate_dongle_taking(coder_id, timestamp) {
                    return Err(BehaviourError {
                        line,
                        line_number,
                        kind,
                    });
                }
            }
            Action::Compile => {
                if let Err(kind) = self.validate_compiling(coder_id, timestamp) {
                    return Err(BehaviourError {
                        line,
                        line_number,
                        kind,
                    });
                }
            }
            Action::Debug => {
                if let Err(kind) = self.validate_debugging(coder_id, timestamp) {
                    return Err(BehaviourError {
                        line,
                        line_number,
                        kind,
                    });
                }
            }
            Action::Refactor => {
                if let Err(kind) = self.validate_refactoring(coder_id, timestamp) {
                    return Err(BehaviourError {
                        line,
                        line_number,
                        kind,
                    });
                }
            }
            Action::BurnOut => {
                if let Err(kind) = self.validate_burning_out(coder_id, timestamp) {
                    return Err(BehaviourError {
                        line,
                        line_number,
                        kind,
                    });
                }
            }
        }

        Ok(())
    }

    fn validate_dongle_taking(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        todo!()
    }

    fn validate_compiling(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        todo!()
    }

    fn validate_debugging(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        todo!()
    }

    fn validate_refactoring(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        todo!()
    }

    fn validate_burning_out(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        todo!()
    }
}

impl Display for BehaviourError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            BehaviourErrorKind::UnsycronizedTimestamps => "the timestamps are unsyncronized",
            _ => "",
        };
        write!(
            f,
            "[Error at line {}]: '{}'\n{}",
            self.line_number, self.line, kind,
        )
    }
}
