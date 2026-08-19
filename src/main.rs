mod parsing;
use std::{
    arch::asm,
    env::args,
    error::Error,
    process::Command,
    time::{Duration, Instant},
};

use crate::parsing::CodexionOutput;

fn main() {
    let args = args().collect::<Vec<_>>();

    let [_, program_path] = args.as_slice() else {
        return eprintln!("Error: wrong argument count");
    };

    let codexion_args = Args {
        number_of_coders: 3,
        time_to_burnout: 3000,
        time_to_compile: 1000,
        time_to_debug: 500,
        time_to_refactor: 500,
        number_of_compiles_required: 2,
        dongle_cooldown: 200,
        scheduler: Scheduler::FIFO,
    };

    let program_output = run_command(program_path, codexion_args, Duration::from_secs(1)).unwrap();
    let i: CodexionOutput = (&(codexion_args, program_output)).try_into().unwrap();
    dbg!(i);
}

fn run_command(
    program: &str,
    args: Args,
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

#[derive(Clone, Copy, Debug)]
pub struct Args {
    number_of_coders: i32,
    time_to_burnout: i32,
    time_to_compile: i32,
    time_to_debug: i32,
    time_to_refactor: i32,
    number_of_compiles_required: i32,
    dongle_cooldown: i32,
    scheduler: Scheduler,
}

#[derive(Debug, Clone, Copy)]
pub enum Scheduler {
    FIFO,
    EDF,
}

impl Args {
    fn to_vec(&self) -> Vec<String> {
        vec![
            self.number_of_coders.to_string(),
            self.time_to_burnout.to_string(),
            self.time_to_compile.to_string(),
            self.time_to_debug.to_string(),
            self.time_to_refactor.to_string(),
            self.number_of_compiles_required.to_string(),
            self.dongle_cooldown.to_string(),
            match self.scheduler {
                Scheduler::FIFO => String::from("fifo"),
                Scheduler::EDF => String::from("edf"),
            },
        ]
    }
}
