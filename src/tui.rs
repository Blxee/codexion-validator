use std::sync::mpsc::Receiver;

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
                    if let TestMessage::Msg(msg) = self.receiver.recv().unwrap() {
                        let line = Line::from(msg);
                        frame.render_widget(line, frame.area());
                    }
                });
            }
        });
    }
}
