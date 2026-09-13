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
    earliest_burnout_coder_idx: usize,
    burn_out_reached: bool,
    action_duration_tolerance: Duration,
}

struct Coder {
    id: u32,
    last_action: Option<Action>,
    last_action_timestamp: Duration,
    dongles_in_hand: u32,
    compile_count: u32,
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
pub enum BehaviourError {
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
    InvalidBurnout(BurnoutError),
}

#[derive(Debug)]
pub enum DongleTakingError {
    TooManyDongles,
    UnavailableDongle,
    CoderCouldHaveTakenDongle { coder_id: u32, timestamp: Duration },
}

#[derive(Debug)]
pub enum CompilationError {
    MissingDongles,
    NotEnoughCompiles,
    ExceedingMaxCompiles,
    CoderCouldHaveCompiled { coder_id: u32, timestamp: Duration },
}

#[derive(Debug)]
pub enum BurnoutError {
    BurnoutNotDetected {
        coder_id: u32,
        timestamp: Duration,
    },
    ShouldNotBurnout {
        coder_id: u32,
        burnout_until: Duration,
    },
    BurnoutAlreadyReached,
}

impl Error for BehaviourError {}

impl Coder {
    fn from_id(id: u32) -> Self {
        Self {
            id,
            last_action: None,
            last_action_timestamp: Duration::ZERO,
            dongles_in_hand: 0,
            last_compile_timestamp: Duration::ZERO,
            compile_count: 0,
        }
    }

    fn get_nearby_dongles<'a>(
        &'a self,
        dongles: &'a mut Vec<Dongle>,
    ) -> (Option<&'a mut Dongle>, Option<&'a mut Dongle>) {
        let index = CodexionState::id_to_index(self.id);
        if dongles.len() == 1 {
            (Some(&mut dongles[index]), None)
        } else if index < dongles.len() - 1 {
            let (first, second) = dongles.split_at_mut(index + 1);
            (Some(&mut first[index]), Some(&mut second[0]))
        } else {
            let (first, second) = dongles.split_at_mut(index);
            (Some(&mut second[0]), Some(&mut first[0]))
        }
    }

    fn is_waiting_for_dongles(
        &self,
        timestamp: Duration,
        time_to_refactor: Duration,
        action_duration_tolerance: Duration,
    ) -> bool {
        self.dongles_in_hand == 0
            && match self.last_action {
                None if timestamp > action_duration_tolerance => true,
                Some(Action::Refactor)
                    if self.last_action_timestamp + time_to_refactor
                        > timestamp + action_duration_tolerance =>
                {
                    true
                }
                _ => false,
            }
    }
}

trait AvailabilityProvider {
    fn availability(&self, timestamp: Duration) -> Availability;

    fn available(&self, timestamp: Duration) -> bool {
        matches!(self.availability(timestamp), Availability::Available)
    }
}

impl AvailabilityProvider for Dongle {
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

impl AvailabilityProvider for Option<&mut Dongle> {
    fn availability(&self, timestamp: Duration) -> Availability {
        match self {
            Some(dongle) => dongle.availability(timestamp),
            None => Availability::Unavailable,
        }
    }
}

impl AvailabilityProvider for (Option<&mut Dongle>, Option<&mut Dongle>) {
    fn availability(&self, timestamp: Duration) -> Availability {
        match (
            self.0.availability(timestamp),
            self.1.availability(timestamp),
        ) {
            (Availability::Available, Availability::Available) => Availability::Available,
            (
                Availability::Available | Availability::Unknown,
                Availability::Available | Availability::Unknown,
            ) => Availability::Unknown,
            _ => Availability::Unavailable,
        }
    }
}

impl CodexionState {
    pub fn from(args: ProcessedArgs, action_duration_tolerance: Duration) -> Self {
        let mut coders = Vec::new();
        let mut dongles = Vec::new();

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
            earliest_burnout_coder_idx: 0,
            burn_out_reached: false,
            action_duration_tolerance,
        }
    }

