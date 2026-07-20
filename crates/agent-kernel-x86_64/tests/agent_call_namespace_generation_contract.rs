use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, MemoryCellId, NamespaceEntryId, NamespaceEntryRecord,
    NamespaceKey, NamespaceObject, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        encode_namespace_object, AgentCallContext, AgentCallDecodeError, AgentCallOperation,
        AgentCallRequest, AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION,
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0008;
const AUTHORITY: CapabilityId = CapabilityId::new(12);
const ENTRY: NamespaceEntryId = NamespaceEntryId::new(2);
const EXPECTED_REVISION: u64 = 1;

#[test]
fn namespace_generation_calls_decode_and_authenticate_exact_payloads() {
    assert_eq!(AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY, 48);
    assert_eq!(AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY, 49);
    let object = NamespaceObject::Agent(AgentId::new(8));
    let object_word = encode_namespace_object(object).unwrap();
    let requests = [
        (
            request_frame(
                AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                ENTRY.raw(),
                EXPECTED_REVISION,
                object_word,
            ),
            AgentCallRequest::CompareAndRebindNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                entry: ENTRY,
                expected_revision: EXPECTED_REVISION,
                object,
            },
            AgentCallOperation::CompareAndRebindNamespaceEntry,
        ),
        (
            request_frame(
                AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                ENTRY.raw(),
                EXPECTED_REVISION,
                0,
            ),
            AgentCallRequest::CompareAndRetireNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                entry: ENTRY,
                expected_revision: EXPECTED_REVISION,
            },
            AgentCallOperation::CompareAndRetireNamespaceEntry,
        ),
    ];

    for (frame, expected, operation) in requests {
        let decoded = AgentCallRequest::decode(&frame).expect("request decodes");
        assert_eq!(decoded, expected);
        assert_eq!(decoded.operation(), operation);
        assert!(context().authenticates(decoded, NONCE));
        assert!(!context().authenticates(decoded, NONCE + 1));
    }
}

#[test]
fn namespace_generation_calls_reject_zero_malformed_and_reserved_payloads() {
    for operation in [
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY,
    ] {
        let mut frame = valid_frame(operation);
        frame.r10 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
        let mut frame = valid_frame(operation);
        frame.r11 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
        let mut frame = valid_frame(operation);
        frame.r12 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
        let mut frame = valid_frame(operation);
        frame.r14 = 1;
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }

    let mut malformed = valid_frame(AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY);
    malformed.r13 = 14;
    assert_decode_error(malformed, AgentCallDecodeError::InvalidPayload);

    for register in [13, 15, 16] {
        let mut frame = valid_frame(AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY);
        set_payload_register(&mut frame, register, 1);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn namespace_generation_replies_return_complete_canonical_records() {
    let record = NamespaceEntryRecord {
        id: ENTRY,
        owner: AgentId::new(8),
        namespace: ResourceId::new(1),
        capability: AUTHORITY,
        key: NamespaceKey::new(0x4e53_0002),
        object: NamespaceObject::Agent(AgentId::new(8)),
        revision: 2,
    };
    for operation in [
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY,
    ] {
        let mut frame = valid_frame(operation);
        let control = control_words(&frame);
        match operation {
            AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY => context()
                .encode_namespace_compare_rebinding_reply(&mut frame, NONCE, record)
                .unwrap(),
            AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY => context()
                .encode_namespace_compare_retirement_reply(&mut frame, NONCE, record)
                .unwrap(),
            _ => unreachable!(),
        }

        assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
        assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
        assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
        assert_eq!(frame.rdx, operation);
        assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
        assert_eq!(
            [frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,],
            [2, 8, 1, 12, 0x4e53_0002, 65, 2]
        );
        assert_eq!(control_words(&frame), control);
    }
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

fn valid_frame(operation: u64) -> PrivilegeInterruptStackFrame {
    let object =
        encode_namespace_object(NamespaceObject::MemoryCell(MemoryCellId::new(2))).unwrap();
    match operation {
        AGENT_CALL_COMPARE_AND_REBIND_NAMESPACE_ENTRY => request_frame(
            operation,
            AUTHORITY.raw(),
            ENTRY.raw(),
            EXPECTED_REVISION,
            object,
        ),
        AGENT_CALL_COMPARE_AND_RETIRE_NAMESPACE_ENTRY => request_frame(
            operation,
            AUTHORITY.raw(),
            ENTRY.raw(),
            EXPECTED_REVISION,
            0,
        ),
        _ => unreachable!(),
    }
}

fn request_frame(
    operation: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13,
        r12,
        r11,
        r10,
        r9: NONCE,
        r8: 8,
        rbp: 0,
        rdi: 6,
        rsi: 8,
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

fn set_payload_register(frame: &mut PrivilegeInterruptStackFrame, register: u8, value: u64) {
    match register {
        13 => frame.r13 = value,
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
