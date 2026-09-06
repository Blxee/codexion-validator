use std::{
    collections::{BTreeMap, HashMap},
    io::stdout,
    sync::mpsc::Receiver,
    time::Duration,
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    terminal,
};
use ratatui::{
    macros::constraints,
    style::Stylize,
    text::Line,
    widgets::{
        Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState, Widget,
    },
};

use crate::protocol::{TestMessage, TestResult};

pub struct UserInterface {
    receiver: Receiver<TestMessage>,
    tests: BTreeMap<usize, Test>,
    current_behaviour_test: Option<Vec<()>>,
    test_scroll: usize,
    shutdown: bool,
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
            test_scroll: 0,
            shutdown: false,
        }
    }

    pub fn render(&mut self) {
        crossterm::execute!(stdout(), EnableMouseCapture).unwrap();
        ratatui::run(|terminal| {
            loop {
                let (_, height) = terminal::size().unwrap();
                let height = height as usize - 3;

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
                            TestState::Running => [
                                Cell::from(id.to_string()),
                                Cell::from("...".bold().yellow()),
                                Cell::from("running"),
                            ],
                            TestState::Failed { args, reason } => [
                                Cell::from(id.to_string()),
                                Cell::from("[KO]".bold().red()),
                                Cell::from(format!("failed when {args} due to {reason}")),
                            ],
                            TestState::Succeeded => [
                                Cell::from(id.to_string()),
                                Cell::from("[OK]".bold().green()),
                                Cell::from("won the game"),
                            ],
                        }));
                    }

                    let mut scrollbar_state =
                        ScrollbarState::new(self.tests.len().saturating_sub(height))
                            .position(self.test_scroll.saturating_sub(height));
                    let scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

                    let mut table_state = TableState::default();
                    table_state.select(Some(self.test_scroll));
                    let tests_table = Table::new(rows, constraints![==3, ==4, >=5])
                        .block(Block::bordered())
                        .highlight_symbol(">>");

                    frame.render_stateful_widget(tests_table, frame.area(), &mut table_state);
                    frame.render_stateful_widget(scroll, frame.area(), &mut scrollbar_state);
                });

                self.handle_input();
                if self.shutdown {
                    break;
                }
            }
        });
        crossterm::execute!(stdout(), DisableMouseCapture).unwrap();
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

    fn handle_input(&mut self) {
        let scroll_up = || self.test_scroll.saturating_sub(1);

        let scroll_down = || (self.test_scroll + 1).min(self.tests.len() - 1);

        if let Ok(true) = event::poll(Duration::from_millis(10)) {
            match event::read() {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.test_scroll = scroll_up();
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.test_scroll = scroll_down();
                    }
                    KeyCode::Char('q') => {
                        self.shutdown = true;
                    }
                    _ => (),
                },
                Ok(Event::Mouse(mouse)) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        self.test_scroll = scroll_up();
                    }
                    MouseEventKind::ScrollDown => {
                        self.test_scroll = scroll_down();
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }
}
