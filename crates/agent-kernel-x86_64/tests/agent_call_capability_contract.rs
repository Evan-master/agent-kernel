use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, Operation, OperationSet, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_DERIVE_CAPABILITY,
        AGENT_CALL_REVOKE_DERIVED_CAPABILITY, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xf66c_e006;
const SOURCE: CapabilityId = CapabilityId::new(11);
const DERIVED: CapabilityId = CapabilityId::new(12);
const TARGET: AgentId = AgentId::new(2);

#[test]
fn capability_requests_decode_and_authenticate_against_trusted_context() {
    assert_eq!(AGENT_CALL_DERIVE_CAPABILITY, 12);
    assert_eq!(AGENT_CALL_REVOKE_DERIVED_CAPABILITY, 13);

    let derive = AgentCallRequest::decode(&derive_frame()).unwrap();
    assert_eq!(
        derive,
        AgentCallRequest::DeriveCapability {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            source: SOURCE,
            target: TARGET,
            operations: OperationSet::only(Operation::Observe),
        }
    );
    assert_eq!(derive.operation(), AgentCallOperation::DeriveCapability);
    assert!(context().authenticates(derive, NONCE));
    assert!(!context().authenticates(derive, NONCE + 1));

    let revoke = AgentCallRequest::decode(&revoke_frame()).unwrap();
    assert_eq!(
        revoke,
        AgentCallRequest::RevokeDerivedCapability {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            source: SOURCE,
            target: DERIVED,
        }
    );
    assert_eq!(
        revoke.operation(),
        AgentCallOperation::RevokeDerivedCapability
    );
    assert!(context().authenticates(revoke, NONCE));
}

#[test]
fn capability_requests_reject_zero_unknown_and_reserved_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 1 << 6,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = u64::MAX,
    ] {
        let mut frame = derive_frame();
        mutate(&mut frame);
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
    let mut derive_reserved = derive_frame();
    derive_reserved.r13 = 1;
    assert_eq!(
        AgentCallRequest::decode(&derive_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );

    for clear in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
    ] {
        let mut frame = revoke_frame();
        clear(&mut frame);
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
    let mut revoke_reserved = revoke_frame();
    revoke_reserved.r12 = 1;
    assert_eq!(
        AgentCallRequest::decode(&revoke_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn capability_replies_preserve_context_and_return_kernel_handles() {
    let mut derive = derive_frame();
    let derive_control = control_words(&derive);
    context()
        .encode_capability_derived_reply(&mut derive, NONCE, DERIVED)
        .unwrap();
    assert_common_reply(&derive, AGENT_CALL_DERIVE_CAPABILITY);
    assert_eq!([derive.r10, derive.r11], [12, 0]);
    assert_eq!(
        [derive.r12, derive.r13, derive.r14, derive.r15, derive.rbp],
        [0; 5]
    );
    assert_eq!(control_words(&derive), derive_control);

    let mut revoke = revoke_frame();
    let revoke_control = control_words(&revoke);
    context()
        .encode_capability_revoked_reply(&mut revoke, NONCE, SOURCE, DERIVED)
        .unwrap();
    assert_common_reply(&revoke, AGENT_CALL_REVOKE_DERIVED_CAPABILITY);
    assert_eq!([revoke.r10, revoke.r11], [12, 11]);
    assert_eq!(
        [revoke.r12, revoke.r13, revoke.r14, revoke.r15, revoke.rbp],
        [0; 5]
    );
    assert_eq!(control_words(&revoke), revoke_control);

    assert_eq!(
        context().encode_capability_derived_reply(&mut derive, NONCE, CapabilityId::new(0)),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    assert_eq!(
        context().encode_capability_revoked_reply(
            &mut revoke,
            NONCE,
            CapabilityId::new(0),
            DERIVED,
        ),
        Err(AgentCallDecodeError::InvalidPayload)
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(8),
        TaskId::new(6),
        AgentImageId::new(8),
        CapabilityId::new(9),
    )
    .unwrap()
}

fn derive_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_DERIVE_CAPABILITY,
        [
            SOURCE.raw(),
            TARGET.raw(),
            u64::from(OperationSet::only(Operation::Observe).bits()),
            0,
            0,
            0,
            0,
        ],
    )
}

fn revoke_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_REVOKE_DERIVED_CAPABILITY,
        [SOURCE.raw(), DERIVED.raw(), 0, 0, 0, 0, 0],
    )
}

fn request_frame(operation: u64, payload: [u64; 7]) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: payload[5],
        r14: payload[4],
        r13: payload[3],
        r12: payload[2],
        r11: payload[1],
        r10: payload[0],
        r9: NONCE,
        r8: 8,
        rbp: payload[6],
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

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame, operation: u64) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, operation);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [8, 6, 8, NONCE]);
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
