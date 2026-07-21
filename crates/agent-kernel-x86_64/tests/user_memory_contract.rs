use agent_kernel_x86_64::user_memory::{
    UserMemoryLayout, AGENT_CALL_RELEASE_OFFSET, AGENT_CODE_PAGE_CAPACITY,
    AGENT_RESTART_GENERATION_OFFSET, AGENT_RODATA_PAGE_CAPACITY, FIRST_AGENT_RESTART_GENERATION,
    LAZY_DATA_PROOF_VALUE, MAX_AGENT_RESTART_GENERATION, PAGE_BYTES,
    PHYSICAL_QUANTUM_GENERATION_OFFSET, SECOND_AGENT_RESTART_GENERATION, STACK_PAGE_COUNT,
    THIRD_AGENT_RESTART_GENERATION,
};

#[test]
fn user_region_has_separate_sixteen_page_code_and_rodata_windows() {
    let layout = UserMemoryLayout::fixed();
    assert_eq!(AGENT_CODE_PAGE_CAPACITY, 16);
    assert_eq!(AGENT_RODATA_PAGE_CAPACITY, 16);
    assert_eq!(layout.code_start(), 0x0000_4000_0000_0000);
    assert_eq!(
        layout.code_end(),
        layout.code_start() + PAGE_BYTES * AGENT_CODE_PAGE_CAPACITY as u64
    );
    for page in 0..AGENT_CODE_PAGE_CAPACITY {
        assert_eq!(
            layout.code_page_start(page),
            Some(layout.code_start() + PAGE_BYTES * page as u64)
        );
    }
    assert_eq!(layout.code_page_start(AGENT_CODE_PAGE_CAPACITY), None);
    assert_eq!(layout.rodata_start(), layout.code_end());
    assert_eq!(layout.rodata_start(), 0x0000_4000_0001_0000);
    assert_eq!(
        layout.rodata_end(),
        layout.rodata_start() + PAGE_BYTES * AGENT_RODATA_PAGE_CAPACITY as u64
    );
    for page in 0..AGENT_RODATA_PAGE_CAPACITY {
        assert_eq!(
            layout.rodata_page_start(page),
            Some(layout.rodata_start() + PAGE_BYTES * page as u64)
        );
    }
    assert_eq!(layout.rodata_page_start(AGENT_RODATA_PAGE_CAPACITY), None);
    assert_eq!(layout.signal_start(), layout.rodata_end());
    assert_eq!(layout.guard_start(), layout.signal_start() + PAGE_BYTES);
    assert_eq!(layout.stack_bottom(), layout.guard_start() + PAGE_BYTES);
    assert_eq!(
        layout.stack_top(),
        layout.stack_bottom() + PAGE_BYTES * STACK_PAGE_COUNT as u64
    );
    assert_eq!(layout.lazy_data_start(), layout.stack_top());
    assert_eq!(layout.signal_start(), 0x0000_4000_0002_0000);
    assert_eq!(layout.lazy_data_start(), 0x0000_4000_0002_6000);
    assert_eq!(LAZY_DATA_PROOF_VALUE, 0x5a);
    assert!(layout.contains_code(layout.code_start()));
    assert!(layout.contains_code(layout.code_end() - 1));
    assert!(!layout.contains_code(layout.code_end()));
    assert!(layout.contains_rodata(layout.rodata_start()));
    assert!(layout.contains_rodata(layout.rodata_end() - 1));
    assert!(!layout.contains_rodata(layout.rodata_end()));
    assert!(layout.contains_stack(layout.stack_top() - 8));
    assert!(!layout.contains_stack(layout.guard_start()));
    assert!(!layout.contains_stack(layout.lazy_data_start()));
    assert!(layout.contains_lazy_data(layout.lazy_data_start()));
    assert!(layout.contains_lazy_data(layout.lazy_data_start() + PAGE_BYTES - 1));
    assert!(!layout.contains_lazy_data(layout.lazy_data_start() - 1));
}

#[test]
fn call_data_page_follows_the_shifted_reserved_runtime_region() {
    let layout = UserMemoryLayout::fixed();

    assert_eq!(layout.call_data_start(), 0x0000_4000_0003_0000);
    assert_eq!(layout.call_data_start(), layout.runtime_region_end());
    assert_eq!(
        layout.call_data_end(),
        layout.call_data_start() + PAGE_BYTES
    );
    assert!(layout.contains_call_data(layout.call_data_start()));
    assert!(layout.contains_call_data(layout.call_data_end() - 1));
    assert!(!layout.contains_call_data(layout.call_data_start() - 1));
    assert!(!layout.contains_call_data(layout.call_data_end()));
    assert_eq!(layout.last_mapped_p4_index(), layout.p4_index());
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
