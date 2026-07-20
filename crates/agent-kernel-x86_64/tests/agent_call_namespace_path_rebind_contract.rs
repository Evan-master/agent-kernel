use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, NamespaceEntryId, NamespaceEntryRecord, NamespaceKey,
    NamespaceObject, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION,
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_PATH_FROM_MEMORY, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0009;
const GENERATION: u64 = 1;

#[test]
fn typed_namespace_path_rebind_call_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_PATH_FROM_MEMORY, 52);
    let frame = request_frame();
    let request = AgentCallRequest::decode(&frame).unwrap();

    assert_eq!(
        request,
        AgentCallRequest::CompareAndRebindNamespacePathFromMemory {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            generation: GENERATION,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::CompareAndRebindNamespacePathFromMemory
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn typed_namespace_path_rebind_call_rejects_envelope_and_reserved_words() {
    let mut zero_generation = request_frame();
    zero_generation.r10 = 0;
    assert_decode_error(zero_generation, AgentCallDecodeError::InvalidPayload);

    for register in [11, 12, 13, 14, 15, 16] {
        let mut frame = request_frame();
        set_register(&mut frame, register, 1);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn typed_namespace_path_rebind_reply_returns_complete_resulting_record() {
    let record = NamespaceEntryRecord {
        id: NamespaceEntryId::new(4),
        owner: AgentId::new(8),
        namespace: ResourceId::new(9),
        capability: CapabilityId::new(21),
        key: NamespaceKey::new(0x4e53_0004),
        object: NamespaceObject::Resource(ResourceId::new(3)),
        revision: 2,
    };
    let mut frame = request_frame();
    let control = control_words(&frame);

    context()
        .encode_namespace_path_rebinding_reply(&mut frame, NONCE, record)
        .unwrap();

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(
        frame.rdx,
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_PATH_FROM_MEMORY
    );
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
    assert_eq!(
        [frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp],
        [4, 8, 9, 21, 0x4e53_0004, 26, 2]
    );
    assert_eq!(control_words(&frame), control);
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(8),
        TaskId::new(6),
        AgentImageId::new(8),
        CapabilityId::new(11),
    )
    .unwrap()
}

fn request_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: GENERATION,
        r9: NONCE,
        r8: 8,
        rbp: 0,
        rdi: 6,
        rsi: 8,
        rdx: 0,
        rcx: AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_PATH_FROM_MEMORY,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}

fn set_register(frame: &mut PrivilegeInterruptStackFrame, register: u8, value: u64) {
    match register {
        10 => frame.r10 = value,
        11 => frame.r11 = value,
        12 => frame.r12 = value,
        13 => frame.r13 = value,
        14 => frame.r14 = value,
        15 => frame.r15 = value,
        16 => frame.rbp = value,
        _ => unreachable!(),
    }
}

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
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
