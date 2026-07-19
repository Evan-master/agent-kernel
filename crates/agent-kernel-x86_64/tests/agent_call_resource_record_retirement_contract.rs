use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, Resource, ResourceId, ResourceKind, ResourceStatus, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_RESOURCE_DEVICE,
        AGENT_CALL_RESOURCE_FILE, AGENT_CALL_RESOURCE_MEMORY, AGENT_CALL_RESOURCE_NETWORK,
        AGENT_CALL_RESOURCE_PROCESS, AGENT_CALL_RESOURCE_SERVICE, AGENT_CALL_RESOURCE_WORKSPACE,
        AGENT_CALL_RETIRE_RESOURCE_RECORD, AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0005;
const AUTHORITY: CapabilityId = CapabilityId::new(23);
const TARGET: ResourceId = ResourceId::new(3);

#[test]
fn resource_record_retirement_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_RETIRE_RESOURCE_RECORD, 41);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::RetireResourceRecord {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(12),
            nonce: NONCE,
            authority: AUTHORITY,
            target: TARGET,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::RetireResourceRecord
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn resource_record_retirement_rejects_zero_and_reserved_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
    ] {
        let mut frame = request_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r14 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r15 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.rbp = 1,
    ] {
        let mut frame = request_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn resource_record_retirement_reply_is_canonical() {
    assert_eq!(
        [
            AGENT_CALL_RESOURCE_WORKSPACE,
            AGENT_CALL_RESOURCE_MEMORY,
            AGENT_CALL_RESOURCE_SERVICE,
            AGENT_CALL_RESOURCE_NETWORK,
            AGENT_CALL_RESOURCE_DEVICE,
            AGENT_CALL_RESOURCE_FILE,
            AGENT_CALL_RESOURCE_PROCESS,
        ],
        [1, 2, 3, 4, 5, 6, 7]
    );
    let record = Resource {
        id: TARGET,
        kind: ResourceKind::Service,
        parent: Some(ResourceId::new(1)),
        owner: Some(AgentId::new(2)),
        status: ResourceStatus::Retired,
    };
    let mut frame = request_frame();
    let control = control_words(&frame);
    context()
        .encode_resource_record_retirement_reply(&mut frame, NONCE, record)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(payload(&frame), [3, 3, 1, 2, 0, 0, 0]);
    assert_eq!(control_words(&frame), control);

    for (kind, code) in [
        (ResourceKind::Workspace, 1),
        (ResourceKind::Memory, 2),
        (ResourceKind::Service, 3),
        (ResourceKind::Network, 4),
        (ResourceKind::Device, 5),
        (ResourceKind::File, 6),
        (ResourceKind::Process, 7),
    ] {
        let mut frame = request_frame();
        let record = Resource {
            id: TARGET,
            kind,
            parent: None,
            owner: None,
            status: ResourceStatus::Retired,
        };
        context()
            .encode_resource_record_retirement_reply(&mut frame, NONCE, record)
            .expect("retired resource encodes");
        assert_eq!(payload(&frame), [TARGET.raw(), code, 0, 0, 0, 0, 0]);
    }

    for record in [
        Resource {
            id: ResourceId::new(0),
            ..record
        },
        Resource {
            status: ResourceStatus::Active,
            ..record
        },
    ] {
        assert_eq!(
            context().encode_resource_record_retirement_reply(&mut frame, NONCE, record),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(12),
        TaskId::new(10),
        AgentImageId::new(12),
        CapabilityId::new(22),
    )
    .expect("context is valid")
}

fn request_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: TARGET.raw(),
        r10: AUTHORITY.raw(),
        r9: NONCE,
        r8: 12,
        rbp: 0,
        rdi: 10,
        rsi: 12,
        rdx: 0,
        rcx: AGENT_CALL_RETIRE_RESOURCE_RECORD,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_RETIRE_RESOURCE_RECORD);
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [12, 10, 12, NONCE]
    );
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
