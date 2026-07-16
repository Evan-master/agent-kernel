use agent_kernel_x86_64::{
    address_space::{AgentMemoryIdentity, AGENT_CONTENT_FRAME_COUNT},
    context::{PrivilegeInterruptStackFrame, SavedAgentFrame, SAVED_AGENT_FRAME_BYTES},
};

#[test]
fn agent_memory_identities_reject_aliases_and_prove_disjoint_frames() {
    let first = AgentMemoryIdentity::new(
        0x1000,
        [0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000],
    )
    .unwrap();
    let second = AgentMemoryIdentity::new(
        0x9000,
        [0xa000, 0xb000, 0xc000, 0xd000, 0xe000, 0xf000, 0x10_000],
    )
    .unwrap();
    let overlapping = AgentMemoryIdentity::new(
        0x11_000,
        [
            0x2000, 0x12_000, 0x13_000, 0x14_000, 0x15_000, 0x16_000, 0x17_000,
        ],
    )
    .unwrap();
    let root_overlapping = AgentMemoryIdentity::new(
        0x2000,
        [
            0x11_000, 0x12_000, 0x13_000, 0x14_000, 0x15_000, 0x16_000, 0x17_000,
        ],
    )
    .unwrap();

    assert_eq!(AGENT_CONTENT_FRAME_COUNT, 7);
    assert!(first.is_disjoint_from(second));
    assert!(!first.is_disjoint_from(overlapping));
    assert!(!first.is_disjoint_from(root_overlapping));
    assert_eq!(first.root(), 0x1000);
    assert_eq!(first.content_frames()[0], 0x2000);
    assert!(AgentMemoryIdentity::new(
        0x1000,
        [0x2000, 0x2000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000],
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        0x1000,
        [0x1000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000],
    )
    .is_none());
    assert!(AgentMemoryIdentity::new(
        0x1001,
        [0x2000, 0x3000, 0x4000, 0x5000, 0x6000, 0x7000, 0x8000],
    )
    .is_none());
}

#[test]
fn saved_agent_frame_owns_a_complete_privilege_frame_by_value() {
    let mut hardware: PrivilegeInterruptStackFrame = unsafe { core::mem::zeroed() };
    hardware.rip = 0x4000_0000_0042;
    hardware.user_rsp = 0x4000_0000_7000;
    let saved = SavedAgentFrame::new(hardware);
    hardware.rip = 0;

    assert_eq!(hardware.rip, 0);
    assert_eq!(SAVED_AGENT_FRAME_BYTES, 160);
    assert_eq!(saved.frame().rip, 0x4000_0000_0042);
    assert_eq!(saved.frame().user_rsp, 0x4000_0000_7000);
}
