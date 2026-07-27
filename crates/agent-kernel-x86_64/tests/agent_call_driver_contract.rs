use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, DeviceEventId, DeviceEventKind, DeviceEventPayload,
    DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload, DriverCommandResult,
    DriverInvocationId, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT,
        AGENT_CALL_COMPLETE_DRIVER_INVOCATION, AGENT_CALL_CONTEXT_DRIVER,
        AGENT_CALL_DRIVER_COMMAND_CONFIGURE, AGENT_CALL_DRIVER_COMMAND_WRITE,
        AGENT_CALL_EVENT_STATE_CHANGED, AGENT_CALL_INSPECT_DRIVER_INVOCATION, AGENT_CALL_STATUS_OK,
        AGENT_CALL_SUBMIT_DRIVER_COMMAND,
    },
    context::PrivilegeInterruptStackFrame,
};

const AGENT: AgentId = AgentId::new(10);
const INVOCATION: DriverInvocationId = DriverInvocationId::new(2);
const IMAGE: AgentImageId = AgentImageId::new(15);
const CAPABILITY: CapabilityId = CapabilityId::new(27);
const EVENT: DeviceEventId = DeviceEventId::new(2);
const RESOURCE: ResourceId = ResourceId::new(10);
const BINDING: DriverBindingId = DriverBindingId::new(2);
const COMMAND: DriverCommandId = DriverCommandId::new(2);
const NONCE: u64 = 0xd81c_e024;
const EVENT_PAYLOAD: DeviceEventPayload = DeviceEventPayload {
    code: 0x1b36,
    value: 0x0002,
};
const COMMAND_PAYLOAD: DriverCommandPayload = DriverCommandPayload {
    opcode: 0,
    value: 0x50,
};
const COMMAND_RESULT: DriverCommandResult = DriverCommandResult {
    code: 0,
    value: 0x50,
};

#[test]
fn driver_operation_numbers_extend_abi_v1_without_renumbering_tasks() {
    assert_eq!(AGENT_CALL_INSPECT_DRIVER_INVOCATION, 57);
    assert_eq!(AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT, 58);
    assert_eq!(AGENT_CALL_SUBMIT_DRIVER_COMMAND, 59);
    assert_eq!(AGENT_CALL_COMPLETE_DRIVER_INVOCATION, 60);
    assert_eq!(AGENT_CALL_CONTEXT_DRIVER, 1);
    assert_eq!(AGENT_CALL_DRIVER_COMMAND_WRITE, 1);
    assert_eq!(AGENT_CALL_DRIVER_COMMAND_CONFIGURE, 2);
}

#[test]
fn driver_context_is_mutually_exclusive_with_task_context() {
    let driver = context();
    assert_eq!(driver.agent(), AGENT);
    assert_eq!(driver.task(), TaskId::new(0));
    assert_eq!(driver.driver_invocation(), Some(INVOCATION));
    assert_eq!(driver.image(), IMAGE);
    assert_eq!(driver.capability(), CAPABILITY);

    let task = AgentCallContext::new(AGENT, TaskId::new(2), IMAGE, CAPABILITY).unwrap();
    assert_eq!(task.driver_invocation(), None);
    assert_ne!(task, driver);

    assert_eq!(
        AgentCallContext::new_driver(AgentId::new(0), INVOCATION, IMAGE, CAPABILITY),
        None
    );
    assert_eq!(
        AgentCallContext::new_driver(AGENT, DriverInvocationId::new(0), IMAGE, CAPABILITY),
        None
    );
    assert_eq!(
        AgentCallContext::new_driver(AGENT, INVOCATION, AgentImageId::new(0), CAPABILITY),
        None
    );
    assert_eq!(
        AgentCallContext::new_driver(AGENT, INVOCATION, IMAGE, CapabilityId::new(0)),
        None
    );
}

