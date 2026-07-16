use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, TaskId, TaskResult};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_INSPECT_TASK_RESULT,
        AGENT_CALL_STATUS_OK, AGENT_CALL_VERIFY_TASK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xc33c_e003;
const TARGET: TaskId = TaskId::new(1);
const RESULT: TaskResult = TaskResult {
    code: 0x0a01,
    value: 0xa11c_0001,
};

#[test]
fn verifier_requests_decode_and_match_one_trusted_target() {
    assert_eq!(AGENT_CALL_INSPECT_TASK_RESULT, 5);
    assert_eq!(AGENT_CALL_VERIFY_TASK, 6);
    let inspect = AgentCallRequest::decode(&verifier_frame(5, TARGET.raw())).unwrap();
    let verify = AgentCallRequest::decode(&verifier_frame(6, TARGET.raw())).unwrap();

    assert_eq!(
        inspect,
        AgentCallRequest::InspectTaskResult {
            agent: AgentId::new(5),
            task: TaskId::new(3),
            image: AgentImageId::new(5),
            nonce: NONCE,
            target_task: TARGET,
        }
    );
    assert_eq!(inspect.operation(), AgentCallOperation::InspectTaskResult);
    assert_eq!(verify.operation(), AgentCallOperation::VerifyTask);
    assert!(context().matches_task_result_inspection(inspect, NONCE, TARGET));
    assert!(context().matches_task_verification(verify, NONCE, TARGET));
    assert!(!context().matches_task_result_inspection(inspect, NONCE, TaskId::new(2)));
    assert!(!context().matches_task_verification(verify, 0, TARGET));
}

#[test]
fn verifier_requests_require_nonzero_target_and_zero_reserved_value() {
    assert_eq!(
        AgentCallRequest::decode(&verifier_frame(5, 0)),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    let mut reserved = verifier_frame(6, TARGET.raw());
    reserved.r11 = 1;
    assert_eq!(
        AgentCallRequest::decode(&reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn inspection_and_verification_replies_preserve_context_and_bound_result() {
    let mut inspect = verifier_frame(5, TARGET.raw());
    let control = control_words(&inspect);
    context()
        .encode_task_result_inspection_reply(&mut inspect, NONCE, RESULT)
        .unwrap();
    assert_eq!(inspect.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(inspect.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(inspect.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(inspect.rdx, AGENT_CALL_INSPECT_TASK_RESULT);
    assert_eq!(
        [inspect.rsi, inspect.rdi, inspect.r8, inspect.r9],
        [5, 3, 5, NONCE]
    );
    assert_eq!(inspect.r10, u64::from(RESULT.code));
    assert_eq!(inspect.r11, RESULT.value);
    assert_eq!(control_words(&inspect), control);

    let mut verify = verifier_frame(6, TARGET.raw());
    context()
        .encode_task_verification_reply(&mut verify, NONCE)
        .unwrap();
    assert_eq!(verify.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(verify.rdx, AGENT_CALL_VERIFY_TASK);
    assert_eq!(
        [verify.rsi, verify.rdi, verify.r8, verify.r9],
        [5, 3, 5, NONCE]
    );
    assert_eq!([verify.r10, verify.r11], [0, 0]);
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(5),
        TaskId::new(3),
        AgentImageId::new(5),
        CapabilityId::new(5),
    )
    .unwrap()
}

fn verifier_frame(operation: u64, target: u64) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: target,
        r9: NONCE,
        r8: 5,
        rbp: 0,
        rdi: 3,
        rsi: 5,
        rdx: 0,
        rcx: operation,
        rbx: AGENT_CALL_ABI_VERSION,
        rax: AGENT_CALL_ABI_MAGIC,
        rip: 0x4040,
        cs: 0x23,
        rflags: 0x202,
        user_rsp: 0x8080,
        user_ss: 0x1b,
    }
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
