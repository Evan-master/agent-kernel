#![no_std]
#![no_main]
#![cfg_attr(
    any(
        feature = "qemu-dma-iommu-proof",
        feature = "qemu-msi-msix-proof",
        feature = "qemu-native-net-proof",
        feature = "qemu-native-udp-driver-proof"
    ),
    allow(dead_code)
)]

//! x86_64 bootloader entry for Agent Kernel.
//!
//! This crate owns the architecture-specific QEMU boot entry. It proves
//! persistent exceptions, CPL3 Agent preemption, and the bounded COM1
//! interrupt-to-command flow before publishing the deterministic handoff.

use core::{arch::asm, panic::PanicInfo};

use agent_kernel_boot::BootedKernel;
use bootloader_api::{entry_point, BootInfo};

mod admission_supervisor_flow;
mod agent_boot_flow;
mod agent_cpu;
mod agent_memory;
mod boot_agent_images;
mod boot_agent_trust;
mod boot_config;
#[cfg(any(
    all(feature = "qemu-dma-iommu-proof", feature = "qemu-msi-msix-proof"),
    all(feature = "qemu-dma-iommu-proof", feature = "qemu-native-net-proof"),
    all(feature = "qemu-msi-msix-proof", feature = "qemu-native-net-proof"),
    all(
        feature = "qemu-dma-iommu-proof",
        feature = "qemu-native-udp-driver-proof"
    ),
    all(
        feature = "qemu-msi-msix-proof",
        feature = "qemu-native-udp-driver-proof"
    ),
    all(
        feature = "qemu-native-net-proof",
        feature = "qemu-native-udp-driver-proof"
    )
))]
compile_error!("QEMU hardware proof profiles are mutually exclusive");
#[cfg(feature = "qemu-dma-iommu-proof")]
mod dma_iommu_boot;
mod event_trace;
mod exception_runtime;
mod fault_handler_flow;
mod fault_task_flow;
#[cfg(feature = "qemu-msi-msix-proof")]
mod msi_msix_boot;
mod native_address_space_service;
mod native_agent_executor;
mod native_agent_runtime;
mod native_driver_executor;
#[cfg(any(
    feature = "qemu-native-net-proof",
    feature = "qemu-native-udp-driver-proof"
))]
mod native_net_boot;
mod native_runtime_admission_broker;
mod pci_serial_driver_flow;
mod pci_serial_interrupt;
mod pci_serial_profile;
mod pic;
mod port_driver_flow;
mod privilege_runtime;
mod resource_manager_flow;
mod reuse_worker_flow;
mod smp_boot;
mod timer_task_flow;
mod uart_interrupt;
mod verifier_task_flow;

use boot_config::BOOTLOADER_CONFIG;
#[cfg(not(any(
    feature = "qemu-dma-iommu-proof",
    feature = "qemu-msi-msix-proof",
    feature = "qemu-native-net-proof",
    feature = "qemu-native-udp-driver-proof"
)))]
use boot_config::{
    durable_proof_role, durable_storage_profile, state_signer_profile, tpm_signer_profile,
};
use privilege_runtime::PrivilegeBoundary;
use smp_boot::SmpBootstrap;

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

pub(crate) const X86_TASK_CAPACITY: usize = 12;
pub(crate) const X86_INTENT_CAPACITY: usize = 12;
pub(crate) const X86_AGENT_RESOURCE_CAPACITY: usize = 10;
pub(crate) const X86_DURABLE_RESOURCE_RESERVE: usize = 1;
pub(crate) const X86_PCI_RESOURCE_RESERVE: usize = 7;
pub(crate) const X86_RESOURCE_CAPACITY: usize =
    X86_AGENT_RESOURCE_CAPACITY + X86_DURABLE_RESOURCE_RESERVE + X86_PCI_RESOURCE_RESERVE;
