use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_DISCOVER_RUNTIME_ADMISSION,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66d_0001;
const TARGET: AgentId = AgentId::new(10);
const TARGET_TASK: TaskId = TaskId::new(8);

#[test]
fn runtime_admission_discovery_decodes_and_authenticates_context() {
    assert_eq!(AGENT_CALL_DISCOVER_RUNTIME_ADMISSION, 28);
    let request = AgentCallRequest::decode(&discovery_frame()).expect("discovery request decodes");

    assert_eq!(
        request,
        AgentCallRequest::DiscoverRuntimeAdmission {
            agent: TARGET,
            task: TARGET_TASK,
            image: AgentImageId::new(12),
            nonce: NONCE,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::DiscoverRuntimeAdmission
    );
    assert!(admitted_context().authenticates(request, NONCE));
    assert!(!admitted_context().authenticates(request, NONCE + 1));
}

#[test]
fn runtime_admission_discovery_rejects_noncanonical_requests() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.rsi = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.rdi = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r8 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r9 = 0,
    ] {
        let mut frame = discovery_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r13 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r14 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r15 = 1,
        |frame: &mut PrivilegeInterruptStackFrame| frame.rbp = 1,
    ] {
        let mut frame = discovery_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::ReservedNotZero);
    }
}

#[test]
fn admitted_context_returns_requester_and_regular_context_fails_closed() {
    let admitted = admitted_context();
    let regular = AgentCallContext::new(
        TARGET,
        TARGET_TASK,
        AgentImageId::new(12),
        CapabilityId::new(24),
    )
    .expect("regular context is valid");
    let mut frame = discovery_frame();

    admitted
        .encode_runtime_admission_discovery_reply(&mut frame, NONCE)
        .expect("admitted discovery reply encodes");

    assert_eq!(
        admitted.runtime_admission_requester(),
        Some(AgentId::new(42))
    );
    assert_ne!(admitted, regular);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_DISCOVER_RUNTIME_ADMISSION);
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [10, 8, 12, NONCE]
    );
    assert_eq!(payload(&frame), [42, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        regular.encode_runtime_admission_discovery_reply(&mut frame, NONCE),
        Err(AgentCallDecodeError::RuntimeAdmissionContextUnavailable)
    );
    assert!(AgentCallContext::new_admitted(
        TARGET,
        TARGET_TASK,
        AgentImageId::new(12),
        CapabilityId::new(24),
        AgentId::new(0),
    )
    .is_none());
}

fn admitted_context() -> AgentCallContext {
    AgentCallContext::new_admitted(
        TARGET,
        TARGET_TASK,
        AgentImageId::new(12),
        CapabilityId::new(24),
        AgentId::new(42),
    )
    .expect("admitted context is valid")
}

fn discovery_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: 0,
        r9: NONCE,
        r8: 12,
        rbp: 0,
        rdi: TARGET_TASK.raw(),
        rsi: TARGET.raw(),
        rdx: 0,
        rcx: AGENT_CALL_DISCOVER_RUNTIME_ADMISSION,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4000,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8000,
        user_ss: 0x1b,
    }
}

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
}

fn payload(frame: &PrivilegeInterruptStackFrame) -> [u64; 7] {
    [
        frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,
    ]
}
