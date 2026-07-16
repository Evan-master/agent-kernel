use agent_kernel_x86_64::user_memory::{
    UserMemoryLayout, AGENT_CALL_RELEASE_OFFSET, AGENT_RESTART_GENERATION_OFFSET,
    FIRST_AGENT_RESTART_GENERATION, LAZY_DATA_PROOF_VALUE, MAX_AGENT_RESTART_GENERATION,
    PAGE_BYTES, PHYSICAL_QUANTUM_GENERATION_OFFSET, SECOND_AGENT_RESTART_GENERATION,
    STACK_PAGE_COUNT, THIRD_AGENT_RESTART_GENERATION,
};

#[test]
fn user_region_has_code_signal_guard_four_stack_pages_and_lazy_data() {
    let layout = UserMemoryLayout::fixed();
    assert_eq!(layout.code_start(), 0x0000_4000_0000_0000);
    assert_eq!(layout.signal_start(), layout.code_start() + PAGE_BYTES);
    assert_eq!(layout.guard_start(), layout.signal_start() + PAGE_BYTES);
    assert_eq!(layout.stack_bottom(), layout.guard_start() + PAGE_BYTES);
    assert_eq!(
        layout.stack_top(),
        layout.stack_bottom() + PAGE_BYTES * STACK_PAGE_COUNT as u64
    );
    assert_eq!(layout.lazy_data_start(), layout.stack_top());
    assert_eq!(layout.lazy_data_start(), 0x0000_4000_0000_7000);
    assert_eq!(LAZY_DATA_PROOF_VALUE, 0x5a);
    assert!(layout.contains_code(layout.code_start()));
    assert!(layout.contains_stack(layout.stack_top() - 8));
    assert!(!layout.contains_stack(layout.guard_start()));
    assert!(!layout.contains_stack(layout.lazy_data_start()));
    assert!(layout.contains_lazy_data(layout.lazy_data_start()));
    assert!(layout.contains_lazy_data(layout.lazy_data_start() + PAGE_BYTES - 1));
    assert!(!layout.contains_lazy_data(layout.lazy_data_start() - 1));
}

#[test]
fn signal_page_separates_call_release_from_physical_quantum_generation() {
    assert_eq!(AGENT_CALL_RELEASE_OFFSET, 0);
    assert_eq!(PHYSICAL_QUANTUM_GENERATION_OFFSET, 1);
    assert_ne!(
        AGENT_CALL_RELEASE_OFFSET,
        PHYSICAL_QUANTUM_GENERATION_OFFSET
    );
    assert!(PHYSICAL_QUANTUM_GENERATION_OFFSET < PAGE_BYTES as usize);
}

#[test]
fn signal_page_reserves_an_independent_restart_generation() {
    assert_eq!(AGENT_RESTART_GENERATION_OFFSET, 2);
    assert_eq!(FIRST_AGENT_RESTART_GENERATION, 1);
    assert_eq!(SECOND_AGENT_RESTART_GENERATION, 2);
    assert_eq!(THIRD_AGENT_RESTART_GENERATION, 3);
    assert_eq!(MAX_AGENT_RESTART_GENERATION, THIRD_AGENT_RESTART_GENERATION);
    assert_ne!(AGENT_RESTART_GENERATION_OFFSET, AGENT_CALL_RELEASE_OFFSET);
    assert_ne!(
        AGENT_RESTART_GENERATION_OFFSET,
        PHYSICAL_QUANTUM_GENERATION_OFFSET
    );
    assert!(AGENT_RESTART_GENERATION_OFFSET < PAGE_BYTES as usize);
}
