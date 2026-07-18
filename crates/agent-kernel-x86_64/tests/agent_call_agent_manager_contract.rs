use agent_kernel_core::{AgentId, AgentImageId, AgentStatus, CapabilityId, ResourceId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_AGENT_ACTIVE,
        AGENT_CALL_AGENT_RETIRED, AGENT_CALL_AGENT_SUSPENDED, AGENT_CALL_REGISTER_MANAGED_AGENT,
        AGENT_CALL_RESUME_MANAGED_AGENT, AGENT_CALL_RETIRE_MANAGED_AGENT, AGENT_CALL_STATUS_OK,
        AGENT_CALL_SUSPEND_MANAGED_AGENT,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xf66c_e006;
const AUTHORITY: CapabilityId = CapabilityId::new(10);
const RESOURCE: ResourceId = ResourceId::new(1);
const TARGET: AgentId = AgentId::new(9);

#[test]
fn agent_manager_requests_decode_and_authenticate() {
    assert_eq!(
        [
            AGENT_CALL_REGISTER_MANAGED_AGENT,
            AGENT_CALL_SUSPEND_MANAGED_AGENT,
            AGENT_CALL_RESUME_MANAGED_AGENT,
            AGENT_CALL_RETIRE_MANAGED_AGENT,
        ],
        [17, 18, 19, 20]
    );

    let register = AgentCallRequest::decode(&register_frame()).unwrap();
    assert_eq!(
        register,
        AgentCallRequest::RegisterManagedAgent {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            resource: RESOURCE,
            target: TARGET,
        }
    );
    assert_eq!(
        register.operation(),
        AgentCallOperation::RegisterManagedAgent
    );
    assert!(context().authenticates(register, NONCE));
    assert!(!context().authenticates(register, NONCE + 1));

    for (operation, expected) in [
        (
            AGENT_CALL_SUSPEND_MANAGED_AGENT,
            AgentCallOperation::SuspendManagedAgent,
        ),
        (
            AGENT_CALL_RESUME_MANAGED_AGENT,
            AgentCallOperation::ResumeManagedAgent,
        ),
        (
            AGENT_CALL_RETIRE_MANAGED_AGENT,
            AgentCallOperation::RetireManagedAgent,
        ),
    ] {
        let request = AgentCallRequest::decode(&lifecycle_frame(operation)).unwrap();
        assert_eq!(request.operation(), expected);
        assert!(context().authenticates(request, NONCE));
    }
}

#[test]
fn agent_manager_requests_reject_zero_and_reserved_payloads() {
    for register in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
    ] {
        let mut frame = register_frame();
        register(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    let mut register_reserved = register_frame();
    register_reserved.r13 = 1;
    assert_decode_error(register_reserved, AgentCallDecodeError::ReservedNotZero);

    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
    ] {
        let mut frame = lifecycle_frame(AGENT_CALL_SUSPEND_MANAGED_AGENT);
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    let mut lifecycle_reserved = lifecycle_frame(AGENT_CALL_RETIRE_MANAGED_AGENT);
    lifecycle_reserved.r12 = 1;
    assert_decode_error(lifecycle_reserved, AgentCallDecodeError::ReservedNotZero);
}

#[test]
fn agent_manager_replies_return_target_resource_and_status() {
    assert_eq!(
        [
            AGENT_CALL_AGENT_ACTIVE,
            AGENT_CALL_AGENT_SUSPENDED,
            AGENT_CALL_AGENT_RETIRED,
        ],
        [1, 2, 3]
    );
    let cases = [
        (
            AGENT_CALL_REGISTER_MANAGED_AGENT,
            AgentStatus::Active,
            AGENT_CALL_AGENT_ACTIVE,
        ),
        (
            AGENT_CALL_SUSPEND_MANAGED_AGENT,
            AgentStatus::Suspended,
            AGENT_CALL_AGENT_SUSPENDED,
        ),
        (
            AGENT_CALL_RESUME_MANAGED_AGENT,
            AgentStatus::Active,
            AGENT_CALL_AGENT_ACTIVE,
        ),
        (
            AGENT_CALL_RETIRE_MANAGED_AGENT,
            AgentStatus::Retired,
            AGENT_CALL_AGENT_RETIRED,
        ),
    ];

    for (operation, status, status_code) in cases {
        let mut frame = if operation == AGENT_CALL_REGISTER_MANAGED_AGENT {
            register_frame()
        } else {
            lifecycle_frame(operation)
        };
        context()
            .encode_agent_management_reply(&mut frame, NONCE, operation, TARGET, RESOURCE, status)
            .unwrap();
        assert_common_reply(&frame, operation);
        assert_eq!(payload(&frame), [9, 1, status_code, 0, 0, 0, 0]);
    }

    let mut frame = register_frame();
    assert_eq!(
        context().encode_agent_management_reply(
            &mut frame,
            NONCE,
            AGENT_CALL_REGISTER_MANAGED_AGENT,
            AgentId::new(0),
            RESOURCE,
            AgentStatus::Active,
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

fn register_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_REGISTER_MANAGED_AGENT,
        [AUTHORITY.raw(), RESOURCE.raw(), TARGET.raw(), 0, 0, 0, 0],
    )
}

fn lifecycle_frame(operation: u64) -> PrivilegeInterruptStackFrame {
    request_frame(operation, [AUTHORITY.raw(), TARGET.raw(), 0, 0, 0, 0, 0])
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
