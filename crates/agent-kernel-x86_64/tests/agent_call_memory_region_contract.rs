use agent_kernel_core::{AgentId, AgentImageId, CapabilityId, MemoryCellId, ResourceId, TaskId};
use agent_kernel_x86_64::{
    agent_call::{
        AgentCallContext, AgentCallDecodeError, AgentCallOperation, AgentCallRequest,
        AGENT_CALL_ABI_MAGIC, AGENT_CALL_ABI_VERSION, AGENT_CALL_ALLOCATE_MEMORY_REGION,
        AGENT_CALL_INSPECT_MEMORY_REGION, AGENT_CALL_MEMORY_REGION_MAX_PAGES,
        AGENT_CALL_MEMORY_REGION_PAGE_BYTES, AGENT_CALL_RELEASE_MEMORY_REGION,
        AGENT_CALL_STATUS_OK,
    },
    context::PrivilegeInterruptStackFrame,
};

const NONCE: u64 = 0xf66c_e006;
const CAPABILITY: CapabilityId = CapabilityId::new(15);
const RESOURCE: ResourceId = ResourceId::new(4);
const CELL: MemoryCellId = MemoryCellId::new(2);
const VIRTUAL_BASE: u64 = 0x0000_4000_0000_c000;
const FIRST_PROOF: u64 = 0x5245_4749_4f4e_3031;
const LAST_PROOF: u64 = 0x5245_4749_4f4e_3033;

#[test]
fn memory_region_requests_decode_and_authenticate() {
    assert_eq!(
        [
            AGENT_CALL_ALLOCATE_MEMORY_REGION,
            AGENT_CALL_INSPECT_MEMORY_REGION,
            AGENT_CALL_RELEASE_MEMORY_REGION,
        ],
        [24, 25, 26]
    );
    assert_eq!(AGENT_CALL_MEMORY_REGION_PAGE_BYTES, 4096);
    assert_eq!(AGENT_CALL_MEMORY_REGION_MAX_PAGES, 4);

    let allocate = AgentCallRequest::decode(&request_frame(
        AGENT_CALL_ALLOCATE_MEMORY_REGION,
        [CAPABILITY.raw(), RESOURCE.raw(), 3, 0, 0, 0, 0],
    ))
    .unwrap();
    assert_eq!(
        allocate,
        AgentCallRequest::AllocateMemoryRegion {
            agent: AgentId::new(8),
            task: TaskId::new(6),
            image: AgentImageId::new(8),
            nonce: NONCE,
            capability: CAPABILITY,
            resource: RESOURCE,
            page_count: 3,
        }
    );
    assert_eq!(
        allocate.operation(),
        AgentCallOperation::AllocateMemoryRegion
    );
    assert!(context().authenticates(allocate, NONCE));

    for (operation, expected) in [
        (
            AGENT_CALL_INSPECT_MEMORY_REGION,
            AgentCallOperation::InspectMemoryRegion,
        ),
        (
            AGENT_CALL_RELEASE_MEMORY_REGION,
            AgentCallOperation::ReleaseMemoryRegion,
        ),
    ] {
        let request = AgentCallRequest::decode(&request_frame(
            operation,
            [CAPABILITY.raw(), CELL.raw(), 0, 0, 0, 0, 0],
        ))
        .unwrap();
        assert_eq!(request.operation(), expected);
        assert!(context().authenticates(request, NONCE));
        assert!(!context().authenticates(request, NONCE + 1));
    }
}

#[test]
fn memory_region_requests_reject_invalid_counts_and_reserved_payloads() {
    for page_count in [0, AGENT_CALL_MEMORY_REGION_MAX_PAGES + 1] {
        assert_decode_error(
            request_frame(
                AGENT_CALL_ALLOCATE_MEMORY_REGION,
                [CAPABILITY.raw(), RESOURCE.raw(), page_count, 0, 0, 0, 0],
            ),
            AgentCallDecodeError::InvalidPayload,
        );
    }
    for operation in [
        AGENT_CALL_ALLOCATE_MEMORY_REGION,
        AGENT_CALL_INSPECT_MEMORY_REGION,
        AGENT_CALL_RELEASE_MEMORY_REGION,
    ] {
        let mut payload = [CAPABILITY.raw(), RESOURCE.raw(), 0, 0, 0, 0, 0];
        if operation == AGENT_CALL_ALLOCATE_MEMORY_REGION {
            payload[2] = 1;
        }
        payload[3] = 1;
        assert_decode_error(
            request_frame(operation, payload),
            AgentCallDecodeError::ReservedNotZero,
        );
    }
}

#[test]
fn memory_region_replies_are_canonical() {
    let mut allocated = request_frame(
        AGENT_CALL_ALLOCATE_MEMORY_REGION,
        [CAPABILITY.raw(), RESOURCE.raw(), 3, 0, 0, 0, 0],
    );
    context()
        .encode_memory_region_allocated_reply(&mut allocated, NONCE, CELL, VIRTUAL_BASE, 3, 1)
        .unwrap();
    assert_common_reply(&allocated, AGENT_CALL_ALLOCATE_MEMORY_REGION);
    assert_eq!(payload(&allocated), [2, VIRTUAL_BASE, 3 * 4096, 3, 1, 0, 0]);

    let mut inspected = request_frame(
        AGENT_CALL_INSPECT_MEMORY_REGION,
        [CAPABILITY.raw(), CELL.raw(), 0, 0, 0, 0, 0],
    );
    context()
        .encode_memory_region_inspected_reply(
            &mut inspected,
            NONCE,
            CELL,
            FIRST_PROOF,
            LAST_PROOF,
            3,
            1,
        )
        .unwrap();
    assert_common_reply(&inspected, AGENT_CALL_INSPECT_MEMORY_REGION);
    assert_eq!(
        payload(&inspected),
        [2, FIRST_PROOF, LAST_PROOF, 3, 1, 0, 0]
    );

    let mut released = request_frame(
        AGENT_CALL_RELEASE_MEMORY_REGION,
        [CAPABILITY.raw(), CELL.raw(), 0, 0, 0, 0, 0],
    );
    context()
        .encode_memory_region_released_reply(&mut released, NONCE, CELL, RESOURCE, 3, 1)
        .unwrap();
    assert_common_reply(&released, AGENT_CALL_RELEASE_MEMORY_REGION);
    assert_eq!(payload(&released), [2, 4, 3, 1, 0, 0, 0]);
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

fn assert_decode_error(frame: PrivilegeInterruptStackFrame, error: AgentCallDecodeError) {
    assert_eq!(AgentCallRequest::decode(&frame), Err(error));
}

fn payload(frame: &PrivilegeInterruptStackFrame) -> [u64; 7] {
    [
        frame.r10, frame.r11, frame.r12, frame.r13, frame.r14, frame.r15, frame.rbp,
    ]
}
