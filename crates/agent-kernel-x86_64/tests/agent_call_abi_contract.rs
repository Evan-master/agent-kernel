use agent_kernel_core::{AgentId, AgentImageId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_DESCRIBE_CONTEXT,
        AGENT_CALL_STATUS_OK, AGENT_CALL_YIELD,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa11c_e001;

#[test]
fn call_abi_constants_are_native_and_versioned() {
    assert_eq!(AGENT_CALL_ABI_MAGIC, u64::from_le_bytes(*b"AGNTCALL"));
    assert_eq!(AGENT_CALL_ABI_VERSION, 1);
    assert_eq!(AGENT_CALL_DESCRIBE_CONTEXT, 1);
    assert_eq!(AGENT_CALL_YIELD, 2);
    assert_eq!(AGENT_CALL_STATUS_OK, 0);
}

#[test]
fn describe_context_request_decodes_one_nonce_without_identity_claims() {
    let frame = request_frame(AGENT_CALL_DESCRIBE_CONTEXT, [NONCE, 0, 0, 0]);

    assert_eq!(
        AgentCallRequest::decode(&frame),
        Ok(AgentCallRequest::DescribeContext { nonce: NONCE })
    );
    assert_eq!(
        AgentCallRequest::decode(&frame).unwrap().operation(),
        AgentCallOperation::DescribeContext
    );
}

#[test]
fn yield_request_decodes_context_echo() {
    let frame = request_frame(AGENT_CALL_YIELD, [3, 9, 4, NONCE]);

    assert_eq!(
        AgentCallRequest::decode(&frame),
        Ok(AgentCallRequest::Yield {
            agent: AgentId::new(3),
            task: TaskId::new(9),
            image: AgentImageId::new(4),
            nonce: NONCE,
        })
    );
}

#[test]
fn call_request_rejects_unknown_common_header_fields() {
    let valid = request_frame(AGENT_CALL_DESCRIBE_CONTEXT, [NONCE, 0, 0, 0]);
    let mut bad_magic = valid;
    bad_magic.rax ^= 1;
    let mut bad_version = valid;
    bad_version.rbx = 2;
    let mut bad_operation = valid;
    bad_operation.rcx = 99;
    let mut bad_flags = valid;
    bad_flags.rdx = 1;
    let mut bad_r10 = valid;
    bad_r10.r10 = 1;
    let mut bad_r11 = valid;
    bad_r11.r11 = 1;

    let cases = [
        (bad_magic, AgentCallDecodeError::InvalidMagic),
        (bad_version, AgentCallDecodeError::UnsupportedVersion),
        (bad_operation, AgentCallDecodeError::UnsupportedOperation),
        (bad_flags, AgentCallDecodeError::UnsupportedFlags),
        (bad_r10, AgentCallDecodeError::ReservedNotZero),
        (bad_r11, AgentCallDecodeError::ReservedNotZero),
    ];
    for (frame, expected) in cases {
        assert_eq!(AgentCallRequest::decode(&frame), Err(expected));
    }
}

#[test]
fn operations_reject_noncanonical_payloads() {
    for payload in [
        [0, 0, 0, 0],
        [NONCE, 1, 0, 0],
        [NONCE, 0, 1, 0],
        [NONCE, 0, 0, 1],
    ] {
        assert_eq!(
            AgentCallRequest::decode(&request_frame(AGENT_CALL_DESCRIBE_CONTEXT, payload)),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
    for payload in [
        [0, 9, 4, NONCE],
        [3, 0, 4, NONCE],
        [3, 9, 0, NONCE],
        [3, 9, 4, 0],
    ] {
        assert_eq!(
            AgentCallRequest::decode(&request_frame(AGENT_CALL_YIELD, payload)),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

#[test]
fn describe_reply_encodes_trusted_context_without_changing_control_frame() {
    let context = context();
    let mut frame = request_frame(AGENT_CALL_DESCRIBE_CONTEXT, [NONCE, 0, 0, 0]);
    let control = (
        frame.rip,
        frame.cs,
        frame.rflags,
        frame.user_rsp,
        frame.user_ss,
    );

    context.encode_describe_reply(&mut frame, NONCE).unwrap();

    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, AGENT_CALL_DESCRIBE_CONTEXT);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], [3, 9, 4, NONCE]);
    assert_eq!([frame.r10, frame.r11], [0, 0]);
    assert_eq!(
        (
            frame.rip,
            frame.cs,
            frame.rflags,
            frame.user_rsp,
            frame.user_ss
        ),
        control
    );
    assert_eq!(
        context.encode_describe_reply(&mut frame, 0),
        Err(AgentCallDecodeError::InvalidPayload)
    );
}

#[test]
fn trusted_call_context_rejects_zero_identifiers() {
    assert_eq!(
        AgentCallContext::new(AgentId::new(0), TaskId::new(9), AgentImageId::new(4)),
        None
    );
    assert_eq!(
        AgentCallContext::new(AgentId::new(3), TaskId::new(0), AgentImageId::new(4)),
        None
    );
    assert_eq!(
        AgentCallContext::new(AgentId::new(3), TaskId::new(9), AgentImageId::new(0)),
        None
    );
}

#[test]
fn yield_context_must_match_the_immediately_returned_identity_and_nonce() {
    let context = context();
    let describe = AgentCallRequest::decode(&request_frame(
        AGENT_CALL_DESCRIBE_CONTEXT,
        [NONCE, 0, 0, 0],
    ))
    .unwrap();
    assert!(!context.matches_yield(describe, NONCE));
    let request =
        AgentCallRequest::decode(&request_frame(AGENT_CALL_YIELD, [3, 9, 4, NONCE])).unwrap();
    assert!(context.matches_yield(request, NONCE));

    for payload in [
        [5, 9, 4, NONCE],
        [3, 10, 4, NONCE],
        [3, 9, 6, NONCE],
        [3, 9, 4, NONCE + 1],
    ] {
        let request = AgentCallRequest::decode(&request_frame(AGENT_CALL_YIELD, payload)).unwrap();
        assert!(!context.matches_yield(request, NONCE));
    }
}

fn context() -> AgentCallContext {
    AgentCallContext::new(AgentId::new(3), TaskId::new(9), AgentImageId::new(4)).unwrap()
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
