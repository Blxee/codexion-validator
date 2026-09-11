use std::fmt::Display;

pub struct TestMessage {
    pub test_id: usize,
    pub result: TestResult,
}

pub enum TestResult {
    TestStarted { args: String, description: String },
    ProgressLine { fd: FileDescriptor, line: String },
    TestFailed { failure_kind: FailureKind },
    TestSucceeded,
}

pub enum FileDescriptor {
    Stdout,
    Stderr,
}

pub enum FailureKind {
    SegmentationFault,
    ParsingShouldPass,
    ParsingShouldFail,
}

impl Display for FailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FailureKind::SegmentationFault => "Segmentation fault",
                FailureKind::ParsingShouldPass => "Parsing should have passed",
                FailureKind::ParsingShouldFail => "Parsing should have failed",
            }
        )
    }
}
