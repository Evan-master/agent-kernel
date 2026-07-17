use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, Operation, OperationSet, ResourceCreateOutcome,
    ResourceId, ResourceKind, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_CREATE_RESOURCE,
        AGENT_CALL_RESOURCE_DEVICE, AGENT_CALL_RESOURCE_MEMORY, AGENT_CALL_RESOURCE_NETWORK,
        AGENT_CALL_RESOURCE_SERVICE, AGENT_CALL_RESOURCE_WORKSPACE, AGENT_CALL_RETIRE_RESOURCE,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa11c_e006;
const AUTHORITY: CapabilityId = CapabilityId::new(10);
const PARENT: ResourceId = ResourceId::new(1);
const CHILD: ResourceId = ResourceId::new(2);
const CHILD_CAPABILITY: CapabilityId = CapabilityId::new(11);

#[test]
fn resource_requests_decode_and_authenticate_against_trusted_context() {
    assert_eq!(AGENT_CALL_CREATE_RESOURCE, 10);
    assert_eq!(AGENT_CALL_RETIRE_RESOURCE, 11);
    assert_eq!(
        [
            AGENT_CALL_RESOURCE_WORKSPACE,
            AGENT_CALL_RESOURCE_MEMORY,
            AGENT_CALL_RESOURCE_SERVICE,
            AGENT_CALL_RESOURCE_NETWORK,
            AGENT_CALL_RESOURCE_DEVICE,
        ],
        [1, 2, 3, 4, 5]
    );

    let operations = child_operations();
    let create = AgentCallRequest::decode(&create_frame()).unwrap();
    assert_eq!(
        create,
        AgentCallRequest::CreateResource {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            authority: AUTHORITY,
            parent: PARENT,
            kind: ResourceKind::Service,
            operations,
        }
    );
    assert_eq!(create.operation(), AgentCallOperation::CreateResource);
    assert!(context().authenticates(create, NONCE));
    assert!(!context().authenticates(create, NONCE + 1));

    let retire = AgentCallRequest::decode(&retire_frame()).unwrap();
    assert_eq!(
        retire,
        AgentCallRequest::RetireResource {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            resource: CHILD,
            capability: CHILD_CAPABILITY,
        }
    );
    assert_eq!(retire.operation(), AgentCallOperation::RetireResource);
    assert!(context().authenticates(retire, NONCE));
}

#[test]
fn resource_requests_reject_zero_unknown_and_reserved_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 6,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 1 << 6,
    ] {
        let mut frame = create_frame();
        mutate(&mut frame);
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }

    let mut create_reserved = create_frame();
    create_reserved.r14 = 1;
    assert_eq!(
        AgentCallRequest::decode(&create_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );

    for clear in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
    ] {
        let mut frame = retire_frame();
        clear(&mut frame);
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
    let mut retire_reserved = retire_frame();
    retire_reserved.r12 = 1;
    assert_eq!(
        AgentCallRequest::decode(&retire_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn resource_replies_preserve_context_and_return_kernel_handles() {
    let mut create = create_frame();
    let create_control = control_words(&create);
    context()
        .encode_resource_created_reply(
            &mut create,
            NONCE,
            ResourceCreateOutcome {
                resource: CHILD,
                capability: CHILD_CAPABILITY,
            },
        )
        .unwrap();
    assert_common_reply(&create, AGENT_CALL_CREATE_RESOURCE);
    assert_eq!([create.r10, create.r11], [2, 11]);
    assert_eq!(
        [create.r12, create.r13, create.r14, create.r15, create.rbp],
        [0; 5]
    );
    assert_eq!(control_words(&create), create_control);

    let mut retire = retire_frame();
    let retire_control = control_words(&retire);
    context()
        .encode_resource_retired_reply(&mut retire, NONCE, CHILD, CHILD_CAPABILITY)
        .unwrap();
    assert_common_reply(&retire, AGENT_CALL_RETIRE_RESOURCE);
    assert_eq!([retire.r10, retire.r11], [2, 11]);
    assert_eq!(
        [retire.r12, retire.r13, retire.r14, retire.r15, retire.rbp],
        [0; 5]
    );
    assert_eq!(control_words(&retire), retire_control);
}

fn child_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
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

fn create_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_CREATE_RESOURCE,
        [
            AUTHORITY.raw(),
            PARENT.raw(),
            AGENT_CALL_RESOURCE_SERVICE,
            u64::from(child_operations().bits()),
            0,
            0,
            0,
        ],
    )
}

fn retire_frame() -> PrivilegeInterruptStackFrame {
    request_frame(
        AGENT_CALL_RETIRE_RESOURCE,
        [CHILD.raw(), CHILD_CAPABILITY.raw(), 0, 0, 0, 0, 0],
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
