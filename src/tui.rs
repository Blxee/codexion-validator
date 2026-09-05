use std::{sync::mpsc::Receiver, time::Duration};

use crossterm::event::{self, KeyCode};
use ratatui::{text::Line, widgets::Widget};

use crate::protocol::TestMessage;

pub struct UserInterface {
    receiver: Receiver<TestMessage>,
    test_results: Vec<TestMessage>,
    current_behaviour_test: Option<Vec<()>>,
}

impl UserInterface {
    pub fn new(receiver: Receiver<TestMessage>) -> Self {
        Self {
            receiver,
            test_results: Vec::new(),
            current_behaviour_test: None,
        }
    }

    pub fn render(&mut self) {
        ratatui::run(|terminal| {
            loop {
                terminal.draw(|frame| {
                    if let TestMessage::TestStarted {
                        test_id,
                        description,
                    } = self.receiver.recv().unwrap()
                    {
                        let line = Line::from(description);
                        frame.render_widget(line, frame.area());
                    }
                });
                if let Ok(event::Event::Key(key)) = event::read() {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }
        });
    }
}
