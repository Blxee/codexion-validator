pub struct TestMessage {
    test_id: usize,
    result: TestResult,
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
