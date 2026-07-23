//! Native x86_64 byte port instructions.
//!
//! This architecture-layer adapter is compiled only for x86_64. Constructing
//! it is unsafe because possession grants direct port I/O authority; request
//! validation remains in the bounded architecture backends.

use core::arch::asm;

use crate::{ata::AtaRegisterIo, port::PortIo};

pub struct NativePortIo {
    _private: (),
}

impl NativePortIo {
    /// Creates direct x86 port I/O authority.
    ///
    /// # Safety
    ///
    /// The caller must run at a privilege level that permits port I/O and must
    /// bind this value only to a trusted, validated architecture backend.
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

impl AtaRegisterIo for NativePortIo {
    fn read_u8(&mut self, port: u16) -> u8 {
        <Self as PortIo>::read_u8(self, port)
    }

    fn write_u8(&mut self, port: u16, value: u8) {
        <Self as PortIo>::write_u8(self, port, value);
    }

    fn read_u16(&mut self, port: u16) -> u16 {
        let value: u16;
        unsafe {
            asm!(
                "in ax, dx",
                in("dx") port,
                out("ax") value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    fn write_u16(&mut self, port: u16, value: u16) {
        unsafe {
            asm!(
                "out dx, ax",
                in("dx") port,
                in("ax") value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
