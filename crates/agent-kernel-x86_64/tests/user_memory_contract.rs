use agent_kernel_x86_64::user_memory::{
    agent_proof_program, UserMemoryLayout, AGENT_CALL_RETURN_OFFSET, AGENT_CODE_BYTES, PAGE_BYTES,
    STACK_PAGE_COUNT,
};

#[test]
fn user_region_has_code_signal_guard_and_four_stack_pages() {
    let layout = UserMemoryLayout::fixed();
    assert_eq!(layout.code_start(), 0x0000_4000_0000_0000);
    assert_eq!(layout.signal_start(), layout.code_start() + PAGE_BYTES);
    assert_eq!(layout.guard_start(), layout.signal_start() + PAGE_BYTES);
    assert_eq!(layout.stack_bottom(), layout.guard_start() + PAGE_BYTES);
    assert_eq!(
        layout.stack_top(),
        layout.stack_bottom() + PAGE_BYTES * STACK_PAGE_COUNT as u64
    );
    assert!(layout.contains_code(layout.code_start()));
    assert!(layout.contains_stack(layout.stack_top() - 8));
    assert!(!layout.contains_stack(layout.guard_start()));
}

#[test]
fn proof_program_polls_signal_then_invokes_agent_call_gate() {
    let layout = UserMemoryLayout::fixed();
    let program = agent_proof_program();
    assert_eq!(program.len(), AGENT_CODE_BYTES);
    assert_eq!(&program[0..2], &[0x53, 0x5b]);
    assert_eq!(&program[2..4], &[0x48, 0xb8]);
    assert_eq!(
        u64::from_le_bytes(program[4..12].try_into().unwrap()),
        layout.signal_start()
    );
    assert_eq!(&program[12..17], &[0x80, 0x38, 0x00, 0x74, 0xfb]);
    assert_eq!(&program[17..19], &[0xcd, 0x90]);
    assert_eq!(&program[19..21], &[0xeb, 0xfe]);
    assert_eq!(AGENT_CALL_RETURN_OFFSET, 19);
}
