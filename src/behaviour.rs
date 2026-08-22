use std::{error::Error, fmt::Display, time::Duration};

use crate::{
    args::ProcessedArgs,
    behaviour::DongleTakingError::TooManyDongles,
    parsing::{Action, Event},
};

pub struct CodexionState {
    args: ProcessedArgs,
    coders: Vec<Coder>,
    dongles: Vec<Dongle>,
    last_timestamp: Duration,
    burn_out_reached: bool,
}

struct Coder {
    id: u32,
    last_action: Option<Action>,
    last_action_timestamp: Duration,
    dongles_in_hand: u32,
    last_compile_timestamp: Duration,
}

struct Dongle {
    id: u32,
    state: DongleState,
}

enum DongleState {
    Unknown,
    Available,
    Held,
    CoolingDownUntil(Duration),
}

enum Availability {
    Available,
    Unavailable,
    Unknown,
}

#[derive(Debug)]
pub struct BehaviourError {
    pub line: String,
    pub line_number: usize,
    pub kind: BehaviourErrorKind,
}

#[derive(Debug)]
pub enum BehaviourErrorKind {
    UnsynchronizedTimestamps {
        last_timestamp: Duration,
        current_timestamp: Duration,
    },
    InvalidActionDuration {
        action: Action,
        expected_duration: Duration,
        found_duration: Duration,
    },
    InvalidCoderId {
        max_id: u32,
        found: u32,
    },
    InvalidActionOrder {
        last_coder_action: Option<Action>,
        current_coder_action: Action,
    },
    InvalidDongleTaking(DongleTakingError),
    InvalidCompilation(CompilationError),
    InvalidDebugging(DebuggingError),
    InvalidRefactoring(RefactoringError),
    InvalidBurnout(BurnoutError),
}

#[derive(Debug)]
enum DongleTakingError {
    TooManyDongles,
    UnavailableDongle,
}

#[derive(Debug)]
enum CompilationError {
    MissingDongles,
    // ExessDongles,
}

#[derive(Debug)]
enum DebuggingError {}

#[derive(Debug)]
enum RefactoringError {}

#[derive(Debug)]
enum BurnoutError {}

impl Error for BehaviourError {}

impl Coder {
    fn from_id(id: u32) -> Self {
        Self {
            id,
            last_action: None,
            last_action_timestamp: Duration::ZERO,
            dongles_in_hand: 0,
            last_compile_timestamp: Duration::ZERO,
        }
    }
}

impl Dongle {
    fn availability(&self, timestamp: Duration) -> Availability {
        match self.state {
            DongleState::Available => Availability::Available,
            DongleState::CoolingDownUntil(time_available) => {
                if timestamp >= time_available {
                    Availability::Available
                } else {
                    Availability::Unavailable
                }
            }
            DongleState::Held => Availability::Unknown,
            DongleState::Unknown => Availability::Unknown,
        }
    }
}

impl CodexionState {
    pub fn from(args: ProcessedArgs) -> Self {
        let mut coders = Vec::with_capacity(args.number_of_coders as usize);
        let mut dongles = Vec::with_capacity(args.number_of_coders as usize);

        for id in 1..=args.number_of_coders {
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
                kind: BehaviourErrorKind::UnsynchronizedTimestamps {
                    last_timestamp: self.last_timestamp,
                    current_timestamp: timestamp,
                },
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

    fn update_coder(&mut self, coder_id: u32, action: Action, timestamp: Duration) {
        let index = Self::id_to_index(coder_id);
        let coder = &mut self.coders[index];
        coder.last_action = Some(action);
        coder.last_action_timestamp = timestamp;

        match action {
            Action::DongleTaken => coder.dongles_in_hand += 1,
            Action::Debug => coder.dongles_in_hand = 0,
            Action::Compile => coder.last_compile_timestamp = timestamp,
            _ => (),
        }
    }

    fn update_dongles(&mut self, coder_id: u32, action: Action, timestamp: Duration) {
        let index = Self::id_to_index(coder_id);
        let coder = &self.coders[index];
        let (left_dongle, right_dongle) =
            Self::get_coder_nearby_dongles(coder_id, &mut self.dongles);

        match action {
            Action::DongleTaken => {
                if coder.dongles_in_hand == 1 {
                    left_dongle.state = DongleState::Unknown;
                    right_dongle.state = DongleState::Unknown;
                } else if coder.dongles_in_hand == 2 {
                    left_dongle.state = DongleState::Held;
                    right_dongle.state = DongleState::Held;
                }
            }
            Action::Compile => {
                left_dongle.state = DongleState::Held;
                right_dongle.state = DongleState::Held;
            }
            Action::Debug => {
                let time_to_cooldown = self.args.dongle_cooldown;
                left_dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown);
                right_dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown);
            }
            _ => (),
        }
    }

