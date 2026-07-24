use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, EventArchiveDigest, TaskId, DURABLE_ARCHIVE_MANIFEST_BYTES,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_COMMIT_DURABLE_ARCHIVE,
        AGENT_CALL_PREPARE_DURABLE_ARCHIVE, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
    durable_archive_request::DURABLE_ARCHIVE_REQUEST_BYTES,
};

const NONCE: u64 = 0xa11c_e054;
const ARCHIVE_AUTHORITY: CapabilityId = CapabilityId::new(31);
const STORAGE_AUTHORITY: CapabilityId = CapabilityId::new(32);
const THROUGH: u64 = 64;
const CALL_DATA_GENERATION: u64 = 7;
const DIGEST: EventArchiveDigest = EventArchiveDigest::new([0x5a; 32]);

#[test]
fn calls_54_and_55_decode_and_authenticate_exact_register_contracts() {
    assert_eq!(AGENT_CALL_PREPARE_DURABLE_ARCHIVE, 54);
    assert_eq!(AGENT_CALL_COMMIT_DURABLE_ARCHIVE, 55);

    let prepare = AgentCallRequest::decode(&prepare_frame()).unwrap();
    assert_eq!(
        prepare,
        AgentCallRequest::PrepareDurableArchive {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(12),
            nonce: NONCE,
            archive_authority: ARCHIVE_AUTHORITY,
            storage_authority: STORAGE_AUTHORITY,
            through_sequence: THROUGH,
            generation: CALL_DATA_GENERATION,
        }
    );
    assert_eq!(
        prepare.operation(),
        AgentCallOperation::PrepareDurableArchive
    );
    assert!(context().authenticates(prepare, NONCE));

    let commit = AgentCallRequest::decode(&commit_frame()).unwrap();
    assert_eq!(
        commit,
        AgentCallRequest::CommitDurableArchiveFromMemory {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(12),
            nonce: NONCE,
            generation: CALL_DATA_GENERATION,
        }
    );
    assert_eq!(
        commit.operation(),
        AgentCallOperation::CommitDurableArchiveFromMemory
    );
    assert!(context().authenticates(commit, NONCE));
    assert!(!context().authenticates(commit, NONCE + 1));
}

#[test]
fn durable_archive_calls_reject_zero_payloads_and_reserved_registers() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 0,
    ] {
        let mut frame = prepare_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r14 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r15 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.rbp = 1,
    ] {
        let mut frame = prepare_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }

    let mut zero_generation = commit_frame();
    zero_generation.r10 = 0;
    assert_decode_error(zero_generation, AgentCallDecodeError::InvalidPayload);
    let mut reserved = commit_frame();
    reserved.r11 = 1;
    assert_decode_error(reserved, AgentCallDecodeError::ReservedNotZero);
}

#[test]
fn prepare_and_commit_replies_are_canonical() {
    let mut prepare = prepare_frame();
    let prepare_control = control_words(&prepare);
    context()
        .encode_durable_archive_prepare_reply(&mut prepare, NONCE, 1, 1, THROUGH, 64, 23)
        .unwrap();
    assert_common_reply(&prepare, AGENT_CALL_PREPARE_DURABLE_ARCHIVE);
    assert_eq!(
        payload(&prepare),
        [
            1,
            1,
            THROUGH,
            64,
            DURABLE_ARCHIVE_MANIFEST_BYTES as u64,
            DURABLE_ARCHIVE_REQUEST_BYTES as u64,
            23,
        ]
    );
    assert_eq!(control_words(&prepare), prepare_control);

    let mut commit = commit_frame();
    let commit_control = control_words(&commit);
    context()
        .encode_durable_archive_commit_reply(&mut commit, NONCE, 1, THROUGH, 64, DIGEST)
        .unwrap();
    assert_common_reply(&commit, AGENT_CALL_COMMIT_DURABLE_ARCHIVE);
    assert_eq!(payload(&commit)[..3], [1, THROUGH, 64]);
    assert_eq!(payload(&commit)[3..], DIGEST.words_le());
    assert_eq!(control_words(&commit), commit_control);

    assert_eq!(
        context().encode_durable_archive_prepare_reply(&mut prepare, NONCE, 0, 1, THROUGH, 64, 23,),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    assert_eq!(
        context().encode_durable_archive_commit_reply(&mut commit, NONCE, 1, THROUGH, 63, DIGEST),
        Err(AgentCallDecodeError::InvalidPayload)
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(12),
        TaskId::new(10),
        AgentImageId::new(12),
        CapabilityId::new(22),
    )
    .unwrap()
}

fn prepare_frame() -> PrivilegeInterruptStackFrame {
    frame(
        AGENT_CALL_PREPARE_DURABLE_ARCHIVE,
        [
            ARCHIVE_AUTHORITY.raw(),
            STORAGE_AUTHORITY.raw(),
            THROUGH,
            CALL_DATA_GENERATION,
            0,
            0,
            0,
        ],
    )
}

fn commit_frame() -> PrivilegeInterruptStackFrame {
    frame(
        AGENT_CALL_COMMIT_DURABLE_ARCHIVE,
        [CALL_DATA_GENERATION, 0, 0, 0, 0, 0, 0],
    )
}

fn frame(operation: u64, payload: [u64; 7]) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: payload[5],
        r14: payload[4],
        r13: payload[3],
        r12: payload[2],
        r11: payload[1],
        r10: payload[0],
        r9: NONCE,
        r8: 12,
        rbp: payload[6],
        rdi: 10,
        rsi: 12,
        rdx: 0,
        rcx: operation,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
}

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame, operation: u64) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, operation);
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [12, 10, 12, NONCE]
    );
}

fn payload(frame: &PrivilegeInterruptStackFrame) -> [u64; 7] {
    [
        frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,
    ]
}

fn control_words(frame: &PrivilegeInterruptStackFrame) -> [u64; 5] {
    [
        frame.rip,
        frame.cs,
        frame.rflags,
        frame.user_rsp,
        frame.user_ss,
    ]
}