#[test]
fn driver_requests_decode_and_authenticate_only_in_driver_scope() {
    let cases = [
        (
            driver_frame(AGENT_CALL_INSPECT_DRIVER_INVOCATION, [0; 7]),
            AgentCallRequest::InspectDriverInvocation {
                agent: AGENT,
                invocation: INVOCATION,
                image: IMAGE,
                nonce: NONCE,
            },
            AgentCallOperation::InspectDriverInvocation,
        ),
        (
            driver_frame(
                AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT,
                [EVENT.raw(), 0, 0, 0, 0, 0, 0],
            ),
            AgentCallRequest::AcknowledgeDeviceEvent {
                agent: AGENT,
                invocation: INVOCATION,
                image: IMAGE,
                nonce: NONCE,
                event: EVENT,
            },
            AgentCallOperation::AcknowledgeDeviceEvent,
        ),
        (
            driver_frame(
                AGENT_CALL_SUBMIT_DRIVER_COMMAND,
                [
                    EVENT.raw(),
                    AGENT_CALL_DRIVER_COMMAND_WRITE,
                    u64::from(COMMAND_PAYLOAD.opcode),
                    COMMAND_PAYLOAD.value,
                    0,
                    0,
                    0,
                ],
            ),
            AgentCallRequest::SubmitDriverCommand {
                agent: AGENT,
                invocation: INVOCATION,
                image: IMAGE,
                nonce: NONCE,
                event: EVENT,
                kind: DriverCommandKind::Write,
                payload: COMMAND_PAYLOAD,
            },
            AgentCallOperation::SubmitDriverCommand,
        ),
        (
            driver_frame(
                AGENT_CALL_SUBMIT_DRIVER_COMMAND,
                [
                    EVENT.raw(),
                    AGENT_CALL_DRIVER_COMMAND_CONFIGURE,
                    1,
                    0,
                    0,
                    0,
                    0,
                ],
            ),
            AgentCallRequest::SubmitDriverCommand {
                agent: AGENT,
                invocation: INVOCATION,
                image: IMAGE,
                nonce: NONCE,
                event: EVENT,
                kind: DriverCommandKind::Configure,
                payload: DriverCommandPayload {
                    opcode: 1,
                    value: 0,
                },
            },
            AgentCallOperation::SubmitDriverCommand,
        ),
        (
            driver_frame(AGENT_CALL_COMPLETE_DRIVER_INVOCATION, [0; 7]),
            AgentCallRequest::CompleteDriverInvocation {
                agent: AGENT,
                invocation: INVOCATION,
                image: IMAGE,
                nonce: NONCE,
            },
            AgentCallOperation::CompleteDriverInvocation,
        ),
    ];

    let task = AgentCallContext::new(AGENT, TaskId::new(2), IMAGE, CAPABILITY).unwrap();
    for (frame, expected, operation) in cases {
        let request = AgentCallRequest::decode(&frame).expect("Driver request decodes");
        assert_eq!(request, expected);
        assert_eq!(request.operation(), operation);
        assert!(context().authenticates(request, NONCE));
        assert!(!context().authenticates(request, NONCE + 1));
        assert!(!task.authenticates(request, NONCE));
    }

    let task_request = AgentCallRequest::Yield {
        agent: AGENT,
        task: TaskId::new(INVOCATION.raw()),
        image: IMAGE,
        nonce: NONCE,
    };
    assert!(!context().authenticates(task_request, NONCE));
}

#[test]
fn driver_requests_reject_zero_identity_invalid_kind_and_reserved_registers() {
    for field in 0..4 {
        let mut frame = driver_frame(AGENT_CALL_INSPECT_DRIVER_INVOCATION, [0; 7]);
        match field {
            0 => frame.rsi = 0,
            1 => frame.rdi = 0,
            2 => frame.r8 = 0,
            _ => frame.r9 = 0,
        }
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }

    for operation in [
        AGENT_CALL_INSPECT_DRIVER_INVOCATION,
        AGENT_CALL_COMPLETE_DRIVER_INVOCATION,
    ] {
        for index in 0..7 {
            let mut payload = [0; 7];
            payload[index] = 1;
            assert_eq!(
                AgentCallRequest::decode(&driver_frame(operation, payload)),
                Err(AgentCallDecodeError::ReservedNotZero)
            );
        }
    }

    let bad_kind = driver_frame(
        AGENT_CALL_SUBMIT_DRIVER_COMMAND,
        [EVENT.raw(), 99, 0, 0x50, 0, 0, 0],
    );
    assert_eq!(
        AgentCallRequest::decode(&bad_kind),
        Err(AgentCallDecodeError::InvalidPayload)
    );

    for index in 1..7 {
        let mut payload = [EVENT.raw(), 0, 0, 0, 0, 0, 0];
        payload[index] = 1;
        assert_eq!(
            AgentCallRequest::decode(&driver_frame(AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT, payload,)),
            Err(AgentCallDecodeError::ReservedNotZero)
        );
    }
}

