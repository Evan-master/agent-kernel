use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, MemoryCellId, MessageId, NamespaceEntryId,
    NamespaceEntryRecord, NamespaceKey, NamespaceObject, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        decode_namespace_object, encode_namespace_object, AgentCallContext, AgentCallDecodeError,
        AgentCallOperation, AgentCallRequest, AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION,
        AGENT_CALL_BIND_NAMESPACE_ENTRY, AGENT_CALL_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_RESOLVE_NAMESPACE_ENTRY, AGENT_CALL_RETIRE_NAMESPACE_ENTRY,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0008;
const AUTHORITY: CapabilityId = CapabilityId::new(12);
const NAMESPACE: ResourceId = ResourceId::new(1);
const ENTRY: NamespaceEntryId = NamespaceEntryId::new(7);
const KEY: NamespaceKey = NamespaceKey::new(0x4e53_0001);

#[test]
fn namespace_object_words_round_trip_all_native_object_kinds() {
    let objects = [
        NamespaceObject::Agent(AgentId::new(9)),
        NamespaceObject::Resource(ResourceId::new(9)),
        NamespaceObject::Task(TaskId::new(9)),
        NamespaceObject::Message(MessageId::new(9)),
        NamespaceObject::MemoryCell(MemoryCellId::new(9)),
        NamespaceObject::Mount(ResourceId::new(9)),
    ];

    for (index, object) in objects.into_iter().enumerate() {
        let word = encode_namespace_object(object).expect("object encodes");
        assert_eq!(word, (9 << 3) | (index as u64 + 1));
        assert_eq!(decode_namespace_object(word), Some(object));
    }
    let largest = NamespaceObject::Resource(ResourceId::new((1_u64 << 61) - 1));
    assert_eq!(decode_namespace_object(u64::MAX - 5), Some(largest));
    assert_eq!(encode_namespace_object(largest), Some(u64::MAX - 5));
}

#[test]
fn namespace_object_words_reject_zero_reserved_and_oversized_values() {
    for invalid in [0, 1, 2, 3, 4, 5, 6, 8, 15] {
        assert_eq!(decode_namespace_object(invalid), None);
    }
    assert_eq!(
        encode_namespace_object(NamespaceObject::Agent(AgentId::new(0))),
        None
    );
    assert_eq!(
        encode_namespace_object(NamespaceObject::MemoryCell(MemoryCellId::new(1_u64 << 61))),
        None
    );
}

