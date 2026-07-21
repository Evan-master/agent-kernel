use agent_kernel_core::{
    AgentId, AgentImageId, CapabilityId, MemoryCellId, MemoryCellRecord, MemoryValue, ResourceId,
    TaskId,
};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_RETIRE_MEMORY_CELL_RECORD,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xa66e_0007;
const AUTHORITY: CapabilityId = CapabilityId::new(23);
const TARGET: MemoryCellId = MemoryCellId::new(2);

#[test]
fn memory_cell_record_retirement_decodes_and_authenticates() {
    assert_eq!(AGENT_CALL_RETIRE_MEMORY_CELL_RECORD, 43);
    let request = AgentCallRequest::decode(&request_frame()).expect("request decodes");

    assert_eq!(
        request,
        AgentCallRequest::RetireMemoryCellRecord {
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
        AgentCallOperation::RetireMemoryCellRecord
    );
    assert!(context().authenticates(request, NONCE));
    assert!(!context().authenticates(request, NONCE + 1));
}

#[test]
fn memory_cell_record_retirement_rejects_zero_and_reserved_payloads() {
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
fn memory_cell_record_retirement_reply_is_canonical() {
    let record = MemoryCellRecord {
        id: TARGET,
        resource: ResourceId::new(4),
        creator: AgentId::new(8),
        last_writer: AgentId::new(8),
        value: MemoryValue::new([0x0000_4000_0001_7000, 4096, 3, 1]),
        revision: 2,
    };
    let mut frame = request_frame();
    let control = control_words(&frame);
    context()
        .encode_memory_cell_record_retirement_reply(&mut frame, NONCE, record)
        .expect("reply encodes");

    assert_common_reply(&frame);
    assert_eq!(
        payload(&frame),
        [2, 4, 2, 0x0000_4000_0001_7000, 4096, 3, 1,]
    );
    assert_eq!(control_words(&frame), control);

    for invalid in [
        MemoryCellRecord {
            id: MemoryCellId::new(0),
            ..record
        },
        MemoryCellRecord {
            resource: ResourceId::new(0),
            ..record
        },
        MemoryCellRecord {
            creator: AgentId::new(0),
            ..record
        },
        MemoryCellRecord {
            last_writer: AgentId::new(0),
            ..record
        },
        MemoryCellRecord {
            revision: 0,
            ..record
        },
    ] {
        assert_eq!(
            context().encode_memory_cell_record_retirement_reply(&mut frame, NONCE, invalid),
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
        rcx: AGENT_CALL_RETIRE_MEMORY_CELL_RECORD,
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
    assert_eq!(frame.rdx, AGENT_CALL_RETIRE_MEMORY_CELL_RECORD);
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
