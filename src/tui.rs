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
    layout::Layout,
    macros::constraints,
    style::Stylize,
    text::{Line, Text, ToText},
    widgets::{
        Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState, Widget,
    },
};

use crate::protocol::{FileDescriptor, TestMessage, TestResult};

pub struct UserInterface {
    receiver: Receiver<TestMessage>,
    tests: BTreeMap<usize, Test>,
    test_scroll: usize,
    output_scroll: usize,
    is_scrolling_output: bool,
    shutdown: bool,
}

struct Test {
    id: usize,
    args: String,
    description: String,
    state: TestState,
    stdout: Vec<String>,
    stderr: Vec<String>,
}

enum TestState {
    Running,
    Failed { reason: String },
    Succeeded,
}

impl UserInterface {
    pub fn new(receiver: Receiver<TestMessage>) -> Self {
        Self {
            receiver,
            tests: BTreeMap::new(),
            test_scroll: 0,
            output_scroll: 0,
            is_scrolling_output: false,
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

                    let chunks = Layout::horizontal(if self.is_scrolling_output {
                        constraints![==30%, ==70%]
                    } else {
                        constraints![==70%, ==30%]
                    })
                    .split(frame.area());
                    let test_result_layout = chunks[0];
                    let chunks = Layout::vertical(constraints![==70%, ==30%]).split(chunks[1]);
                    let stdout_layout = chunks[0];
                    let stderr_layout = chunks[1];

                    let mut rows = Vec::new();

                    for Test {
                        id,
                        args,
                        description,
                        state,
                        ..
                    } in self.tests.values()
                    {
                        rows.push(
                            match state {
                                TestState::Running => Row::new([
                                    Cell::from(id.to_string()),
                                    Cell::from("[..]".bold().yellow()),
                                    Cell::from(Text::from(vec![
                                        Line::from(vec![
                                            "description".bold().underlined(),
                                            ": ".into(),
                                            description.into(),
                                        ]),
                                        Line::from(vec![
                                            "arguments".bold().underlined(),
                                            ": ".into(),
                                            args.into(),
                                        ]),
                                    ])),
                                ]),
                                TestState::Failed { reason } => Row::new([
                                    Cell::from(id.to_string()),
                                    Cell::from("[KO]".bold().red()),
                                    Cell::from(Text::from(vec![
                                        Line::from(vec![
                                            "description".bold().underlined(),
                                            ": ".into(),
                                            description.into(),
                                        ]),
                                        Line::from(vec![
                                            "arguments".bold().underlined(),
                                            ": ".into(),
                                            args.into(),
                                        ]),
                                        Line::from(vec![
                                            "failure_reason".bold().underlined(),
                                            ": ".into(),
                                            reason.into(),
                                        ]),
                                    ])),
                                ])
                                .on_red(),
                                TestState::Succeeded => Row::new([
                                    Cell::from(id.to_string()),
                                    Cell::from("[OK]".bold().green()),
                                    Cell::from(Text::from(vec![
                                        Line::from(vec![
                                            "description".bold().underlined(),
                                            ": ".into(),
                                            description.into(),
                                        ]),
                                        Line::from(vec![
                                            "arguments".bold().underlined(),
                                            ": ".into(),
                                            args.into(),
                                        ]),
                                    ])),
                                ]),
                            }
                            .height(3)
                            .bottom_margin(1),
                        );
                    }

                    let mut scrollbar_state =
                        ScrollbarState::new(self.tests.len().saturating_sub(height))
                            .position(self.test_scroll.saturating_sub(height));
                    let scroll = Scrollbar::new(ScrollbarOrientation::VerticalRight);

                    let mut table_state = TableState::default();
                    table_state.select(Some(self.test_scroll));
                    let mut tests_table = Table::new(rows, constraints![==3, ==8, >=5])
                        .header(Row::new(["id", "result", "info"]).bold().underlined())
                        .highlight_symbol("=> ".cyan());

                    let test = self.tests.values().nth(self.test_scroll);

                    let mut stdout_text = match test {
                        Some(test) => {
                            let rows = test.stdout.iter().map(|s| Row::new([s.as_str()]));
                            Table::new(rows, constraints![==100%])
                        }
                        None => Table::new([Row::new([""])], constraints![==100%]),
                    }
                    .highlight_symbol("=> ".cyan());
                    if self.is_scrolling_output {
                        stdout_text =
                            stdout_text.block(Block::bordered().white().title_top("stdout"));
                        tests_table =
                            tests_table.block(Block::bordered().dark_gray().title_top("stdout"));
                    } else {
                        stdout_text =
                            stdout_text.block(Block::bordered().dark_gray().title_top("stdout"));
                        tests_table =
                            tests_table.block(Block::bordered().white().title_top("stdout"));
                    }
                    let mut stdout_state = TableState::default();
                    stdout_state.select(Some(self.output_scroll));

                    let stderr_text = match test {
                        Some(test) => Text::from_iter(test.stderr.iter().map(|s| s.as_str())),
                        None => Text::from("N/a"),
                    };
                    let stderr_par = Paragraph::new(stderr_text);

                    if self.test_scroll > self.tests.len().saturating_sub(1) {
                        self.test_scroll = self.tests.len().saturating_sub(1);
                    }
                    let output_len = test.map_or(0, |test| test.stdout.len());
                    if self.output_scroll > output_len.saturating_sub(1) {
                        self.output_scroll = output_len.saturating_sub(1);
                    }

                    frame.render_stateful_widget(tests_table, test_result_layout, &mut table_state);
                    frame.render_stateful_widget(scroll, test_result_layout, &mut scrollbar_state);
                    frame.render_stateful_widget(stdout_text, stdout_layout, &mut stdout_state);
                    frame.render_widget(
                        stderr_par.block(Block::bordered().dark_gray().title_top("stderr")),
                        stderr_layout,
                    );
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
            let test = self.tests.get_mut(&id);
            match result {
                TestResult::TestStarted { description, args } => {
                    self.tests.insert(
                        id,
                        Test {
                            id,
                            description,
                            args,
                            state: TestState::Running,
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        },
                    );
                }
                TestResult::TestFailed { failure_kind } => {
                    test.unwrap().state = TestState::Failed {
                        reason: failure_kind.to_string(),
                    }
                }
                TestResult::TestSucceeded => test.unwrap().state = TestState::Succeeded,
                TestResult::ProgressLine { fd, line } => {
                    match fd {
                        FileDescriptor::Stdout => test.unwrap().stdout.push(line),
                        FileDescriptor::Stderr => test.unwrap().stderr.push(line),
                    }
                    // set the scroll to the current test being updated
                    if !self.is_scrolling_output {
                        self.test_scroll = self.tests.keys().position(|&k| k == id).unwrap();
                    }
                }
            }
        }
    }

