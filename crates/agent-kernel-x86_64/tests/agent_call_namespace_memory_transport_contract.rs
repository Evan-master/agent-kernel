use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, NamespaceEntryId, NamespaceEntryRecord, NamespaceKey,
    NamespaceObject, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION,
        AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
    namespace_path_buffer::NAMESPACE_PATH_BUFFER_BYTES,
};

const NONCE: u64 = 0xa66e_0009;
const ROOT: ResourceId = ResourceId::new(1);
const GENERATION: u64 = 7;

#[test]
fn namespace_memory_path_call_decodes_and_authenticates_a_fixed_page_envelope() {
    assert_eq!(AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY, 51);
    let frame = request_frame();
    let request = AgentCallRequest::decode(&frame).unwrap();

    assert_eq!(
        request,
        AgentCallRequest::ResolveNamespacePathFromMemory {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            root: ROOT,
            generation: GENERATION,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::ResolveNamespacePathFromMemory
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn namespace_memory_path_call_rejects_zero_mismatched_and_reserved_words() {
    for register in [10, 11] {
        let mut frame = request_frame();
        set_register(&mut frame, register, 0);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }

    let mut wrong_length = request_frame();
    wrong_length.r12 -= 1;
    assert_decode_error(wrong_length, AgentCallDecodeError::InvalidPayload);

    for register in [13, 14, 15, 16] {
        let mut frame = request_frame();
        set_register(&mut frame, register, 1);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn namespace_memory_path_reply_returns_the_complete_terminal_record() {
    let record = NamespaceEntryRecord {
        id: NamespaceEntryId::new(4),
        owner: AgentId::new(8),
        namespace: ResourceId::new(9),
        capability: CapabilityId::new(15),
        key: NamespaceKey::new(0x1004),
        object: NamespaceObject::Agent(AgentId::new(8)),
        revision: 1,
    };
    let mut frame = request_frame();
    let control = control_words(&frame);

    context()
        .encode_namespace_memory_path_resolution_reply(&mut frame, NONCE, record)
        .unwrap();

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
    assert_eq!(
        [frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp],
        [4, 8, 9, 15, 0x1004, 65, 1]
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
        r12: NAMESPACE_PATH_BUFFER_BYTES as u64,
        r11: GENERATION,
        r10: ROOT.raw(),
        r9: NONCE,
        r8: 8,
        rbp: 0,
        rdi: 6,
        rsi: 8,
        rdx: 0,
        rcx: AGENT_CALL_RESOLVE_NAMESPACE_PATH_FROM_MEMORY,
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
