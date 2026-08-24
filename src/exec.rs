use std::{
    io::BufReader,
    os::unix::process::ExitStatusExt,
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::args::RawArgs;

pub struct CodexionInstance {
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<BufReader<ChildStderr>>,
    process_result: Option<ProcessResult>,
    child_monitor: Option<JoinHandle<ProcessResult>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ProcessResult {
    Success,
    ExitFailure(i32),
    SegmentationFault,
    Signaled(i32),
    TimedOut,
}

impl<'a> CodexionInstance {
    // create and immediately start a codexion instance
    pub fn new(program: &'a str, args: RawArgs<'a>, timeout: Option<Duration>) -> Self {
        let mut child_process = Command::new(program)
            .args(args.to_vec())
            .stdout(Stdio::piped()) // pipe stdout and stderr to capture them
            .stderr(Stdio::piped())
            .spawn()
            .expect("Error running codexion program");

        let stdout = child_process.stdout.take().map(BufReader::new);
        let stderr = child_process.stderr.take().map(BufReader::new);

        let child_monitor = Some(thread::spawn(move || {
            let span = Duration::from_millis(10);

            if let Some(mut duration) = timeout {
                loop {
                    thread::sleep(span);
                    duration = duration.saturating_sub(span);
                    // if the child exited, stop waiting
                    if child_process.try_wait().unwrap().is_some() {
                        break;
                    }
                    // if the timeout was reached, kill the child
                    if duration.is_zero() {
                        let _ = child_process.kill();
                        return ProcessResult::TimedOut;
                    }
                }
            }
            // reap the exit status of the child if any
            let exit_status = child_process.wait().unwrap();
            match (exit_status.code(), exit_status.signal()) {
                // if there was a seg fault
                (_, Some(11)) => ProcessResult::SegmentationFault,
                // if process terminated with a signal
                (_, Some(sig)) => ProcessResult::Signaled(sig),
                // if process exited with success
                (Some(0), None) => ProcessResult::Success,
                // if process exited with failure
                (Some(status), None) => ProcessResult::ExitFailure(status),
                _ => ProcessResult::Success,
            }
        }));

        Self {
            stdout,
            stderr,
            process_result: None,
            child_monitor,
        }
    }

    pub fn stdout(&mut self) -> Option<BufReader<ChildStdout>> {
        self.stdout.take()
    }

    pub fn stderr(&mut self) -> Option<BufReader<ChildStderr>> {
        self.stderr.take()
    }

    // wait for the child to terminate/timeout and return the exit status if any
    pub fn exit_code(&mut self) -> ProcessResult {
        // get the exit status from the child monitoring thread
        if let Some(child) = self.child_monitor.take() {
            self.process_result = Some(child.join().unwrap());
        }
        self.process_result.unwrap()
    }
}

impl Drop for CodexionInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.child_monitor.take() {
            let _ = handle.join();
        }
    }
}