pub(crate) const X86_AGENT_CAPABILITY_CAPACITY: usize = 30;
pub(crate) const X86_DURABLE_CAPABILITY_RESERVE: usize = 4;
pub(crate) const X86_DURABLE_BIND_EVENT_RESERVE: usize = 1;
pub(crate) const X86_PCI_CAPABILITY_RESERVE: usize = 7;
pub(crate) const X86_CAPABILITY_CAPACITY: usize =
    X86_AGENT_CAPABILITY_CAPACITY + X86_DURABLE_CAPABILITY_RESERVE + X86_PCI_CAPABILITY_RESERVE;
pub(crate) const X86_RUNTIME_ADMISSION_CAPACITY: usize = 16;
pub(crate) const X86_DEVICE_EVENT_CAPACITY: usize = 3;
pub(crate) const X86_DRIVER_COMMAND_CAPACITY: usize = 3;
pub(crate) const X86_DRIVER_INVOCATION_CAPACITY: usize = 3;
pub(crate) const X86_WAITER_CAPACITY: usize = 3;
pub(crate) const X86_FAULT_CAPACITY: usize = 4;
pub(crate) const X86_MEMORY_CELL_CAPACITY: usize = 5;
pub(crate) const X86_NAMESPACE_ENTRY_CAPACITY: usize = 4;
pub(crate) const X86_EVENT_ARCHIVE_WATERMARK: usize = 378;
pub(crate) const X86_TERMINAL_EVENT_SEQUENCE: usize = 412;
pub(crate) const X86_PCI_CLAIM_EVENT_RESERVE: usize =
    2 + agent_kernel_core::DRIVER_RESOURCE_REGION_CAPACITY * 3;
pub(crate) const X86_PCI_DRIVER_EVENT_RESERVE: usize = 34;
pub(crate) const X86_PCI_EVENT_RESERVE: usize =
    X86_PCI_CLAIM_EVENT_RESERVE + X86_PCI_DRIVER_EVENT_RESERVE;
pub(crate) const X86_HANDOFF_EVENT_RESERVE: usize = 64;
pub(crate) const X86_EVENT_CAPACITY: usize =
    X86_TERMINAL_EVENT_SEQUENCE + X86_PCI_EVENT_RESERVE + X86_HANDOFF_EVENT_RESERVE;
pub(crate) type NativeDurableSession<'a> = agent_kernel_x86_64::ata::NativeAtaDurableSession<
    'a,
    agent_kernel_x86_64::ata::AtaPioDevice<agent_kernel_x86_64::NativePortIo>,
>;
pub(crate) type X86BootedKernel = BootedKernel<
    14,
    X86_RESOURCE_CAPACITY,
    X86_CAPABILITY_CAPACITY,
    X86_EVENT_CAPACITY,
    1,
    1,
    0,
    X86_INTENT_CAPACITY,
    X86_TASK_CAPACITY,
    2,
    2,
    X86_DEVICE_EVENT_CAPACITY,
    X86_DRIVER_COMMAND_CAPACITY,
    X86_DRIVER_INVOCATION_CAPACITY,
    4,
    X86_WAITER_CAPACITY,
    X86_FAULT_CAPACITY,
    1,
    1,
    X86_MEMORY_CELL_CAPACITY,
    X86_RUNTIME_ADMISSION_CAPACITY,
    X86_NAMESPACE_ENTRY_CAPACITY,
>;

