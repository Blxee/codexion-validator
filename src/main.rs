mod args;
mod behaviour;
mod parsing;
use std::{
    arch::asm,
    env::args,
    error::Error,
    process::Command,
    time::{Duration, Instant},
};

use crate::{args::RawArgs, behaviour::CodexionState, parsing::CodexionOutput};

fn main() {
    let args = args().collect::<Vec<_>>();

    let [_, program_path] = args.as_slice() else {
        return eprintln!("Error: wrong argument count");
    };

    let codexion_args = RawArgs {
        number_of_coders: "3",
        time_to_burnout: "3000",
        time_to_compile: "1000",
        time_to_debug: "500",
        time_to_refactor: "500",
        number_of_compiles_required: "2",
        dongle_cooldown: "200",
        scheduler: "fifo",
    };

    let program_output = run_command(program_path, codexion_args, Duration::from_secs(1)).unwrap();
    println!("output: {}", &program_output.stdout);

    let codexion_output: CodexionOutput = match (&program_output).try_into() {
        Ok(res) => res,
        Err(err) => return println!("{err}"),
    };

    let mut state =
        CodexionState::from(codexion_args.try_into().unwrap(), Duration::from_millis(10));

    for event in codexion_output.events {
        state.update(event).unwrap();
    }
}

fn run_command(
    program: &str,
    args: RawArgs,
    timeout: Duration,
) -> Result<ProgramOutput, Box<dyn Error>> {
    let output = Command::new(program).args(args.to_vec()).output()?;

    Ok(ProgramOutput {
        stdout: String::from_utf8(output.stdout)?,
        stderr: String::from_utf8(output.stderr)?,
        status: output.status.code().unwrap(),
        duration: Duration::from_secs(1),
    })
}

#[derive(Debug)]
struct ProgramOutput {
    stdout: String,
    stderr: String,
    status: i32,
    duration: Duration,
}