    pub fn update(
        &mut self,
        Event {
            timestamp,
            coder_id,
            action,
            ..
        }: Event,
    ) -> Result<(), BehaviourError> {
        let mut result = Ok(());

        // check whether a burnout already happened
        if self.burn_out_reached {
            result = Err(BehaviourError::InvalidBurnout(
                BurnoutError::BurnoutAlreadyReached,
            ));
        }
        // validate common event attributes
        if result.is_ok() && timestamp < self.last_timestamp {
            result = Err(BehaviourError::UnsynchronizedTimestamps {
                last_timestamp: self.last_timestamp,
                current_timestamp: timestamp,
            });
        }
        if result.is_ok() && coder_id > self.args.number_of_coders {
            result = Err(BehaviourError::InvalidCoderId {
                max_id: self.args.number_of_coders,
                found: coder_id,
            });
        }
        // check whether a burnout should happen
        let earliest_burnout_timestamp =
            self.coders[self.earliest_burnout_coder_idx].last_compile_timestamp;
        if result.is_ok()
            && timestamp.saturating_sub(earliest_burnout_timestamp)
                > self.args.time_to_burnout + self.action_duration_tolerance
        {
            self.burn_out_reached = true;
            result = Err(BehaviourError::InvalidBurnout(
                BurnoutError::BurnoutNotDetected {
                    coder_id,
                    timestamp: earliest_burnout_timestamp + self.args.time_to_burnout,
                },
            ));
        }
        // check if a coder could have compiled
        for coder in &self.coders {
            if result.is_ok()
                && coder.dongles_in_hand == 2
                && matches!(coder.last_action, Some(Action::DongleTaken))
                && !self.is_duration_within_tolerance(coder.last_action_timestamp, timestamp)
            {
                result = Err(BehaviourError::InvalidCompilation(
                    CompilationError::CoderCouldHaveCompiled {
                        coder_id: coder.id,
                        timestamp: coder.last_action_timestamp,
                    },
                ));
            }
            if result.is_ok()
                && !matches!(action, Action::DongleTaken)
                && coder.is_waiting_for_dongles(
                    timestamp,
                    self.args.time_to_refactor,
                    self.action_duration_tolerance,
                )
                && coder
                    .get_nearby_dongles(&mut self.dongles)
                    .available(timestamp)
            {
                result = Err(BehaviourError::InvalidDongleTaking(
                    DongleTakingError::CoderCouldHaveTakenDongle {
                        coder_id: coder.id,
                        timestamp: coder.last_action_timestamp,
                    },
                ));
            }
        }
        // validate behaviour/logic according to action
        if result.is_ok()
            && let Err(err) = match action {
                Action::DongleTaken => self.validate_dongle_taking(coder_id, timestamp),
                Action::Compile => self.validate_compiling(coder_id),
                Action::Debug => self.validate_debugging(coder_id, timestamp),
                Action::Refactor => self.validate_refactoring(coder_id, timestamp),
                Action::Burnout => self.validate_burning_out(coder_id, timestamp),
            }
        {
            result = Err(err)
        };

        // update state
        self.last_timestamp = timestamp;
        self.update_coder(coder_id, action, timestamp);
        self.update_dongles(coder_id, action, timestamp);
        if matches!(action, Action::Burnout) {
            self.burn_out_reached = true;
        }

        result
    }

