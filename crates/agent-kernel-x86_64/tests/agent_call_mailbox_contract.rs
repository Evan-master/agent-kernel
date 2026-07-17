use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, FaultId, IntentId, MessageId, MessageKind, MessagePayload,
    MessageRecord, MessageStatus, ResourceId, TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_ACKNOWLEDGE_MESSAGE,
        AGENT_CALL_MESSAGE_FAULT, AGENT_CALL_MESSAGE_NOTIFY, AGENT_CALL_MESSAGE_REQUEST,
        AGENT_CALL_MESSAGE_RESPONSE, AGENT_CALL_RECEIVE_MESSAGE, AGENT_CALL_SEND_MESSAGE,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa11c_e001;
const MESSAGE: MessageId = MessageId::new(1);
const RECIPIENT: AgentId = AgentId::new(4);
const PAYLOAD_TASK: TaskId = TaskId::new(1);

#[test]
fn mailbox_requests_decode_and_match_trusted_context() {
    assert_eq!(AGENT_CALL_SEND_MESSAGE, 7);
    assert_eq!(AGENT_CALL_RECEIVE_MESSAGE, 8);
    assert_eq!(AGENT_CALL_ACKNOWLEDGE_MESSAGE, 9);
    assert_eq!(AGENT_CALL_MESSAGE_NOTIFY, 1);

    let send = AgentCallRequest::decode(&send_frame()).unwrap();
    assert_eq!(
        send,
        AgentCallRequest::SendMessage {
            agent: AgentId::new(3),
            task: TaskId::new(1),
            image: AgentImageId::new(3),
            nonce: NONCE,
            recipient: RECIPIENT,
            kind: MessageKind::Notify,
            payload: MessagePayload {
                task: Some(PAYLOAD_TASK),
                ..MessagePayload::empty()
            },
        }
    );
    assert_eq!(send.operation(), AgentCallOperation::SendMessage);
    assert_eq!(
        context().match_message_send(send, NONCE, RECIPIENT),
        Some((
            MessageKind::Notify,
            MessagePayload {
                task: Some(PAYLOAD_TASK),
                ..MessagePayload::empty()
            }
        ))
    );
    assert_eq!(
        context().match_message_send(send, NONCE, AgentId::new(5)),
        None
    );

    let receive = AgentCallRequest::decode(&receive_frame()).unwrap();
    assert_eq!(receive.operation(), AgentCallOperation::ReceiveMessage);
    assert!(receiver_context().matches_message_receive(receive, NONCE));
    assert!(!receiver_context().matches_message_receive(receive, NONCE + 1));

    let acknowledge = AgentCallRequest::decode(&acknowledge_frame()).unwrap();
    assert_eq!(
        acknowledge.operation(),
        AgentCallOperation::AcknowledgeMessage
    );
    assert!(receiver_context().matches_message_acknowledgement(acknowledge, NONCE, MESSAGE));
    assert!(!receiver_context().matches_message_acknowledgement(
        acknowledge,
        NONCE,
        MessageId::new(2)
    ));
}