    fn handle_input(&mut self) {
        let scroll_test_up = || self.test_scroll.saturating_sub(1);
        let scroll_test_down = || self.test_scroll + 1;

        let scroll_output_up = || self.output_scroll.saturating_sub(1);
        let scroll_output_down = || self.output_scroll + 1;

        if let Ok(true) = event::poll(Duration::from_millis(10)) {
            match event::read() {
                Ok(Event::Key(key)) => match key.code {
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.is_scrolling_output {
                            self.output_scroll = scroll_output_up();
                        } else {
                            self.test_scroll = scroll_test_up();
                            self.output_scroll = 0;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.is_scrolling_output {
                            self.output_scroll = scroll_output_down();
                        } else {
                            self.test_scroll = scroll_test_down();
                            self.output_scroll = 0;
                        }
                    }
                    KeyCode::Char('q') => {
                        self.shutdown = true;
                    }
                    KeyCode::Left | KeyCode::Right => {
                        self.is_scrolling_output = !self.is_scrolling_output;
                    }
                    KeyCode::Enter if !self.is_scrolling_output => {
                        self.is_scrolling_output = true;
                    }
                    KeyCode::Esc if self.is_scrolling_output => {
                        self.is_scrolling_output = false;
                    }
                    _ => (),
                },
                Ok(Event::Mouse(mouse)) => match mouse.kind {
                    MouseEventKind::ScrollUp => {
                        if self.is_scrolling_output {
                            self.output_scroll = scroll_output_up();
                        } else {
                            self.test_scroll = scroll_test_up();
                            self.output_scroll = 0;
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        if self.is_scrolling_output {
                            self.output_scroll = scroll_output_down();
                        } else {
                            self.test_scroll = scroll_test_down();
                            self.output_scroll = 0;
                        }
                    }
                    MouseEventKind::Up(event::MouseButton::Left) => (),
                    _ => (),
                },
                _ => (),
            }
        }
    }
}
