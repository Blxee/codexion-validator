use std::{error::Error, fmt::Display, time::Duration};

use crate::{
    CodexionInput,
    parsing::{Action, Event},
};

pub struct CodexionState {
    args: CodexionInput,
    coders: Vec<Coder>,
    dongles: Vec<Dongle>,
    last_timestamp: Duration,
    burn_out_reached: bool,
}

struct Coder {
    id: i32,
    last_action: Option<Action>,
    last_action_timestamp: Option<Duration>,
    dongles_in_hand: u32,
}

struct Dongle {
    id: i32,
    state: DongleState,
}

enum DongleState {
    Unknown,
    Available,
    Held,
    CoolingDownUntil(Duration),
}

#[derive(Debug)]
pub struct BehaviourError {
    pub line: String,
    pub line_number: usize,
    pub kind: BehaviourErrorKind,
}

#[derive(Debug)]
pub enum BehaviourErrorKind {
    UnsycronizedTimestamps,
    InvalidCoderId { max_id: i32, found: i32 },
    CoderTookADongleOutLfOrder,
    CoderTookMoreThanTwoDongles,
}

impl Error for BehaviourError {}

impl Coder {
    fn from_id(id: i32) -> Self {
        Self {
            id,
            last_action: None,
            last_action_timestamp: None,
            dongles_in_hand: 0,
        }
    }
}

impl CodexionState {
    pub fn from(args: CodexionInput) -> Self {
        let mut coders = Vec::with_capacity(args.number_of_coders as usize);
        let mut dongles = Vec::with_capacity(args.number_of_coders as usize);

        for id in 0..args.number_of_coders {
            coders.push(Coder::from_id(id));
            dongles.push(Dongle {
                id,
                state: DongleState::Available,
            });
        }

        Self {
            args,
            coders,
            dongles,
            last_timestamp: Duration::ZERO,
            burn_out_reached: false,
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
        // validate common event attributes
        if timestamp < self.last_timestamp {
            return Err(BehaviourError {
                line,
                line_number,
                kind: BehaviourErrorKind::UnsycronizedTimestamps,
            });
        }
        if coder_id > self.args.number_of_coders {
            return Err(BehaviourError {
                line,
                line_number,
                kind: BehaviourErrorKind::InvalidCoderId {
                    max_id: self.args.number_of_coders,
                    found: coder_id,
                },
            });
        }
        // validate behaviour/logic according to action
        if let Err(kind) = match action {
            Action::DongleTaken => self.validate_dongle_taking(coder_id, timestamp),
            Action::Compile => self.validate_compiling(coder_id, timestamp),
            Action::Debug => self.validate_debugging(coder_id, timestamp),
            Action::Refactor => self.validate_refactoring(coder_id, timestamp),
            Action::BurnOut => self.validate_burning_out(coder_id, timestamp),
        } {
            return Err(BehaviourError {
                line,
                line_number,
                kind,
            });
        }
        // update state
        self.last_timestamp = timestamp;
        self.update_coder(coder_id, action, timestamp);
        self.update_dongles(coder_id, action, timestamp);

        Ok(())
    }

    fn update_coder(&mut self, coder_id: i32, action: Action, timestamp: Duration) {
        let index = self.id_to_index(coder_id);
        let coder = &mut self.coders[index];
        coder.last_action = Some(action);
        coder.last_action_timestamp = Some(timestamp);

        match action {
            Action::DongleTaken => coder.dongles_in_hand += 1,
            Action::Debug => coder.dongles_in_hand = 0,
            _ => (),
        }
    }

    fn update_dongles(&mut self, coder_id: i32, action: Action, timestamp: Duration) {
        let index = self.id_to_index(coder_id);
        let coder = &self.coders[index];
        let (left_dongle, right_dongle) = if index < self.dongles.len() - 1 {
            let (first, second) = self.dongles.split_at_mut(index + 1);
            (&mut first[index], &mut second[0])
        } else {
            let (first, second) = self.dongles.split_at_mut(index);
            (&mut second[0], &mut first[0])
        };

        match action {
            Action::DongleTaken => {
                if coder.dongles_in_hand == 2 {
                    left_dongle.state = DongleState::Held;
                    right_dongle.state = DongleState::Held;
                }
            }
            Action::Compile => {
                left_dongle.state = DongleState::Held;
                right_dongle.state = DongleState::Held;
            }
            Action::Debug => {
                let time_to_cooldown = Duration::from_millis(self.args.dongle_cooldown as u64);
                left_dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown);
                right_dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown);
            }
            _ => (),
        }
    }

    fn id_to_index(&self, id: i32) -> usize {
        (id - 1) as usize
    }

    fn validate_dongle_taking(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        let coder = &self.coders[self.id_to_index(coder_id)];

        if let Some(last_action) = coder.last_action {
            if ![Action::Refactor, Action::DongleTaken].contains(&last_action) {
                return Err(BehaviourErrorKind::CoderTookADongleOutLfOrder);
            }
        }

        if coder.dongles_in_hand >= 2 {
            return Err(BehaviourErrorKind::CoderTookMoreThanTwoDongles);
        }

        Ok(())
    }

    fn validate_compiling(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
    }

    fn validate_debugging(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
    }

    fn validate_refactoring(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
    }

    fn validate_burning_out(
        &self,
        coder_id: i32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
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