const KERNEL_STATE_STACK_HEADROOM: usize = 4;
const _: () = assert!(
    core::mem::size_of::<X86BootedKernel>() * KERNEL_STATE_STACK_HEADROOM
        <= boot_config::KERNEL_STACK_SIZE as usize
);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    serial_init();
    serial_write_line("AGENT_KERNEL_QEMU_BOOT_OK");
    if privilege_runtime::prepare_guard_pages(boot_info).is_err() {
        fatal_boot("AGENT_KERNEL_PER_CPU_GUARD_PAGES_ERROR");
    }
    serial_write_line("AGENT_KERNEL_PER_CPU_GUARD_PAGES_OK");
    let Some(privilege_boundary) =
        PrivilegeBoundary::install(agent_kernel_x86_64::cpu::CpuIndex::BSP)
    else {
        fatal_boot("AGENT_KERNEL_GDT_TSS_ERROR");
    };
    serial_write_line("AGENT_KERNEL_GDT_TSS_OK");
    if exception_runtime::install_and_probe().is_none() {
        fatal_boot("AGENT_KERNEL_EXCEPTION_BASELINE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_EXCEPTION_BASELINE_OK");
    let Ok(smp_bootstrap) = SmpBootstrap::discover(boot_info) else {
        fatal_boot("AGENT_KERNEL_ACPI_TOPOLOGY_ERROR");
    };
    serial_write_line("AGENT_KERNEL_ACPI_TOPOLOGY_OK");
    #[cfg(feature = "qemu-dma-iommu-proof")]
    {
        dma_iommu_boot::run(boot_info, privilege_boundary, smp_bootstrap)
    }
    #[cfg(feature = "qemu-msi-msix-proof")]
    {
        msi_msix_boot::run(boot_info, privilege_boundary, smp_bootstrap)
    }
    #[cfg(feature = "qemu-native-net-proof")]
    {
        native_net_boot::run(boot_info, privilege_boundary, smp_bootstrap)
    }
    #[cfg(feature = "qemu-native-udp-driver-proof")]
    {
        native_net_boot::run(boot_info, privilege_boundary, smp_bootstrap)
    }
    #[cfg(not(any(
        feature = "qemu-dma-iommu-proof",
        feature = "qemu-msi-msix-proof",
        feature = "qemu-native-net-proof",
        feature = "qemu-native-udp-driver-proof"
    )))]
    {
        let Some(durable_role) = durable_proof_role() else {
            fatal_boot("AGENT_KERNEL_QEMU_DURABLE_PROFILE_ERROR");
        };
        let Some(durable_profile) = durable_storage_profile(durable_role) else {
            fatal_boot("AGENT_KERNEL_QEMU_DURABLE_PROFILE_ERROR");
        };
        let Some(tpm_profile) = tpm_signer_profile(durable_role) else {
            fatal_boot("AGENT_KERNEL_QEMU_DURABLE_PROFILE_ERROR");
        };
        agent_boot_flow::run(
            boot_info,
            privilege_boundary,
            smp_bootstrap,
            durable_profile,
            tpm_profile,
            durable_role,
            state_signer_profile(durable_role),
        )
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_init();
    fatal_boot("AGENT_KERNEL_PANIC")
}

const COM1: u16 = 0x3f8;
const QEMU_EXIT_PORT: u16 = 0xf4;

fn serial_init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xc7);
        outb(COM1 + 4, 0x0b);
    }
}

fn serial_write_line(text: &str) {
    serial_write_str(text);
    serial_write_str("\n");
}

fn serial_write_str(text: &str) {
    for byte in text.bytes() {
        serial_write_byte(byte);
    }
}

fn serial_write_u64(mut value: u64) {
    if value == 0 {
        serial_write_byte(b'0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut len = 0;
    while value > 0 {
        digits[len] = b'0' + (value % 10) as u8;
        value /= 10;
        len += 1;
    }
    while len > 0 {
        len -= 1;
        serial_write_byte(digits[len]);
    }
}

fn serial_write_byte(byte: u8) {
    while !serial_transmit_empty() {}
    unsafe {
        outb(COM1, byte);
    }
}

fn serial_transmit_empty() -> bool {
    unsafe { inb(COM1 + 5) & 0x20 != 0 }
}

fn fatal_boot(message: &str) -> ! {
    serial_write_line(message);
    exit_qemu(0x11);
    halt_forever()
}

fn exit_qemu(code: u8) {
    unsafe {
        outb(QEMU_EXIT_PORT, code);
    }
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack, preserves_flags));
    }
    value
}
