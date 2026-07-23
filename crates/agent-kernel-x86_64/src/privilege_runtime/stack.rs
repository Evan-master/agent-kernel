//! Page-aligned storage for one guarded per-CPU privileged-entry stack.
//!
//! This architecture-binary child owns the static guard-plus-stack shape and
//! exports validated bounds to the TSS installer. Mapping removal remains in the
//! guard-page child, while descriptor installation remains in the parent module.

use core::cell::UnsafeCell;

use agent_kernel_x86_64::privilege::{
    PrivilegedStackLayout, PRIVILEGED_STACK_BYTES, PRIVILEGED_STACK_GUARD_BYTES,
};

#[repr(C, align(4096))]
pub(super) struct PrivilegedStack {
    guard: UnsafeCell<[u8; PRIVILEGED_STACK_GUARD_BYTES]>,
    bytes: UnsafeCell<[u8; PRIVILEGED_STACK_BYTES]>,
}

impl PrivilegedStack {
    pub(super) const fn new() -> Self {
        Self {
            guard: UnsafeCell::new([0; PRIVILEGED_STACK_GUARD_BYTES]),
            bytes: UnsafeCell::new([0; PRIVILEGED_STACK_BYTES]),
        }
    }

    pub(super) fn layout(&self) -> Option<PrivilegedStackLayout> {
        let guard_start = self.guard.get().cast::<u8>() as usize;
        let stack_start = self.bytes.get().cast::<u8>() as usize;
        let layout = PrivilegedStackLayout::new(guard_start)?;
        (layout.stack_start() == stack_start).then_some(layout)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct PrivilegedStackBounds {
    pub(super) guard_start: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

const _: () = assert!(core::mem::offset_of!(PrivilegedStack, guard) == 0);
const _: () =
    assert!(core::mem::offset_of!(PrivilegedStack, bytes) == PRIVILEGED_STACK_GUARD_BYTES);
const _: () = assert!(
    core::mem::size_of::<PrivilegedStack>()
        == PRIVILEGED_STACK_GUARD_BYTES + PRIVILEGED_STACK_BYTES
);
