mod args;
mod behaviour;
mod exec;
mod parsing;
mod protocol;
mod test_suit;
mod tui;
use std::{env::args, io::BufRead, sync::mpsc, thread, time::Duration};

use ratatui::{Frame, text::Line, widgets::Widget};

use crate::{
    args::RawArgs, behaviour::CodexionState, exec::CodexionInstance, parsing::Event,
    test_suit::TestSuit, tui::UserInterface,
};

fn main() {
    let Some(program_path) = args().nth(1) else {
        return eprintln!("Error: wrong argument count");
    };

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut test = TestSuit::new(&program_path, sender);
        test.start();
    });
    let mut ui = UserInterface::new(receiver);
    ui.render();
}