#[test]
fn driver_describe_and_operation_replies_are_canonical() {
    let mut describe = driver_frame(1, [0; 7]);
    context()
        .encode_describe_reply(&mut describe, NONCE)
        .expect("Driver describe reply encodes");
    assert_common_reply(&describe, 1);
    assert_eq!(
        [
            describe.r10,
            describe.r11,
            describe.r12,
            describe.r13,
            describe.r14,
            describe.r15,
            describe.rbp,
        ],
        [AGENT_CALL_CONTEXT_DRIVER, 0, 0, 0, 0, 0, 0]
    );

    let mut inspect = driver_frame(AGENT_CALL_INSPECT_DRIVER_INVOCATION, [0; 7]);
    context()
        .encode_driver_invocation_reply(
            &mut inspect,
            NONCE,
            EVENT,
            RESOURCE,
            BINDING,
            DeviceEventKind::StateChanged,
            EVENT_PAYLOAD,
        )
        .expect("Driver Invocation reply encodes");
    assert_common_reply(&inspect, AGENT_CALL_INSPECT_DRIVER_INVOCATION);
    assert_eq!(
        [
            inspect.r10,
            inspect.r11,
            inspect.r12,
            inspect.r13,
            inspect.r14,
            inspect.r15,
            inspect.rbp,
        ],
        [
            EVENT.raw(),
            RESOURCE.raw(),
            BINDING.raw(),
            AGENT_CALL_EVENT_STATE_CHANGED,
            u64::from(EVENT_PAYLOAD.code),
            EVENT_PAYLOAD.value,
            0,
        ]
    );

    let mut acknowledged = driver_frame(
        AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT,
        [EVENT.raw(), 0, 0, 0, 0, 0, 0],
    );
    context()
        .encode_device_event_acknowledgement_reply(&mut acknowledged, NONCE, EVENT)
        .expect("Device Event reply encodes");
    assert_common_reply(&acknowledged, AGENT_CALL_ACKNOWLEDGE_DEVICE_EVENT);
    assert_eq!(extended(&acknowledged), [EVENT.raw(), 0, 0, 0, 0, 0, 0]);

    let mut command = driver_frame(
        AGENT_CALL_SUBMIT_DRIVER_COMMAND,
        [
            EVENT.raw(),
            AGENT_CALL_DRIVER_COMMAND_WRITE,
            0,
            0x50,
            0,
            0,
            0,
        ],
    );
    context()
        .encode_driver_command_reply(&mut command, NONCE, COMMAND, COMMAND_RESULT)
        .expect("Driver Command reply encodes");
    assert_common_reply(&command, AGENT_CALL_SUBMIT_DRIVER_COMMAND);
    assert_eq!(
        extended(&command),
        [
            COMMAND.raw(),
            u64::from(COMMAND_RESULT.code),
            COMMAND_RESULT.value,
            0,
            0,
            0,
            0,
        ]
    );
}

fn context() -> AgentCallContext {
    AgentCallContext::new_driver(AGENT, INVOCATION, IMAGE, CAPABILITY).unwrap()
}

fn driver_frame(operation: u64, extended: [u64; 7]) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: extended[5],
        r14: extended[4],
        r13: extended[3],
        r12: extended[2],
        r11: extended[1],
        r10: extended[0],
        r9: NONCE,
        r8: IMAGE.raw(),
        rbp: extended[6],
        rdi: INVOCATION.raw(),
        rsi: AGENT.raw(),
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
    assert_eq!(
        [frame.rsi, frame.rdi, frame.r8, frame.r9],
        [AGENT.raw(), INVOCATION.raw(), IMAGE.raw(), NONCE]
    );
}

fn extended(frame: &PrivilegeInterruptStackFrame) -> [u64; 7] {
    [
        frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,
    ]
}
