use std::{collections::HashMap, sync::mpsc::Receiver, time::Duration};

use crossterm::event::{self, KeyCode};
use ratatui::{
    macros::constraints,
    text::Line,
    widgets::{Block, Row, Table, Widget},
};

use crate::protocol::TestMessage;

pub struct UserInterface {
    receiver: Receiver<TestMessage>,
    tests: HashMap<usize, TestMessage>,
    current_behaviour_test: Option<Vec<()>>,
}

impl UserInterface {
    pub fn new(receiver: Receiver<TestMessage>) -> Self {
        Self {
            receiver,
            tests: HashMap::new(),
            current_behaviour_test: None,
        }
    }

    pub fn render(&mut self) {
        ratatui::run(|terminal| {
            loop {
                let _ = terminal.draw(|frame| {
                    if let Ok(test) = self.receiver.try_recv() {
                        self.tests.insert(test.test_id);
                    }

                    let mut rows = Vec::new();

                    for test in &self.tests {
                        rows.push(match test {
                            TestMessage::TestStarted {
                                test_id,
                                description,
                            } => Row::new([test_id.to_string()]),
                            TestMessage::TestFailed {
                                test_id,
                                args,
                                kind,
                            } => Row::new([test_id.to_string()]),
                            TestMessage::TestSucceeded(test_id) => Row::new([test_id.to_string()]),
                        });
                    }

                    let table = Table::new(rows, constraints![==100%]).block(Block::bordered());
                    frame.render_widget(table, frame.area());
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
}
