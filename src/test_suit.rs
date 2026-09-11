use std::{
    io::{BufRead, BufReader, Read},
    process::{ChildStderr, ChildStdout},
    sync::mpsc::Sender,
    thread::sleep,
    time::Duration,
};

use crate::{
    args::RawArgs,
    exec::{CodexionInstance, ProcessResult},
    protocol::{
        FailureKind::{self, ParsingShouldFail, ParsingShouldPass, SegmentationFault},
        FileDescriptor, TestMessage, TestResult,
    },
};

pub struct TestSuit<'a> {
    program_path: &'a str,
    sender: Sender<TestMessage>,
    current_test_id: usize,
}

fn expect_normal((stdout, stderr, exit_status): (String, String, ProcessResult)) -> bool {
    let exit_code_was_success = matches!(exit_status, ProcessResult::Success);
    let output_printed = stdout.lines().count() >= 1;
    let nothing_in_stderr = stderr.is_empty();

    exit_code_was_success && output_printed && nothing_in_stderr
    // add some behaviour shet
}

fn expect_error((stdout, stderr, exit_status): (String, String, ProcessResult)) -> bool {
    let exit_code_was_failure = matches!(exit_status, ProcessResult::ExitFailure(_));
    let error_printed = stderr.lines().count() >= 1;
    let nothing_in_stdout = stdout.is_empty();

    exit_code_was_failure && error_printed && nothing_in_stdout
}

fn expect_no_crash((_stdout, _stderr, exit_status): (String, String, ProcessResult)) -> bool {
    !matches!(exit_status, ProcessResult::SegmentationFault)
}

impl<'a> TestSuit<'a> {
    pub fn new(program_path: &'a str, sender: Sender<TestMessage>) -> Self {
        Self {
            program_path,
            sender,
            current_test_id: 0,
        }
    }

    pub fn start(&mut self) {
        // for i in 0..100 {
        //     self.sender
        //         .send(TestMessage {
        //             test_id: i,
        //             result: TestResult::TestStarted {
        //                 description: "testing parsing number of coders".into(),
        //             },
        //         })
        //         .unwrap();
        //     sleep(Duration::from_millis(100));
        // }
        // for i in 0..100 {
        //     if i % 2 == 0 {
        //         self.sender
        //             .send(TestMessage {
        //                 test_id: i,
        //                 result: TestResult::TestSucceeded,
        //             })
        //             .unwrap();
        //     } else {
        //         self.sender
        //             .send(TestMessage {
        //                 test_id: i,
        //                 result: TestResult::TestFailed {
        //                     args: "38 38 59 fifo".to_owned(),
        //                     failure_kind: FailureKind::SegmentationFault,
        //                 },
        //             })
        //             .unwrap();
        //     }
        //     sleep(Duration::from_millis(100));
        // }
        // test no args
        // test extra args
        self.test_parsing_number_of_coders();
        self.test_parsing_time_to_burnout();
        self.test_parsing_time_to_compile();
        self.test_parsing_time_to_debug();
        self.test_parsing_time_to_refactor();
        self.test_parsing_number_of_compiles_required();
        self.test_parsing_dongle_cooldown();
        // self.test_parsing_dongle_scheduler(0);
    }

    fn test_parsing_numeric_argument(&mut self, args_template: String) {
        const ERROR_NUMERIC_ARGS: [&'static str; 23] = [
            "-4294967296",
            "-2147483648",
            "2147483648",
            "21474836472147483647",
            "4294967295",
            "-1",
            "-10",
            "340282366920938463463374607431768211455",
            "-340282366920938463463374607431768211455",
            "abc",
            "ABC",
            "#$%",
            "--1",
            "++1",
            "--abc",
            "++abc",
            "----",
            "++++",
            "1++1",
            "1--1",
            "+",
            "-",
            "\"\"",
        ];

        const NORMAL_NUMERIC_ARGS: [&'static str; 4] = ["10", "3", "1", "10"];

        const NO_CRASH_NUMERIC_ARGS: [&'static str; 7] =
            ["2147483647", "0", "+0", "-0", "+10", "001", "000"];

        for arg in ERROR_NUMERIC_ARGS {
            let args = args_template.replace("{}", arg);

            self.current_test_id += 1;

            let excution_result = self.excute(
                self.current_test_id,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(1)),
            );

            if !expect_error(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestFailed {
                            failure_kind: ParsingShouldFail,
                        },
                    })
                    .unwrap();
                return;
            } else {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestSucceeded,
                    })
                    .unwrap();
            }
        }

        for arg in NORMAL_NUMERIC_ARGS {
            let args = args_template.replace("{}", arg);
            self.current_test_id += 1;

            let excution_result = self.excute(
                self.current_test_id,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(10)),
            );

            if !expect_normal(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestFailed {
                            failure_kind: ParsingShouldPass,
                        },
                    })
                    .unwrap();
                return;
            } else {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestSucceeded,
                    })
                    .unwrap();
            }
        }

        for arg in NO_CRASH_NUMERIC_ARGS {
            let args = args_template.replace("{}", arg);
            self.current_test_id += 1;

            let excution_result = self.excute(
                self.current_test_id,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(10)),
            );

            if !expect_no_crash(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestFailed {
                            failure_kind: SegmentationFault,
                        },
                    })
                    .unwrap();
                return;
            } else {
                self.sender
                    .send(TestMessage {
                        test_id: self.current_test_id,
                        result: TestResult::TestSucceeded,
                    })
                    .unwrap();
            }
        }
    }

    fn excute(
        &self,
        test_id: usize,
        args: RawArgs<'a>,
        timeout: Option<Duration>,
    ) -> (String, String, ProcessResult) {
        self.sender
            .send(TestMessage {
                test_id,
                result: TestResult::TestStarted {
                    description: "testing parsing number of coders".into(),
                    args: args.into(),
                },
            })
            .unwrap();

        let mut instance = CodexionInstance::new(self.program_path, args, timeout);

        let mut stderr = String::new();
        for line in instance.stderr().unwrap().lines() {
            let line = line.unwrap();
            stderr.push_str(&line);
            stderr.push('\n');
            self.sender
                .send(TestMessage {
                    test_id,
                    result: TestResult::ProgressLine {
                        fd: FileDescriptor::Stderr,
                        line,
                    },
                })
                .unwrap();
        }

        let mut stdout = String::new();
        for line in instance.stdout().unwrap().lines() {
            let line = line.unwrap();
            stdout.push_str(&line);
            stdout.push('\n');
            self.sender
                .send(TestMessage {
                    test_id,
                    result: TestResult::ProgressLine {
                        fd: FileDescriptor::Stdout,
                        line,
                    },
                })
                .unwrap();
        }

        (stdout, stderr, instance.exit_status())
    }

    fn test_parsing_number_of_coders(&mut self) {
        let args_template = "{} 1000 500 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template);
    }

    fn test_parsing_time_to_burnout(&mut self) {
        let args_template = "4 {} 500 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }

    fn test_parsing_time_to_compile(&mut self) {
        let args_template = "4 1000 {} 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }

    fn test_parsing_time_to_debug(&mut self) {
        let args_template = "4 1000 500 {} 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }

    fn test_parsing_time_to_refactor(&mut self) {
        let args_template = "4 1000 500 300 {} 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }

    fn test_parsing_number_of_compiles_required(&mut self) {
        let args_template = "4 1000 500 300 300 {} 200 fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }

    fn test_parsing_dongle_cooldown(&mut self) {
        let args_template = "4 1000 500 300 300 4 {} fifo".to_string();
        self.test_parsing_numeric_argument(args_template)
    }
}
