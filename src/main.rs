mod args;
mod behaviour;
mod exec;
mod parsing;
mod protocol;
mod test_suit;
mod tui;
use std::{io::BufRead, sync::mpsc, thread, time::Duration};

use ratatui::{Frame, text::Line, widgets::Widget};

use crate::{
    args::RawArgs, behaviour::CodexionState, exec::CodexionInstance, parsing::Event,
    test_suit::TestSuit, tui::UserInterface,
};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    program_path: String,
}

fn main() {
    let args = Args::parse();

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut test = TestSuit::new(&args.program_path, sender);
        test.start();
    });
    let mut ui = UserInterface::new(receiver);
    ui.render();
}
