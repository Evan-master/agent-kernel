use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, ResourceId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xf66c_e007;
const AUTHORITY: CapabilityId = CapabilityId::new(12);
const TARGET: AgentImageId = AgentImageId::new(9);
const RESOURCE: ResourceId = ResourceId::new(1);
const OWNER: AgentId = AgentId::new(1);

#[test]
fn agent_image_record_retirement_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD, 37);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::RetireAgentImageRecord {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            target: TARGET,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::RetireAgentImageRecord
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn agent_image_record_retirement_rejects_zero_and_reserved_payloads() {
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
fn agent_image_record_retirement_reply_is_canonical() {
    let mut frame = request_frame();
    let control = control_words(&frame);
    context()
        .encode_agent_image_record_retirement_reply(&mut frame, NONCE, TARGET, RESOURCE, OWNER)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(
        payload(&frame),
        [TARGET.raw(), RESOURCE.raw(), OWNER.raw(), 0, 0, 0, 0]
    );
    assert_eq!(control_words(&frame), control);

    for (target, resource, owner) in [
        (AgentImageId::new(0), RESOURCE, OWNER),
        (TARGET, ResourceId::new(0), OWNER),
        (TARGET, RESOURCE, AgentId::new(0)),
    ] {
        assert_eq!(
            context().encode_agent_image_record_retirement_reply(
                &mut frame, NONCE, target, resource, owner,
            ),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(8),
        TaskId::new(6),
        AgentImageId::new(8),
        CapabilityId::new(11),
    )
    .expect("context is valid")
}

fn request_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: TARGET.raw(),
        r10: AUTHORITY.raw(),
        r9: NONCE,
        r8: 8,
        rbp: 0,
        rdi: 6,
        rsi: 8,
        rdx: 0,
        rcx: AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD,
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
    assert_eq!(frame.rdx, AGENT_CALL_RETIRE_AGENT_IMAGE_RECORD);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
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