    pub fn finish(&self) -> Result<(), BehaviourError> {
        if self.burn_out_reached {
            return Ok(());
        }
        let coder_with_least_compiles = self
            .coders
            .iter()
            .map(|coder| coder.compile_count)
            .min()
            .unwrap();
        if coder_with_least_compiles < self.args.number_of_compiles_required {
            return Err(BehaviourError::InvalidCompilation(
                CompilationError::NotEnoughCompiles,
            ));
        }
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
            Action::Compile => {
                coder.compile_count += 1;
                coder.last_compile_timestamp = timestamp;
                // refresh the coder closest to burnout
                self.earliest_burnout_coder_idx = self
                    .coders
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, coder)| coder.last_compile_timestamp)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
            _ => (),
        }
    }

    fn update_dongles(&mut self, coder_id: u32, action: Action, timestamp: Duration) {
        let index = Self::id_to_index(coder_id);
        let coder = &self.coders[index];
        let (left_dongle, right_dongle) = coder.get_nearby_dongles(&mut self.dongles);

        match action {
            Action::DongleTaken => {
                if coder.dongles_in_hand == 1 {
                    left_dongle.map(|dongle| dongle.state = DongleState::Unknown);
                    right_dongle.map(|dongle| dongle.state = DongleState::Unknown);
                } else if coder.dongles_in_hand == 2 {
                    left_dongle.map(|dongle| dongle.state = DongleState::Held);
                    right_dongle.map(|dongle| dongle.state = DongleState::Held);
                }
            }
            Action::Compile => {
                left_dongle.map(|dongle| dongle.state = DongleState::Held);
                right_dongle.map(|dongle| dongle.state = DongleState::Held);
            }
            Action::Debug => {
                let time_to_cooldown = self.args.dongle_cooldown;
                left_dongle.map(|dongle| {
                    dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown)
                });
                right_dongle.map(|dongle| {
                    dongle.state = DongleState::CoolingDownUntil(timestamp + time_to_cooldown)
                });
            }
            _ => (),
        }
    }

    fn id_to_index(id: u32) -> usize {
        (id - 1) as usize
    }

    fn validate_dongle_taking(
        &mut self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourError> {
        let coder = &self.coders[Self::id_to_index(coder_id)];
        let (left_dongle, right_dongle) = coder.get_nearby_dongles(&mut self.dongles);

        match (coder.last_action, coder.dongles_in_hand) {
            // validate that taking a dongle
            // should only come after refactoring or taking a first dongle
            (Some(Action::Refactor | Action::DongleTaken) | None, 0 | 1) => {
                // if the last action was a refactor
                if let Some(action @ Action::Refactor) = coder.last_action {
                    let duration_since_refactoring_start =
                        timestamp.saturating_sub(coder.last_action_timestamp);
                    // validate that at least refactoring time has passed
                    if duration_since_refactoring_start < self.args.time_to_refactor {
                        return Err(BehaviourError::InvalidActionDuration {
                            action,
                            expected_duration: self.args.time_to_refactor,
                            found_duration: duration_since_refactoring_start,
                        });
                    }
                }
                // validate that when the coder tries to take a dongle,
                // one of the neighboring dongles at least must be available
                match (
                    left_dongle.availability(timestamp),
                    right_dongle.availability(timestamp),
                ) {
                    (Availability::Unavailable, Availability::Unavailable) => Err(
                        BehaviourError::InvalidDongleTaking(DongleTakingError::UnavailableDongle),
                    ),
                    _ => Ok(()),
                }
            }

            // check if coder is trying to take a third dongle
            (Some(Action::Refactor | Action::DongleTaken) | None, 2..) => Err(
                BehaviourError::InvalidDongleTaking(DongleTakingError::TooManyDongles),
            ),

            (_, 0 | 1) => Err(BehaviourError::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::DongleTaken,
            }),

            _ => Ok(()),
        }
    }

    fn is_duration_within_tolerance(&self, timestamp_1: Duration, timestamp_2: Duration) -> bool {
        let diff = timestamp_1.abs_diff(timestamp_2);
        diff <= self.action_duration_tolerance
    }

    fn validate_compiling(&self, coder_id: u32) -> Result<(), BehaviourError> {
        // check whether a coder is compiling
        // even after all have reached number of compiles required
        let coder_with_least_compiles = self
            .coders
            .iter()
            .map(|coder| coder.compile_count)
            .min()
            .unwrap();
        if coder_with_least_compiles >= self.args.number_of_compiles_required {
            return Err(BehaviourError::InvalidCompilation(
                CompilationError::ExceedingMaxCompiles,
            ));
        }

        let coder = &self.coders[Self::id_to_index(coder_id)];

        match (coder.last_action, coder.dongles_in_hand) {
            (Some(Action::DongleTaken), 2) => Ok(()),

            (_, 2) => Err(BehaviourError::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::Compile,
            }),

            (Some(Action::DongleTaken), 0 | 1) => Err(BehaviourError::InvalidCompilation(
                CompilationError::MissingDongles,
            )),

            _ => Ok(()),
        }
    }

    fn validate_debugging(&self, coder_id: u32, timestamp: Duration) -> Result<(), BehaviourError> {
        let coder = &self.coders[Self::id_to_index(coder_id)];

        match coder.last_action {
            Some(action @ Action::Compile) => {
                let last_action_duration = timestamp.saturating_sub(coder.last_action_timestamp);

                if !self
                    .is_duration_within_tolerance(last_action_duration, self.args.time_to_compile)
                {
                    Err(BehaviourError::InvalidActionDuration {
                        action,
                        expected_duration: self.args.time_to_compile,
                        found_duration: last_action_duration,
                    })
                } else {
                    Ok(())
                }
            }

            _ => Err(BehaviourError::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::Debug,
            }),
        }
    }

    fn validate_refactoring(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourError> {
        let coder = &self.coders[Self::id_to_index(coder_id)];

        match coder.last_action {
            Some(action @ Action::Debug) => {
                let last_action_duration = timestamp.saturating_sub(coder.last_action_timestamp);

                if !self.is_duration_within_tolerance(last_action_duration, self.args.time_to_debug)
                {
                    Err(BehaviourError::InvalidActionDuration {
                        action,
                        expected_duration: self.args.time_to_debug,
                        found_duration: last_action_duration,
                    })
                } else {
                    Ok(())
                }
            }

            _ => Err(BehaviourError::InvalidActionOrder {
                last_coder_action: coder.last_action,
                current_coder_action: Action::Refactor,
            }),
        }
    }

    fn validate_burning_out(
        &self,
        coder_id: u32,
        timestamp: Duration,
    ) -> Result<(), BehaviourError> {
        let last_compile_timestamp =
            self.coders[Self::id_to_index(coder_id)].last_compile_timestamp;
        let minimum_time_to_burnout = last_compile_timestamp + self.args.time_to_burnout;

        if timestamp < minimum_time_to_burnout {
            return Err(BehaviourError::InvalidBurnout(
                BurnoutError::ShouldNotBurnout {
                    coder_id,
                    burnout_until: minimum_time_to_burnout,
                },
            ));
        }

        let coder = &self.coders[Self::id_to_index(coder_id)];

        if coder.dongles_in_hand == 2
            && matches!(coder.last_action, Some(Action::DongleTaken))
            && !self.is_duration_within_tolerance(coder.last_action_timestamp, timestamp)
        {
            return Err(BehaviourError::InvalidCompilation(
                CompilationError::CoderCouldHaveCompiled {
                    coder_id,
                    timestamp: coder.last_action_timestamp,
                },
            ));
        }
        Ok(())
    }
}