    fn get_coder_nearby_dongles(
        coder_id: u32,
        dongles: &mut Vec<Dongle>,
    ) -> (&mut Dongle, &mut Dongle) {
        let index = Self::id_to_index(coder_id);
        if index < dongles.len() - 1 {
            let (first, second) = dongles.split_at_mut(index + 1);
            (&mut first[index], &mut second[0])
        } else {
            let (first, second) = dongles.split_at_mut(index);
            (&mut second[0], &mut first[0])
        }
    }

    fn id_to_index(id: u32) -> usize {
        (id - 1) as usize
    }

    fn validate_dongle_taking(
        &mut self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        let coder = &self.coders[Self::id_to_index(coder_id)];
        let (left_dongle, right_dongle) =
            Self::get_coder_nearby_dongles(coder_id, &mut self.dongles);

        match (coder.last_action, coder.dongles_in_hand) {
            // validate that taking a dongle
            // should only come after refactoring or taking a first dongle
            (Some(Action::Refactor | Action::DongleTaken) | None, 0 | 1) => {
                // validate that when the coder tries to take a dongle,
                // one of the neighboring dongles at least must be available
                match (
                    left_dongle.availability(timestamp),
                    right_dongle.availability(timestamp),
                ) {
                    (Availability::Unavailable, Availability::Unavailable) => {
                        Err(BehaviourErrorKind::InvalidDongleTaking(
                            DongleTakingError::UnavailableDongle,
                        ))
                    }
                    _ => Ok(()),
                }
            }

            // check if coder is trying to take a third dongle
            (Some(Action::Refactor | Action::DongleTaken) | None, 2..) => Err(
                BehaviourErrorKind::InvalidDongleTaking(DongleTakingError::TooManyDongles),
            ),

            (_, 0 | 1) => Err(BehaviourErrorKind::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::DongleTaken,
            }),

            _ => Ok(()),
        }
    }

    fn timestamp_not_eqauls(
        timestamp_1: Duration,
        timestamp_2: Duration,
        tolerance: Duration,
    ) -> bool {
        let diff = if timestamp_1 > timestamp_2 {
            timestamp_1 - timestamp_2
        } else {
            timestamp_2 - timestamp_1
        };
        diff > tolerance
    }

    fn validate_compiling(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        let coder = &self.coders[Self::id_to_index(coder_id)];

        match (coder.last_action, coder.dongles_in_hand) {
            (Some(Action::DongleTaken), 2) => Ok(()),

            (_, 2) => Err(BehaviourErrorKind::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::Compile,
            }),

            (Some(Action::DongleTaken), 0 | 1) => Err(BehaviourErrorKind::InvalidCompilation(
                CompilationError::MissingDongles,
            )),

            _ => Ok(()),
        }
    }

    fn validate_debugging(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        let coder = &self.coders[Self::id_to_index(coder_id)];

        match coder.last_action {
            Some(Action::Compile) => {
                let last_action_duration = timestamp.saturating_sub(coder.last_action_timestamp);

                if Self::timestamp_not_eqauls(
                    last_action_duration,
                    self.args.time_to_compile,
                    Duration::from_millis(10),
                ) {
                    Err(BehaviourErrorKind::InvalidActionDuration {
                        action: coder.last_action.unwrap(),
                        expected_duration: self.args.time_to_compile,
                        found_duration: last_action_duration,
                    })
                } else {
                    Ok(())
                }
            }

            _ => Err(BehaviourErrorKind::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::Debug,
            }),
        }
    }

    fn validate_refactoring(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
    }

    fn validate_burning_out(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourErrorKind> {
        Ok(())
    }
}

impl Display for BehaviourError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.kind {
            BehaviourErrorKind::UnsynchronizedTimestamps { .. } => {
                "the timestamps are unsyncronized"
            }
            _ => "",
        };
        write!(
            f,
            "[Error at line {}]: '{}'\n{}",
            self.line_number, self.line, kind,
        )
    }
}
