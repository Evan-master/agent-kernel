//! Native x86_64 byte port instructions.
//!
//! This architecture-layer adapter is compiled only for x86_64. Constructing
//! it is unsafe because possession grants direct port I/O authority; request
//! validation remains in `PortIoBackend`.

use core::arch::asm;

use crate::port::PortIo;

pub struct NativePortIo {
    _private: (),
}

impl NativePortIo {
    /// Creates direct x86 port I/O authority.
    ///
    /// # Safety
    ///
    /// The caller must run at a privilege level that permits port I/O and must
    /// bind this value only to a trusted, validated kernel endpoint.
    pub const unsafe fn new() -> Self {
        Self { _private: () }
    }
}

impl PortIo for NativePortIo {
    fn read_u8(&mut self, port: u16) -> u8 {
        let value: u8;
        unsafe {
            asm!(
                "in al, dx",
                in("dx") port,
                out("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    fn write_u8(&mut self, port: u16, value: u8) {
        unsafe {
            asm!(
                "out dx, al",
                in("dx") port,
                in("al") value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
