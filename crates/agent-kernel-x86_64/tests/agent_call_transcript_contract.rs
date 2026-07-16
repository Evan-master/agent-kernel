use agent_kernel_x86_64::agent_call::{
    AgentCallOperation, AgentCallTranscript, AgentCallTranscriptError,
};

#[test]
fn transcript_records_role_independent_call_evidence() {
    let mut transcript = AgentCallTranscript::<5>::new();
    let operations = [
        AgentCallOperation::DescribeContext,
        AgentCallOperation::SubmitTaskResult,
        AgentCallOperation::SendMessage,
        AgentCallOperation::Yield,
        AgentCallOperation::CompleteTask,
    ];
    let offsets = [46, 67, 94, 112, 127];

    for (operation, return_offset) in operations.into_iter().zip(offsets) {
        transcript.record(operation, return_offset).unwrap();
    }

    assert_eq!(transcript.call_count(), 5);
    assert_eq!(transcript.address_space_switch_count(), 10);
    assert_eq!(transcript.operations(), &operations);
    assert_eq!(transcript.return_offsets(), &offsets);
}

#[test]
fn transcript_requires_one_initial_describe_without_mutating_on_rejection() {
    let mut transcript = AgentCallTranscript::<3>::new();
    let empty = transcript;

    assert_eq!(
        transcript.record(AgentCallOperation::Yield, 12),
        Err(AgentCallTranscriptError::DescribeRequired)
    );
    assert_eq!(transcript, empty);

    transcript
        .record(AgentCallOperation::DescribeContext, 10)
        .unwrap();
    let described = transcript;
    assert_eq!(
        transcript.record(AgentCallOperation::DescribeContext, 20),
        Err(AgentCallTranscriptError::DuplicateDescribe)
    );
    assert_eq!(transcript, described);
}

#[test]
fn transcript_capacity_and_offset_failures_are_atomic() {
    let mut transcript = AgentCallTranscript::<2>::new();
    assert_eq!(
        transcript.record(AgentCallOperation::DescribeContext, 0),
        Err(AgentCallTranscriptError::InvalidReturnOffset)
    );
    assert!(transcript.is_empty());

    transcript
        .record(AgentCallOperation::DescribeContext, 10)
        .unwrap();
    transcript
        .record(AgentCallOperation::CompleteTask, 20)
        .unwrap();
    let full = transcript;

    assert_eq!(
        transcript.record(AgentCallOperation::Yield, 30),
        Err(AgentCallTranscriptError::Full)
    );
    assert_eq!(transcript, full);
}
