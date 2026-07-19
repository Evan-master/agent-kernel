use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId, WaiterId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_COMPACT_WAITERS,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa88c_e008;
const AUTHORITY: CapabilityId = CapabilityId::new(23);
const THROUGH: WaiterId = WaiterId::new(3);

#[test]
fn waiter_compaction_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_COMPACT_WAITERS, 38);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::CompactWaiters {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(12),
            nonce: NONCE,
            authority: AUTHORITY,
            through: THROUGH,
        }
    );
    assert_eq!(request.operation(), AgentCallOperation::CompactWaiters);
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn waiter_compaction_rejects_zero_and_reserved_payloads() {
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
fn waiter_compaction_reply_is_canonical() {
    let mut frame = request_frame();
    let control = control_words(&frame);
    context()
        .encode_waiter_compaction_reply(&mut frame, NONCE, WaiterId::new(1), THROUGH, 3)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(payload(&frame), [1, 3, 3, 0, 0, 0, 0]);
    assert_eq!(control_words(&frame), control);

    for (first, through, count) in [
        (WaiterId::new(0), THROUGH, 3),
        (WaiterId::new(4), THROUGH, 3),
        (WaiterId::new(1), WaiterId::new(0), 3),
        (WaiterId::new(1), THROUGH, 0),
    ] {
        assert_eq!(
            context().encode_waiter_compaction_reply(&mut frame, NONCE, first, through, count,),
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
        r11: THROUGH.raw(),
        r10: AUTHORITY.raw(),
        r9: NONCE,
        r8: 12,
        rbp: 0,
        rdi: 10,
        rsi: 12,
        rdx: 0,
        rcx: AGENT_CALL_COMPACT_WAITERS,
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
    assert_eq!(frame.rdx, AGENT_CALL_COMPACT_WAITERS);
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