#[test]
fn namespace_calls_decode_and_authenticate_exact_payloads() {
    assert_eq!(AGENT_CALL_BIND_NAMESPACE_ENTRY, 44);
    assert_eq!(AGENT_CALL_RESOLVE_NAMESPACE_ENTRY, 45);
    assert_eq!(AGENT_CALL_REBIND_NAMESPACE_ENTRY, 46);
    assert_eq!(AGENT_CALL_RETIRE_NAMESPACE_ENTRY, 47);
    let object = NamespaceObject::MemoryCell(MemoryCellId::new(2));
    let object_word = encode_namespace_object(object).unwrap();
    let requests = [
        (
            request_frame(
                AGENT_CALL_BIND_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                NAMESPACE.raw(),
                KEY.raw(),
                object_word,
            ),
            AgentCallRequest::BindNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                namespace: NAMESPACE,
                key: KEY,
                object,
            },
            AgentCallOperation::BindNamespaceEntry,
        ),
        (
            request_frame(
                AGENT_CALL_RESOLVE_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                NAMESPACE.raw(),
                KEY.raw(),
                0,
            ),
            AgentCallRequest::ResolveNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                namespace: NAMESPACE,
                key: KEY,
            },
            AgentCallOperation::ResolveNamespaceEntry,
        ),
        (
            request_frame(
                AGENT_CALL_REBIND_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                ENTRY.raw(),
                object_word,
                0,
            ),
            AgentCallRequest::RebindNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                entry: ENTRY,
                object,
            },
            AgentCallOperation::RebindNamespaceEntry,
        ),
        (
            request_frame(
                AGENT_CALL_RETIRE_NAMESPACE_ENTRY,
                AUTHORITY.raw(),
                ENTRY.raw(),
                0,
                0,
            ),
            AgentCallRequest::RetireNamespaceEntry {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                authority: AUTHORITY,
                entry: ENTRY,
            },
            AgentCallOperation::RetireNamespaceEntry,
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
fn namespace_calls_reject_zero_malformed_and_reserved_payloads() {
    for operation in [
        AGENT_CALL_BIND_NAMESPACE_ENTRY,
        AGENT_CALL_RESOLVE_NAMESPACE_ENTRY,
        AGENT_CALL_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_RETIRE_NAMESPACE_ENTRY,
    ] {
        let mut frame = valid_frame(operation);
        frame.r10 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
        let mut frame = valid_frame(operation);
        frame.r11 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }

    let mut malformed_bind = valid_frame(AGENT_CALL_BIND_NAMESPACE_ENTRY);
    malformed_bind.r13 = 15;
    assert_decode_error(malformed_bind, AgentCallDecodeError::InvalidPayload);
    let mut malformed_rebind = valid_frame(AGENT_CALL_REBIND_NAMESPACE_ENTRY);
    malformed_rebind.r12 = 8;
    assert_decode_error(malformed_rebind, AgentCallDecodeError::InvalidPayload);

    for (operation, register) in [
        (AGENT_CALL_BIND_NAMESPACE_ENTRY, 14),
        (AGENT_CALL_BIND_NAMESPACE_ENTRY, 15),
        (AGENT_CALL_BIND_NAMESPACE_ENTRY, 16),
        (AGENT_CALL_RESOLVE_NAMESPACE_ENTRY, 13),
        (AGENT_CALL_REBIND_NAMESPACE_ENTRY, 13),
        (AGENT_CALL_RETIRE_NAMESPACE_ENTRY, 12),
    ] {
        let mut frame = valid_frame(operation);
        set_payload_register(&mut frame, register, 1);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn namespace_replies_return_the_complete_canonical_record() {
    let record = sample_record();
    for operation in [
        AGENT_CALL_BIND_NAMESPACE_ENTRY,
        AGENT_CALL_RESOLVE_NAMESPACE_ENTRY,
        AGENT_CALL_REBIND_NAMESPACE_ENTRY,
        AGENT_CALL_RETIRE_NAMESPACE_ENTRY,
    ] {
        let mut frame = valid_frame(operation);
        let control = control_words(&frame);
        match operation {
            AGENT_CALL_BIND_NAMESPACE_ENTRY => context()
                .encode_namespace_binding_reply(&mut frame, NONCE, record)
                .unwrap(),
            AGENT_CALL_RESOLVE_NAMESPACE_ENTRY => context()
                .encode_namespace_resolution_reply(&mut frame, NONCE, record)
                .unwrap(),
            AGENT_CALL_REBIND_NAMESPACE_ENTRY => context()
                .encode_namespace_rebinding_reply(&mut frame, NONCE, record)
                .unwrap(),
            AGENT_CALL_RETIRE_NAMESPACE_ENTRY => context()
                .encode_namespace_retirement_reply(&mut frame, NONCE, record)
                .unwrap(),
            _ => unreachable!(),
        }
        assert_common_reply(&frame, operation);
        assert_eq!(payload(&frame), [7, 8, 1, 12, KEY.raw(), 18, 2]);
        assert_eq!(control_words(&frame), control);
    }
}

#[test]
fn namespace_replies_reject_invalid_records() {
    let record = sample_record();
    for invalid in [
        NamespaceEntryRecord {
            id: NamespaceEntryId::new(0),
            ..record
        },
        NamespaceEntryRecord {
            owner: AgentId::new(0),
            ..record
        },
        NamespaceEntryRecord {
            namespace: ResourceId::new(0),
            ..record
        },
        NamespaceEntryRecord {
            capability: CapabilityId::new(0),
            ..record
        },
        NamespaceEntryRecord {
            object: NamespaceObject::Task(TaskId::new(0)),
            ..record
        },
        NamespaceEntryRecord {
            revision: 0,
            ..record
        },
    ] {
        assert_eq!(
            context().encode_namespace_binding_reply(
                &mut valid_frame(AGENT_CALL_BIND_NAMESPACE_ENTRY),
                NONCE,
                invalid,
            ),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

fn sample_record() -> NamespaceEntryRecord {
    NamespaceEntryRecord {
        id: ENTRY,
        owner: AgentId::new(8),
        namespace: NAMESPACE,
        capability: AUTHORITY,
        key: KEY,
        object: NamespaceObject::Resource(ResourceId::new(2)),
        revision: 2,
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

fn valid_frame(operation: u64) -> PrivilegeInterruptStackFrame {
    let object =
        encode_namespace_object(NamespaceObject::MemoryCell(MemoryCellId::new(2))).unwrap();
    match operation {
        AGENT_CALL_BIND_NAMESPACE_ENTRY => request_frame(
            operation,
            AUTHORITY.raw(),
            NAMESPACE.raw(),
            KEY.raw(),
            object,
        ),
        AGENT_CALL_RESOLVE_NAMESPACE_ENTRY => {
            request_frame(operation, AUTHORITY.raw(), NAMESPACE.raw(), KEY.raw(), 0)
        }
        AGENT_CALL_REBIND_NAMESPACE_ENTRY => {
            request_frame(operation, AUTHORITY.raw(), ENTRY.raw(), object, 0)
        }
        AGENT_CALL_RETIRE_NAMESPACE_ENTRY => {
            request_frame(operation, AUTHORITY.raw(), ENTRY.raw(), 0, 0)
        }
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
        12 => frame.r12 = value,
        13 => frame.r13 = value,
        14 => frame.r14 = value,
        15 => frame.r15 = value,
        16 => frame.rbp = value,
        _ => unreachable!(),
    }
}

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame, operation: u64) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, operation);
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
