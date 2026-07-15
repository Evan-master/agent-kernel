use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId, TaskResult};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_COMPLETE_TASK,
        AGENT_CALL_STATUS_OK, AGENT_CALL_SUBMIT_TASK_RESULT,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa11c_e001;
const RESULT: TaskResult = TaskResult {
    code: 0x0a01,
    value: 0xa11c_0001,
};

#[test]
fn submit_task_result_decodes_fixed_result_and_matches_trusted_context() {
    assert_eq!(AGENT_CALL_SUBMIT_TASK_RESULT, 4);
    let request = AgentCallRequest::decode(&result_frame(3, 9, 4, NONCE, RESULT)).unwrap();

    assert_eq!(
        request,
        AgentCallRequest::SubmitTaskResult {
            agent: AgentId::new(3),
            task: TaskId::new(9),
            image: AgentImageId::new(4),
            nonce: NONCE,
            result: RESULT,
        }
    );
    assert_eq!(request.operation(), AgentCallOperation::SubmitTaskResult);
    assert_eq!(context().match_task_result(request, NONCE), Some(RESULT));
    assert_eq!(context().match_task_result(request, 0), None);

    let wrong = AgentCallRequest::decode(&result_frame(3, 10, 4, NONCE, RESULT)).unwrap();
    assert_eq!(context().match_task_result(wrong, NONCE), None);
}

#[test]
fn result_registers_are_operation_specific_and_canonical() {
    let mut oversized_code = result_frame(3, 9, 4, NONCE, RESULT);
    oversized_code.r10 = u64::from(u16::MAX) + 1;
    assert_eq!(
        AgentCallRequest::decode(&oversized_code),
        Err(AgentCallDecodeError::InvalidPayload)
    );

    let mut missing_identity = result_frame(3, 9, 4, NONCE, RESULT);
    missing_identity.rsi = 0;
    assert_eq!(
        AgentCallRequest::decode(&missing_identity),
        Err(AgentCallDecodeError::InvalidPayload)
    );

    let mut legacy = request_frame(AGENT_CALL_COMPLETE_TASK, [3, 9, 4, NONCE]);
    legacy.r10 = 1;
    assert_eq!(
        AgentCallRequest::decode(&legacy),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn result_success_reply_returns_trusted_context_and_clears_result_registers() {
    let mut frame = result_frame(3, 9, 4, NONCE, RESULT);
    let control = (
        frame.rip,
        frame.cs,
        frame.rflags,
        frame.user_rsp,
        frame.user_ss,
    );

    context()
        .encode_task_result_reply(&mut frame, NONCE)
        .unwrap();

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_SUBMIT_TASK_RESULT);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [3, 9, 4, NONCE]);
    assert_eq!([frame.r10, frame.r11], [0, 0]);
    assert_eq!(
        (
            frame.rip,
            frame.cs,
            frame.rflags,
            frame.user_rsp,
            frame.user_ss,
        ),
        control
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(3),
        TaskId::new(9),
        AgentImageId::new(4),
        CapabilityId::new(7),
    )
    .unwrap()
}

fn result_frame(
    agent: u64,
    task: u64,
    image: u64,
    nonce: u64,
    result: TaskResult,
) -> PrivilegeInterruptStackFrame {
    let mut frame = request_frame(AGENT_CALL_SUBMIT_TASK_RESULT, [agent, task, image, nonce]);
    frame.r10 = u64::from(result.code);
    frame.r11 = result.value;
    frame
}

fn request_frame(operation: u64, payload: [u64; 4]) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: 0,
        r9: payload[3],
        r8: payload[2],
        rbp: 0,
        rdi: payload[1],
        rsi: payload[0],
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
