use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, RuntimeAdmissionId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_REQUEST_RUNTIME_ADMISSION,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66d_0001;
const AUTHORITY: CapabilityId = CapabilityId::new(23);
const TARGET: AgentId = AgentId::new(10);
const TARGET_TASK: TaskId = TaskId::new(8);
const ADMISSION: RuntimeAdmissionId = RuntimeAdmissionId::new(1);

#[test]
fn runtime_admission_request_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_REQUEST_RUNTIME_ADMISSION, 27);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::RequestRuntimeAdmission {
            agent: AgentId::new(12),
            task: TaskId::new(10),
            image: AgentImageId::new(11),
            nonce: NONCE,
            authority: AUTHORITY,
            target: TARGET,
            target_task: TARGET_TASK,
        }
    );
    assert_eq!(
        request.operation(),
        AgentCallOperation::RequestRuntimeAdmission
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn runtime_admission_request_rejects_zero_and_reserved_payloads() {
    for mutate in [
        |frame: &mut PrivilegeInterruptStackFrame| frame.r10 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r11 = 0,
        |frame: &mut PrivilegeInterruptStackFrame| frame.r12 = 0,
    ] {
        let mut frame = request_frame();
        mutate(&mut frame);
        assert_decode_error(frame, AgentCallDecodeError::InvalidPayload);
    }
    for mutate in [
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
fn runtime_admission_reply_returns_only_kernel_issued_identity() {
    let mut frame = request_frame();
    context()
        .encode_runtime_admission_reply(&mut frame, NONCE, ADMISSION, TARGET, TARGET_TASK)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(payload(&frame), [1, 10, 8, 0, 0, 0, 0]);
    assert_eq!(
        context().encode_runtime_admission_reply(
            &mut frame,
            NONCE,
            RuntimeAdmissionId::new(0),
            TARGET,
            TARGET_TASK,
        ),
        Err(AgentCallDecodeError::InvalidPayload)
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(12),
        TaskId::new(10),
        AgentImageId::new(11),
        CapabilityId::new(22),
    )
    .expect("context is valid")
}

fn request_frame() -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: TARGET_TASK.raw(),
        r11: TARGET.raw(),
        r10: AUTHORITY.raw(),
        r9: NONCE,
        r8: 11,
        rbp: 0,
        rdi: 10,
        rsi: 12,
        rdx: 0,
        rcx: AGENT_CALL_REQUEST_RUNTIME_ADMISSION,
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
    assert_eq!(frame.rdx, AGENT_CALL_REQUEST_RUNTIME_ADMISSION);
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [12, 10, 11, NONCE]
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
