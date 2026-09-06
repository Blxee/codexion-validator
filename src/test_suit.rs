use std::{
    io::{BufRead, BufReader, Read},
    process::{ChildStderr, ChildStdout},
    sync::mpsc::Sender,
    thread::sleep,
    time::Duration,
};

use crate::{
    exec::{CodexionInstance, ProcessResult},
    protocol::{
        FailureKind::{self, ParsingShouldFail, ParsingShouldPass, SegmentationFault},
        TestMessage, TestResult,
    },
};

pub struct TestSuit<'a> {
    program_path: &'a str,
    sender: Sender<TestMessage>,
}

fn expect_normal(
    (stdout, mut stderr, exit_status): (
        BufReader<ChildStdout>,
        BufReader<ChildStderr>,
        ProcessResult,
    ),
) -> bool {
    let exit_code_was_success = matches!(exit_status, ProcessResult::Success);
    let output_printed = stdout.lines().count() > 1;
    let mut buf = String::new();
    stderr.read_to_string(&mut buf);
    let nothing_in_stderr = buf.is_empty();

    exit_code_was_success && output_printed && nothing_in_stderr
    // add some behaviour shet
}

fn expect_error(
    (mut stdout, stderr, exit_status): (
        BufReader<ChildStdout>,
        BufReader<ChildStderr>,
        ProcessResult,
    ),
) -> bool {
    let exit_code_was_failure = matches!(exit_status, ProcessResult::ExitFailure(_));
    let error_printed = stderr.lines().count() >= 1;
    let mut buf = String::new();
    stdout.read_to_string(&mut buf);
    let nothing_in_stdout = buf.is_empty();

    exit_code_was_failure && error_printed && nothing_in_stdout
}

fn expect_no_crash(
    (_stdout, _stderr, exit_status): (
        BufReader<ChildStdout>,
        BufReader<ChildStderr>,
        ProcessResult,
    ),
) -> bool {
    !matches!(exit_status, ProcessResult::SegmentationFault)
}

impl<'a> TestSuit<'a> {
    pub fn new(program_path: &'a str, sender: Sender<TestMessage>) -> Self {
        Self {
            program_path,
            sender,
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
        self.test_parsing_number_of_coders(1);
        self.test_parsing_time_to_burnout(2);
        self.test_parsing_time_to_compile(3);
        self.test_parsing_time_to_debug(4);
        self.test_parsing_time_to_refactor(5);
        self.test_parsing_number_of_compiles_required(6);
        self.test_parsing_dongle_cooldown(7);
        // self.test_parsing_dongle_scheduler(0);
    }

    fn test_parsing_numeric_argument(&self, test_id: usize, args_template: String) {
        self.sender
            .send(TestMessage {
                test_id,
                result: TestResult::TestStarted {
                    description: "testing parsing number of coders".into(),
                },
            })
            .unwrap();

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

            let excution_result = CodexionInstance::excute(
                self.program_path,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(1)),
            );

            if !expect_error(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id,
                        result: TestResult::TestFailed {
                            args,
                            failure_kind: ParsingShouldFail,
                        },
                    })
                    .unwrap();
                return;
            }
        }

        for arg in NORMAL_NUMERIC_ARGS {
            let args = args_template.replace("{}", arg);

            let excution_result = CodexionInstance::excute(
                self.program_path,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(10)),
            );

            if !expect_normal(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id,
                        result: TestResult::TestFailed {
                            args,
                            failure_kind: ParsingShouldPass,
                        },
                    })
                    .unwrap();
                return;
            }
        }

        for arg in NO_CRASH_NUMERIC_ARGS {
            let args = args_template.replace("{}", arg);

            let excution_result = CodexionInstance::excute(
                self.program_path,
                args.as_str().try_into().unwrap(),
                Some(Duration::from_secs(10)),
            );

            if !expect_no_crash(excution_result) {
                self.sender
                    .send(TestMessage {
                        test_id,
                        result: TestResult::TestFailed {
                            args,
                            failure_kind: SegmentationFault,
                        },
                    })
                    .unwrap();
                return;
            }
        }

        self.sender
            .send(TestMessage {
                test_id,
                result: TestResult::TestSucceeded,
            })
            .unwrap();
    }

    fn test_parsing_number_of_coders(&self, test_id: usize) {
        let args_template = "{} 1000 500 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template);
    }

    fn test_parsing_time_to_burnout(&self, test_id: usize) {
        let args_template = "4 {} 500 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }

    fn test_parsing_time_to_compile(&self, test_id: usize) {
        let args_template = "4 1000 {} 300 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }

    fn test_parsing_time_to_debug(&self, test_id: usize) {
        let args_template = "4 1000 500 {} 300 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }

    fn test_parsing_time_to_refactor(&self, test_id: usize) {
        let args_template = "4 1000 500 300 {} 4 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }

    fn test_parsing_number_of_compiles_required(&self, test_id: usize) {
        let args_template = "4 1000 500 300 300 {} 200 fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }

    fn test_parsing_dongle_cooldown(&self, test_id: usize) {
        let args_template = "4 1000 500 300 300 4 {} fifo".to_string();
        self.test_parsing_numeric_argument(test_id, args_template)
    }
}
