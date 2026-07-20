use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, NamespaceEntryId, NamespaceEntryRecord, NamespaceKey,
    NamespaceObject, NamespacePathSegment, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_RESOLVE_NAMESPACE_PATH,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0008;
const ROOT: ResourceId = ResourceId::new(1);
const FIRST_AUTHORITY: CapabilityId = CapabilityId::new(12);
const SECOND_AUTHORITY: CapabilityId = CapabilityId::new(13);
const FIRST_KEY: NamespaceKey = NamespaceKey::new(0x4e53_0001);
const SECOND_KEY: NamespaceKey = NamespaceKey::new(0x4e53_0002);

#[test]
fn namespace_path_call_decodes_one_and_two_hop_payloads() {
    assert_eq!(AGENT_CALL_RESOLVE_NAMESPACE_PATH, 50);
    let cases = [
        (
            request_frame(1, 0, 0),
            AgentCallRequest::ResolveNamespacePath {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                root: ROOT,
                first: NamespacePathSegment::new(FIRST_AUTHORITY, FIRST_KEY),
                second: None,
            },
        ),
        (
            request_frame(2, SECOND_AUTHORITY.raw(), SECOND_KEY.raw()),
            AgentCallRequest::ResolveNamespacePath {
                agent: AgentId::new(8),
                task: TaskId::new(6),
                image: AgentImageId::new(8),
                nonce: NONCE,
                root: ROOT,
                first: NamespacePathSegment::new(FIRST_AUTHORITY, FIRST_KEY),
                second: Some(NamespacePathSegment::new(SECOND_AUTHORITY, SECOND_KEY)),
            },
        ),
    ];

    for (frame, expected) in cases {
        let decoded = AgentCallRequest::decode(&frame).expect("path request decodes");
        assert_eq!(decoded, expected);
        assert_eq!(
            decoded.operation(),
            AgentCallOperation::ResolveNamespacePath
        );
        assert!(context().authenticates(decoded, NONCE));
        assert!(!context().authenticates(decoded, NONCE + 1));
    }
}

#[test]
fn namespace_path_call_rejects_invalid_depth_identity_and_reserved_payloads() {
    for depth in [0, 3, u64::MAX] {
        assert_decode_error(
            request_frame(depth, SECOND_AUTHORITY.raw(), SECOND_KEY.raw()),
            AgentCallDecodeError::InvalidPayload,
        );
    }
    for register in [10, 12] {
        let mut frame = request_frame(2, SECOND_AUTHORITY.raw(), SECOND_KEY.raw());
        set_register(&mut frame, register, 0);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    let mut missing_second_authority = request_frame(2, 0, SECOND_KEY.raw());
    assert_decode_error(
        missing_second_authority,
        AgentCallDecodeError::InvalidPayload,
    );
    missing_second_authority.r14 = SECOND_AUTHORITY.raw();
    missing_second_authority.rbp = 1;
    assert_decode_error(
        missing_second_authority,
        AgentCallDecodeError::ReservedNotZero,
    );
    for register in [14, 15] {
        let mut frame = request_frame(1, 0, 0);
        set_register(&mut frame, register, 1);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn namespace_path_reply_returns_the_complete_terminal_record() {
    let record = NamespaceEntryRecord {
        id: NamespaceEntryId::new(2),
        owner: AgentId::new(8),
        namespace: ResourceId::new(3),
        capability: SECOND_AUTHORITY,
        key: SECOND_KEY,
        object: NamespaceObject::Agent(AgentId::new(8)),
        revision: 1,
    };
    let mut frame = request_frame(2, SECOND_AUTHORITY.raw(), SECOND_KEY.raw());
    let control = control_words(&frame);

    context()
        .encode_namespace_path_resolution_reply(&mut frame, NONCE, record)
        .expect("terminal record reply encodes");

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_RESOLVE_NAMESPACE_PATH);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
    assert_eq!(
        [frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,],
        [2, 8, 3, 13, SECOND_KEY.raw(), 65, 1]
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

fn request_frame(
    depth: u64,
    second_authority: u64,
    second_key: u64,
) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: second_key,
        r14: second_authority,
        r13: FIRST_KEY.raw(),
        r12: FIRST_AUTHORITY.raw(),
        r11: depth,
        r10: ROOT.raw(),
        r9: NONCE,
        r8: 8,
        rbp: 0,
        rdi: 6,
        rsi: 8,
        rdx: 0,
        rcx: AGENT_CALL_RESOLVE_NAMESPACE_PATH,
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
        12 => frame.r12 = value,
        14 => frame.r14 = value,
        15 => frame.r15 = value,
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
