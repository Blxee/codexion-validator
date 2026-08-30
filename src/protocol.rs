pub enum TestMessage {
    TestStarted {
        test_id: usize,
        description: String,
    },
    TestFailed {
        test_id: usize,
        args: String,
        kind: FailureKind,
    },
    TestSucceeded(usize),
}

pub enum FailureKind {
    SegmentationFault,
    ParsingShouldPass,
    ParsingShouldFail,
}
