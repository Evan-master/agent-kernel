use agent_kernel_x86_64::{
    address_space::{AddressSpaceKind, AddressSpaceRoots, AGENT_P4_INDEX},
    user_memory::UserMemoryLayout,
};

#[test]
fn agent_region_owns_one_dedicated_p4_slot() {
    let layout = UserMemoryLayout::fixed();
    assert_eq!(AGENT_P4_INDEX, 128);
    assert_eq!(layout.p4_index(), AGENT_P4_INDEX);
    assert_eq!(layout.last_mapped_p4_index(), AGENT_P4_INDEX);
}

#[test]
fn address_space_roots_are_aligned_distinct_and_preserve_cr3_control() {
    let roots = AddressSpaceRoots::new(0x2000, 0x9000, 0x18).unwrap();
    assert_eq!(roots.kernel_cr3(), 0x2018);
    assert_eq!(roots.agent_cr3(), 0x9018);
    assert_eq!(roots.classify(0x2018), Some(AddressSpaceKind::Kernel));
    assert_eq!(roots.classify(0x9018), Some(AddressSpaceKind::Agent));
    assert_eq!(roots.classify(0xa018), None);

    assert_eq!(
        AddressSpaceRoots::new(0, 0x9000, 0).unwrap().kernel_cr3(),
        0
    );
    assert_eq!(AddressSpaceRoots::new(0x2000, 0, 0).unwrap().agent_cr3(), 0);
    assert!(AddressSpaceRoots::new(0x2001, 0x9000, 0).is_none());
    assert!(AddressSpaceRoots::new(0x2000, 0x9001, 0).is_none());
    assert!(AddressSpaceRoots::new(0x2000, 0x2000, 0).is_none());
    assert!(AddressSpaceRoots::new(1 << 52, 0x9000, 0).is_none());
    assert!(AddressSpaceRoots::new(0x2000, 0x9000, 0x1).is_none());
    assert!(AddressSpaceRoots::new(0x2000, 0x9000, 0x1000).is_none());
}