impl Display for BehaviourError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BehaviourError as Behaviour;

        match &self {
            Behaviour::UnsynchronizedTimestamps {
                last_timestamp,
                current_timestamp,
            } => write!(
                f,
                "timestamps are not synchronized (last: {}ms, current: {}ms)",
                last_timestamp.as_millis(),
                current_timestamp.as_millis()
            ),
            Behaviour::InvalidActionDuration {
                action,
                expected_duration,
                found_duration,
            } => write!(
                f,
                "invalid action duration, '{}' should have taken {}ms but took {}ms",
                action,
                expected_duration.as_millis(),
                found_duration.as_millis()
            ),
            Behaviour::InvalidCoderId { max_id, found } => {
                write!(f, "invalid coder id, max id is {max_id} but found {found}")
            }
            Behaviour::InvalidActionOrder {
                last_coder_action,
                current_coder_action,
            } => write!(
                f,
                "invalid action order, last action was '{}' but current is '{}'",
                if let Some(action) = last_coder_action {
                    action.to_string()
                } else {
                    "none".to_string()
                },
                current_coder_action
            ),
            Behaviour::InvalidDongleTaking(DongleTakingError::UnavailableDongle) => {
                write!(f, "coder took a dongle while it's unavailable")
            }
            Behaviour::InvalidDongleTaking(DongleTakingError::TooManyDongles) => {
                write!(f, "coder tried to take more than 2 dongles")
            }
            Behaviour::InvalidDongleTaking(DongleTakingError::CoderCouldHaveTakenDongle {
                coder_id,
                timestamp,
            }) => write!(
                f,
                "coder_{coder_id} should have a taken dongle at {} since both were available",
                timestamp.as_millis()
            ),
            Behaviour::InvalidCompilation(CompilationError::MissingDongles) => {
                write!(f, "coder tried to compile while having less than 2 dongles")
            }
            Behaviour::InvalidCompilation(CompilationError::ExceedingMaxCompiles) => {
                write!(
                    f,
                    "coder tried to compile even after everyone reached compiles required"
                )
            }
            Behaviour::InvalidCompilation(CompilationError::NotEnoughCompiles) => {
                write!(f, "one or more coders still need to compile")
            }
            Behaviour::InvalidCompilation(CompilationError::CoderCouldHaveCompiled {
                coder_id,
                timestamp,
            }) => write!(
                f,
                "coder_{coder_id} should have compiled at {} since he had 2 dongles",
                timestamp.as_millis()
            ),
            Behaviour::InvalidBurnout(BurnoutError::BurnoutNotDetected {
                coder_id,
                timestamp,
            }) => write!(
                f,
                "coder_{coder_id} should have burned out at {}ms",
                timestamp.as_millis()
            ),
            Behaviour::InvalidBurnout(BurnoutError::ShouldNotBurnout {
                coder_id,
                burnout_until,
            }) => write!(
                f,
                "coder_{coder_id} should not have burned out until {}ms",
                burnout_until.as_millis()
            ),
            Behaviour::InvalidBurnout(BurnoutError::BurnoutAlreadyReached) => {
                write!(f, "nothing should print after burnout")
            }
        }
    }
}
