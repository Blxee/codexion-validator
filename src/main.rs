mod args;
mod behaviour;
mod exec;
mod parsing;
use std::{env::args, io::BufRead, time::Duration};

use crate::{args::RawArgs, behaviour::CodexionState, exec::CodexionInstance, parsing::Event};

fn main() {
    let args = args().collect::<Vec<_>>();

    let [_, program_path] = args.as_slice() else {
        return eprintln!("Error: wrong argument count");
    };

    let codexion_args = RawArgs {
        number_of_coders: "10",
        time_to_burnout: "200",
        time_to_compile: "50",
        time_to_debug: "50",
        time_to_refactor: "50",
        number_of_compiles_required: "20",
        dongle_cooldown: "20",
        scheduler: "fifo",
    };

    let mut instance =
        CodexionInstance::new(program_path, codexion_args, Some(Duration::from_secs(2)));
    let mut state =
        CodexionState::from(codexion_args.try_into().unwrap(), Duration::from_millis(10));

    for line in instance.stdout().unwrap().lines() {
        let line = line.unwrap();
        println!("{}", line);
        let event = Event::try_from(line.as_str()).unwrap();
        match state.update(event) {
            Ok(_) => (),
            Err(err) => println!("{err}"),
        }
    }

    println!("exit code: {:?}", instance.exit_code());
}
