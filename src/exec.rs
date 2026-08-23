use std::{
    io::BufReader,
    process::{Child, ChildStderr, ChildStdout, Command, Stdio},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::args::RawArgs;

pub struct CodexionInstance {
    stdout: Option<BufReader<ChildStdout>>,
    stderr: Option<BufReader<ChildStderr>>,
    exit_code: Option<i32>,
    child_monitor: Option<JoinHandle<Option<i32>>>,
}

impl<'a> CodexionInstance {
    pub fn new(program: &'a str, args: RawArgs<'a>, timeout: Option<Duration>) -> Self {
        let mut child_process = Command::new(program)
            .args(args.to_vec())
            .stdout(Stdio::piped())
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
                        break;
                    }
                }
            }
            // reap the exit status of the child if any
            child_process.wait().unwrap().code()
        }));

        Self {
            stdout,
            stderr,
            exit_code: None,
            child_monitor,
        }
    }

    pub fn stdout(&mut self) -> Option<BufReader<ChildStdout>> {
        self.stdout.take()
    }

    pub fn stderr(&mut self) -> Option<BufReader<ChildStderr>> {
        self.stderr.take()
    }

    pub fn exit_code(&mut self) -> Option<i32> {
        // get the status code from the child monitoring thread
        if self.child_monitor.is_some() {
            self.exit_code = self
                .child_monitor
                .take()
                .and_then(|handle| handle.join().unwrap())
        }
        self.exit_code
    }
}

impl Drop for CodexionInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.child_monitor.take() {
            let _ = handle.join();
        }
    }
}
