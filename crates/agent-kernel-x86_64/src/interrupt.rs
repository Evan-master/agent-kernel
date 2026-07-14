//! Fixed-width x86_64 interrupt descriptor and legacy PIC contracts.
//!
//! This architecture-library module owns byte-level IDT encoding and pure IRQ
//! vector/mask derivation. It performs no privileged instructions or I/O, so
//! host tests can validate the exact structures consumed by bare-metal code.

use core::mem::size_of;

pub const IDT_INTERRUPT_GATE_OPTIONS: u16 = 0x8e00;
pub const IDT_TRAP_GATE_OPTIONS: u16 = 0x8f00;
pub const PIC_MASTER_OFFSET: u8 = 0x20;
pub const PIC_SLAVE_OFFSET: u8 = 0x28;
pub const PIC_CASCADE_IRQ: u8 = 2;
pub const UART_IRQ_LINE: u8 = 4;
pub const UART_IRQ_VECTOR: u8 = PIC_MASTER_OFFSET + UART_IRQ_LINE;

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            options: 0,
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub const fn interrupt_gate(handler: u64, selector: u16) -> Self {
        Self::gate(handler, selector, IDT_INTERRUPT_GATE_OPTIONS)
    }

    pub const fn trap_gate(handler: u64, selector: u16) -> Self {
        Self::gate(handler, selector, IDT_TRAP_GATE_OPTIONS)
    }

    const fn gate(handler: u64, selector: u16, options: u16) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            options,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    pub const fn handler_address(self) -> u64 {
        self.offset_low as u64
            | ((self.offset_middle as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    pub const fn selector(self) -> u16 {
        self.selector
    }

    pub const fn options(self) -> u16 {
        self.options
    }

    pub const fn ist(self) -> u8 {
        (self.options & 0x7) as u8
    }

    pub const fn reserved(self) -> u32 {
        self.reserved
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

impl IdtPointer {
    pub const fn for_table(base: u64, entry_count: usize) -> Option<Self> {
        if entry_count == 0 {
            return None;
        }
        let Some(byte_len) = entry_count.checked_mul(size_of::<IdtEntry>()) else {
            return None;
        };
        let Some(limit) = byte_len.checked_sub(1) else {
            return None;
        };
        if limit > u16::MAX as usize {
            return None;
        }

        Some(Self {
            limit: limit as u16,
            base,
        })
    }

    pub const fn limit(self) -> u16 {
        self.limit
    }

    pub const fn base(self) -> u64 {
        self.base
    }
}

pub const fn legacy_irq_vector(irq: u8) -> Option<u8> {
    if irq < 8 {
        Some(PIC_MASTER_OFFSET + irq)
    } else if irq < 16 {
        Some(PIC_SLAVE_OFFSET + (irq - 8))
    } else {
        None
    }
}

pub const fn pic_masks_for_irq(irq: u8) -> Option<(u8, u8)> {
    if irq < 8 {
        Some((!(1u8 << irq), u8::MAX))
    } else if irq < 16 {
        Some((!(1u8 << PIC_CASCADE_IRQ), !(1u8 << (irq - 8))))
    } else {
        None
    }
}
