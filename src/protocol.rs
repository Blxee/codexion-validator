pub struct TestMessage {
    pub test_id: usize,
    pub result: TestResult,
}

pub enum TestResult {
    TestStarted {
        description: String,
    },
    TestFailed {
        args: String,
        failure_kind: FailureKind,
    },
    TestSucceeded,
}

pub enum FailureKind {
    SegmentationFault,
    ParsingShouldPass,
    ParsingShouldFail,
}
