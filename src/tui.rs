use std::{
    collections::{BTreeMap, HashMap},
    sync::mpsc::Receiver,
    time::Duration,
};

use crossterm::event::{self, KeyCode};
use ratatui::{
    macros::constraints,
    text::Line,
    widgets::{Block, Row, Table, Widget},
};

use crate::protocol::{TestMessage, TestResult};

pub struct UserInterface {
    receiver: Receiver<TestMessage>,
    tests: BTreeMap<usize, Test>,
    current_behaviour_test: Option<Vec<()>>,
}

struct Test {
    id: usize,
    description: String,
    state: TestState,
}

enum TestState {
    Running,
    Failed { args: String, reason: String },
    Succeeded,
}

impl UserInterface {
    pub fn new(receiver: Receiver<TestMessage>) -> Self {
        Self {
            receiver,
            tests: BTreeMap::new(),
            current_behaviour_test: None,
        }
    }

    pub fn render(&mut self) {
        ratatui::run(|terminal| {
            loop {
                let _ = terminal.draw(|frame| {
                    self.fetch_test();

                    let mut rows = Vec::new();

                    for Test {
                        id,
                        description,
                        state,
                    } in self.tests.values()
                    {
                        rows.push(Row::new(match state {
                            TestState::Running => [id.to_string(), "running".to_string()],
                            TestState::Failed { args, reason } => [
                                id.to_string(),
                                format!("failed when {args} due to {reason}"),
                            ],
                            TestState::Succeeded => [id.to_string(), "won the game".to_string()],
                        }));
                    }

                    let tests_table =
                        Table::new(rows, constraints![==50%, ==50%]).block(Block::bordered());
                    frame.render_widget(tests_table, frame.area());
                });

                if let Ok(true) = event::poll(Duration::from_millis(10)) {
                    if let Ok(event::Event::Key(key)) = event::read() {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                    }
                }
            }
        });
    }

    fn fetch_test(&mut self) {
        if let Ok(TestMessage {
            test_id: id,
            result,
        }) = self.receiver.try_recv()
        {
            match result {
                TestResult::TestStarted { description } => {
                    self.tests.insert(
                        id,
                        Test {
                            id,
                            description,
                            state: TestState::Running,
                        },
                    );
                }
                TestResult::TestFailed { args, failure_kind } => {
                    self.tests.get_mut(&id).unwrap().state = TestState::Failed {
                        args,
                        reason: failure_kind.to_string(),
                    }
                }
                TestResult::TestSucceeded => {
                    self.tests.get_mut(&id).unwrap().state = TestState::Succeeded
                }
            }
        }
    }
}
