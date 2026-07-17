use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, IntentId, IntentKind, ResourceId, TaskId,
    VerificationRequirement,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_CREATE_TASK,
        AGENT_CALL_DECLARE_INTENT, AGENT_CALL_DELEGATE_TASK, AGENT_CALL_INTENT_ACT,
        AGENT_CALL_STATUS_OK, AGENT_CALL_VERIFICATION_OPTIONAL,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xf66c_e006;
const AUTHORITY: CapabilityId = CapabilityId::new(10);
const RESOURCE: ResourceId = ResourceId::new(1);
const INTENT: IntentId = IntentId::new(7);
const MANAGED_TASK: TaskId = TaskId::new(7);
const TARGET: AgentId = AgentId::new(2);
const DELEGATED_CAPABILITY: CapabilityId = CapabilityId::new(13);

#[test]
fn task_manager_requests_decode_and_authenticate() {
    assert_eq!(
        [
            AGENT_CALL_DECLARE_INTENT,
            AGENT_CALL_CREATE_TASK,
            AGENT_CALL_DELEGATE_TASK,
        ],
        [14, 15, 16]
    );
    let declare = AgentCallRequest::decode(&declare_frame()).unwrap();
    assert_eq!(
        declare,
        AgentCallRequest::DeclareIntent {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            resource: RESOURCE,
            kind: IntentKind::Act,
            verification: VerificationRequirement::Optional,
        }
    );
    assert_eq!(declare.operation(), AgentCallOperation::DeclareIntent);
    assert!(context().authenticates(declare, NONCE));
    assert!(!context().authenticates(declare, NONCE + 1));

    let create = AgentCallRequest::decode(&create_frame()).unwrap();
    assert_eq!(
        create,
        AgentCallRequest::CreateTask {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            intent: INTENT,
        }
    );
    assert_eq!(create.operation(), AgentCallOperation::CreateTask);
    assert!(context().authenticates(create, NONCE));

    let delegate = AgentCallRequest::decode(&delegate_frame()).unwrap();
    assert_eq!(
        delegate,
        AgentCallRequest::DelegateTask {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            delegated_task: MANAGED_TASK,
            target: TARGET,
        }
    );
    assert_eq!(delegate.operation(), AgentCallOperation::DelegateTask);
    assert!(context().authenticates(delegate, NONCE));
}

#[test]
fn task_manager_requests_reject_malformed_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 6,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = u64::MAX,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 3,
    ] {
        let mut frame = declare_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    let mut declare_reserved = declare_frame();
    declare_reserved.r14 = 1;
    assert_decode_error(declare_reserved, AgentCallDecodeError::ReservedNotZero);

    for mut frame in [create_frame(), delegate_frame()] {
        frame.r10 = 0;
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    let mut create_reserved = create_frame();
    create_reserved.r12 = 1;
    assert_decode_error(create_reserved, AgentCallDecodeError::ReservedNotZero);
    let mut delegate_zero_target = delegate_frame();
    delegate_zero_target.r12 = 0;
    assert_decode_error(delegate_zero_target, AgentCallDecodeError::InvalidPayload);
    let mut delegate_reserved = delegate_frame();
    delegate_reserved.r13 = 1;
    assert_decode_error(delegate_reserved, AgentCallDecodeError::ReservedNotZero);
}

#[test]
fn task_manager_replies_return_only_kernel_handles() {
    let mut declare = declare_frame();
    context()
        .encode_intent_declared_reply(&mut declare, NONCE, INTENT)
        .unwrap();
    assert_common_reply(&declare, AGENT_CALL_DECLARE_INTENT);
    assert_eq!(payload(&declare), [7, 0, 0, 0, 0, 0, 0]);

    let mut create = create_frame();
    context()
        .encode_task_created_reply(&mut create, NONCE, MANAGED_TASK)
        .unwrap();
    assert_common_reply(&create, AGENT_CALL_CREATE_TASK);
    assert_eq!(payload(&create), [7, 0, 0, 0, 0, 0, 0]);

    let mut delegate = delegate_frame();
    context()
        .encode_task_delegated_reply(
            &mut delegate,
            NONCE,
            MANAGED_TASK,
            DELEGATED_CAPABILITY,
            TARGET,
        )
        .unwrap();
    assert_common_reply(&delegate, AGENT_CALL_DELEGATE_TASK);
    assert_eq!(payload(&delegate), [7, 13, 2, 0, 0, 0, 0]);

    assert_eq!(
        context().encode_intent_declared_reply(&mut declare, NONCE, IntentId::new(0)),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    assert_eq!(
        context().encode_task_created_reply(&mut create, NONCE, TaskId::new(0)),
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

fn declare_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_DECLARE_INTENT,
        [
            AUTHORITY.raw(),
            RESOURCE.raw(),
            AGENT_CALL_INTENT_ACT,
            AGENT_CALL_VERIFICATION_OPTIONAL,
            0,
            0,
            0,
        ],
    )
}

fn create_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_CREATE_TASK,
        [AUTHORITY.raw(), INTENT.raw(), 0, 0, 0, 0, 0],
    )
}

fn delegate_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_DELEGATE_TASK,
        [
            AUTHORITY.raw(),
            MANAGED_TASK.raw(),
            TARGET.raw(),
            0,
            0,
            0,
            0,
        ],
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

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
}

fn payload(frame: &PrivilegeInterruptStackFrame) -> [u64; 7] {
    [
        frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,
    ]
}