#[test]
fn mailbox_requests_reject_noncanonical_payloads() {
    let mut zero_recipient = send_frame();
    zero_recipient.r10 = 0;
    assert_eq!(
        AgentCallRequest::decode(&zero_recipient),
        Err(AgentCallDecodeError::InvalidPayload)
    );

    for (code, expected) in [
        (AGENT_CALL_MESSAGE_NOTIFY, MessageKind::Notify),
        (AGENT_CALL_MESSAGE_REQUEST, MessageKind::Request),
        (AGENT_CALL_MESSAGE_RESPONSE, MessageKind::Response),
        (AGENT_CALL_MESSAGE_FAULT, MessageKind::Fault),
    ] {
        let mut frame = send_frame();
        frame.r11 = code;
        assert!(matches!(
            AgentCallRequest::decode(&frame),
            Ok(AgentCallRequest::SendMessage { kind, .. }) if kind == expected
        ));
    }
    for kind in [0, 5] {
        let mut frame = send_frame();
        frame.r11 = kind;
        assert_eq!(
            AgentCallRequest::decode(&frame),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }

    let mut send_reserved = send_frame();
    send_reserved.r13 = 1;
    assert_eq!(
        AgentCallRequest::decode(&send_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
    let mut receive_reserved = receive_frame();
    receive_reserved.r12 = 1;
    assert_eq!(
        AgentCallRequest::decode(&receive_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
    let mut zero_message = acknowledge_frame();
    zero_message.r10 = 0;
    assert_eq!(
        AgentCallRequest::decode(&zero_message),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    let mut acknowledge_reserved = acknowledge_frame();
    acknowledge_reserved.r11 = 1;
    assert_eq!(
        AgentCallRequest::decode(&acknowledge_reserved),
        Err(AgentCallDecodeError::ReservedNotZero)
    );
}

#[test]
fn mailbox_replies_preserve_context_and_return_bounded_records() {
    let mut send = send_frame();
    let send_control = control_words(&send);
    context()
        .encode_message_send_reply(&mut send, NONCE, MESSAGE)
        .unwrap();
    assert_common_reply(&send, AGENT_CALL_SEND_MESSAGE, [3, 1, 3, NONCE]);
    assert_eq!([send.r10, send.r11, send.r12, send.r13], [1, 0, 0, 0]);
    assert_eq!([send.r14, send.r15, send.rbp], [0, 0, 0]);
    assert_eq!(control_words(&send), send_control);

    let record = received_record(MessagePayload {
        task: Some(PAYLOAD_TASK),
        ..MessagePayload::empty()
    });
    let mut receive = receive_frame();
    let receive_control = control_words(&receive);
    receiver_context()
        .encode_message_receive_reply(&mut receive, NONCE, record)
        .unwrap();
    assert_common_reply(&receive, AGENT_CALL_RECEIVE_MESSAGE, [4, 2, 4, NONCE]);
    assert_eq!(
        [receive.r10, receive.r11, receive.r12, receive.r13],
        [1, 3, AGENT_CALL_MESSAGE_NOTIFY, 1]
    );
    assert_eq!([receive.r14, receive.r15, receive.rbp], [0, 0, 0]);
    assert_eq!(control_words(&receive), receive_control);

    let unsupported = received_record(MessagePayload {
        capability: Some(CapabilityId::new(9)),
        ..MessagePayload::empty()
    });
    assert_eq!(
        receiver_context().encode_message_receive_reply(&mut receive, NONCE, unsupported),
        Err(AgentCallDecodeError::InvalidPayload)
    );
    let ambiguous_zero_task = received_record(MessagePayload {
        task: Some(TaskId::new(0)),
        ..MessagePayload::empty()
    });
    assert_eq!(
        receiver_context().encode_message_receive_reply(&mut receive, NONCE, ambiguous_zero_task),
        Err(AgentCallDecodeError::InvalidPayload)
    );

    let mut acknowledge = acknowledge_frame();
    receiver_context()
        .encode_message_acknowledgement_reply(&mut acknowledge, NONCE)
        .unwrap();
    assert_common_reply(
        &acknowledge,
        AGENT_CALL_ACKNOWLEDGE_MESSAGE,
        [4, 2, 4, NONCE],
    );
    assert_eq!(
        [
            acknowledge.r10,
            acknowledge.r11,
            acknowledge.r12,
            acknowledge.r13,
            acknowledge.r14,
            acknowledge.r15,
            acknowledge.rbp,
        ],
        [0; 7]
    );
}

#[test]
fn mailbox_receive_reply_exposes_bounded_fault_route_payload() {
    let record = MessageRecord {
        id: MessageId::new(2),
        sender: AgentId::new(1),
        recipient: RECIPIENT,
        kind: MessageKind::Fault,
        payload: MessagePayload {
            resource: Some(ResourceId::new(1)),
            capability: None,
            intent: Some(IntentId::new(4)),
            task: Some(TaskId::new(4)),
            action: None,
            fault: Some(FaultId::new(4)),
        },
        status: MessageStatus::Received,
    };
    let mut receive = receive_frame();

    receiver_context()
        .encode_message_receive_reply(&mut receive, NONCE, record)
        .expect("fault route payload should fit the bounded reply");

    assert_common_reply(&receive, AGENT_CALL_RECEIVE_MESSAGE, [4, 2, 4, NONCE]);
    assert_eq!(
        [
            receive.r10,
            receive.r11,
            receive.r12,
            receive.r13,
            receive.r14,
            receive.r15,
            receive.rbp,
        ],
        [2, 1, AGENT_CALL_MESSAGE_FAULT, 4, 1, 4, 4]
    );

    for invalid in [
        MessagePayload {
            fault: Some(FaultId::new(0)),
            ..record.payload
        },
        MessagePayload {
            resource: Some(ResourceId::new(0)),
            ..record.payload
        },
        MessagePayload {
            intent: Some(IntentId::new(0)),
            ..record.payload
        },
    ] {
        assert_eq!(
            receiver_context().encode_message_receive_reply(
                &mut receive,
                NONCE,
                MessageRecord {
                    payload: invalid,
                    ..record
                }
            ),
            Err(AgentCallDecodeError::InvalidPayload)
        );
    }
}

fn context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(3),
        TaskId::new(1),
        AgentImageId::new(3),
        CapabilityId::new(3),
    )
    .unwrap()
}

fn receiver_context() -> AgentCallContext {
    AgentCallContext::new(
        AgentId::new(4),
        TaskId::new(2),
        AgentImageId::new(4),
        CapabilityId::new(4),
    )
    .unwrap()
}

fn send_frame() -> PrivilegeInterruptStackFrame {
    request_frame(3, 1, 3, AGENT_CALL_SEND_MESSAGE, [4, 1, 1, 0])
}

fn receive_frame() -> PrivilegeInterruptStackFrame {
    request_frame(4, 2, 4, AGENT_CALL_RECEIVE_MESSAGE, [0; 4])
}

fn acknowledge_frame() -> PrivilegeInterruptStackFrame {
    request_frame(4, 2, 4, AGENT_CALL_ACKNOWLEDGE_MESSAGE, [1, 0, 0, 0])
}

fn request_frame(
    agent: u64,
    task: u64,
    image: u64,
    operation: u64,
    operation_payload: [u64; 4],
) -> PrivilegeInterruptStackFrame {
    PrivilegeInterruptStackFrame {
        r15: 0,
        r14: 0,
        r13: operation_payload[3],
        r12: operation_payload[2],
        r11: operation_payload[1],
        r10: operation_payload[0],
        r9: NONCE,
        r8: image,
        rbp: 0,
        rdi: task,
        rsi: agent,
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

fn received_record(payload: MessagePayload) -> MessageRecord {
    MessageRecord {
        id: MESSAGE,
        sender: AgentId::new(3),
        recipient: RECIPIENT,
        kind: MessageKind::Notify,
        payload,
        status: MessageStatus::Received,
    }
}

fn assert_common_reply(frame: &PrivilegeInterruptStackFrame, operation: u64, identity: [u64; 4]) {
    assert_eq!(frame.rax, AGENT_CALL_ABI_MAGIC);
    assert_eq!(frame.rbx, AGENT_CALL_ABI_VERSION);
    assert_eq!(frame.rcx, AGENT_CALL_STATUS_OK);
    assert_eq!(frame.rdx, operation);
    assert_eq!([frame.rsi, frame.rdi, frame.r8, frame.r9], identity);
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
