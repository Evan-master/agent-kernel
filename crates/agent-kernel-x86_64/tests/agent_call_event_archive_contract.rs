use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, EventArchiveDigest, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_ARCHIVE_EVENTS,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa88c_e010;
const AUTHORITY: CapabilityId = CapabilityId::new(23);
const THROUGH: u64 = 64;
const DIGEST: EventArchiveDigest = EventArchiveDigest::new([
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
]);

#[test]
fn event_archive_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_ARCHIVE_EVENTS, 40);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::ArchiveEvents {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(12),
            nonce: NONCE,
            authority: AUTHORITY,
            through_sequence: THROUGH,
        }
    );
    assert_eq!(request.operation(), AgentCallOperation::ArchiveEvents);
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn event_archive_rejects_zero_and_reserved_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
    ] {
        let mut frame = request_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r14 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r15 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.rbp = 1,
    ] {
        let mut frame = request_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn event_archive_reply_is_canonical() {
    let mut frame = request_frame();
    let control = control_words(&frame);
    context()
        .encode_event_archive_reply(&mut frame, NONCE, 1, THROUGH, 64, DIGEST)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(payload(&frame)[..3], [1, 64, 64]);
    assert_eq!(payload(&frame)[3..], DIGEST.words_le());
    assert_eq!(control_words(&frame), control);

    for (first, through, count) in [
        (0, 64, 64),
        (65, 64, 64),
        (1, 0, 64),
        (1, 64, 0),
        (1, 64, 63),
    ] {
        assert_eq!(
            context().encode_event_archive_reply(&mut frame, NONCE, first, through, count, DIGEST,),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(12),
        TaskId::new(10),
        AgentImageId::new(12),
        CapabilityId::new(22),
    )
    .expect("context is valid")
}

fn request_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: THROUGH,
        r10: AUTHORITY.raw(),
        r9: NONCE,
        r8: 12,
        rbp: 0,
        rdi: 10,
        rsi: 12,
        rdx: 0,
        rcx: AGENT_CALL_ARCHIVE_EVENTS,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_ARCHIVE_EVENTS);
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [12, 10, 12, NONCE]
    );
}

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
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
