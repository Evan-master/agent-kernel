use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa11c_e053;
const MESSAGE_GENERATION: u64 = 7;

#[test]
fn call_53_decodes_one_authenticated_typed_message_generation() {
    assert_eq!(AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER, 53);
    let frame = request_frame(MESSAGE_GENERATION);
    let request = AgentCallRequest::decode(&frame).expect("call 53 should decode");

    assert_eq!(
        request,
        AgentCallRequest::RotateAgentImageSignerFromMemory {
            agent: AgentId::new(3),
            task: TaskId::new(9),
            image: AgentImageId::new(4),
            nonce: NONCE,
            generation: MESSAGE_GENERATION,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::RotateAgentImageSignerFromMemory
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn call_53_rejects_zero_generation_and_nonzero_reserved_registers() {
    let mut zero_generation = request_frame(0);
    let mut nonzero_r11 = request_frame(MESSAGE_GENERATION);
    nonzero_r11.r11 = 1;
    let mut nonzero_rbp = request_frame(MESSAGE_GENERATION);
    nonzero_rbp.rbp = 1;
    zero_generation.r10 = 0;

    assert_eq!(
        AgentCallRequest::decode(&zero_generation),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    assert_eq!(
        AgentCallRequest::decode(&nonzero_r11),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
    assert_eq!(
        AgentCallRequest::decode(&nonzero_rbp),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn call_53_reply_binds_policy_generation_and_rotation_evidence() {
    let context = context();
    let mut frame = request_frame(MESSAGE_GENERATION);
    let control = (
        frame.rip,
        frame.cs,
        frame.rflags,
        frame.user_rsp,
        frame.user_ss,
    );

    context
        .encode_agent_image_signer_rotation_reply(&mut frame, NONCE, 2, 2)
        .expect("canonical rotation reply should encode");

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [3, 9, 4, NONCE]);
    assert_eq!([frame.r10, frame.r11, frame.r12], [2, 2, 2]);
    assert_eq!([frame.r13, frame.r14, frame.r15, frame.rbp], [2, 1, 0, 0]);
    assert_eq!(
        (
            frame.rip,
            frame.cs,
            frame.rflags,
            frame.user_rsp,
            frame.user_ss,
        ),
        control
    );
    assert_eq!(
        context.encode_agent_image_signer_rotation_reply(&mut frame, NONCE, 0, 2),
        Err(AgentCallDecodeError::InvalidPayload)
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(3),
        TaskId::new(9),
        AgentImageId::new(4),
        CapabilityId::new(7),
    )
    .expect("context should be canonical")
}

fn request_frame(generation: u64) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: generation,
        r9: NONCE,
        r8: 4,
        rbp: 0,
        rdi: 9,
        rsi: 3,
        rdx: 0,
        rcx: AGENT_CALL_ROTATE_AGENT_IMAGE_SIGNER,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}
