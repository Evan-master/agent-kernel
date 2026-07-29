//! V30 Ring-3 Network Driver Agent and native IPv4/UDP proof.

mod admission;
mod backend;
mod platform;
mod session;

use agent_kernel_core::{AgentId, DriverEndpointDescriptor, NetworkMacAddress};
use agent_kernel_x86_64::agent_image::{AgentImageFormat, VerifiedAgentImage};
use bootloader_api::BootInfo;

use crate::{
    agent_cpu::AgentCpuRuntime, agent_memory::PreparedAgentMemory, boot_agent_images,
    exception_runtime, fatal_boot, privilege_runtime::PrivilegeBoundary, serial_write_line,
    smp_boot::SmpBootstrap,
};

use super::{authority, interrupts, pci};

pub(super) const DRIVER: AgentId = AgentId::new(2);
pub(super) const NETWORK_COMMAND_RESOLVE_NEIGHBOR: u16 = 0x3001;
pub(super) const NETWORK_COMMAND_EXCHANGE_UDP: u16 = 0x3002;
pub(super) const NETWORK_RESULT_OK: u16 = 0;
pub(super) const UDP_SOURCE_PORT: u16 = 40131;
pub(super) const UDP_DESTINATION_PORT: u16 = 40130;
pub(super) const UDP_PAYLOAD: &[u8; 13] = b"AGENT-V30-UDP";
pub(super) const GUEST_MAC_BYTES: [u8; 6] = [0x52, 0x54, 0, 0x12, 0x34, 0x56];

pub(super) fn run(
    boot_info: &'static mut BootInfo,
    privilege_boundary: PrivilegeBoundary,
    mut smp_bootstrap: SmpBootstrap,
) -> ! {
    smp_bootstrap
        .prepare_apic_mmio(boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    let hardware = pci::prepare(&mut smp_bootstrap, boot_info)
        .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_DMAR_DISCOVERY_OK");

    let guest_mac =
        NetworkMacAddress::new(GUEST_MAC_BYTES).expect("fixed unicast network identity");
    let (mut booted, authority) = authority::reserve_for_driver(
        smp_bootstrap.bsp_apic_id().get(),
        hardware.source_id(),
        guest_mac,
    )
    .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_AUTHORITY_ERROR"));
    let common_bar = hardware.regions().common().bar();
    let image_contract = boot_agent_images::network_driver();
    let driver = admission::prepare(
        &mut booted,
        authority.device(),
        DriverEndpointDescriptor::mmio(common_bar.base(), common_bar.size()),
        image_contract,
    )
    .unwrap_or_else(|error| fatal_boot(error.diagnostic_marker()));
    let image_record = booted
        .kernel()
        .agent_image(driver.image)
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_ERROR"));
    let verified_image = VerifiedAgentImage::verify(image_record, image_contract.bytes())
        .unwrap_or_else(|_| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_ERROR"));
    if verified_image.format() != AgentImageFormat::CapsuleV1 {
        fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_DRIVER_IMAGE_OK");

    let neighbor_memory = PreparedAgentMemory::prepare(boot_info, verified_image)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_MEMORY_ERROR"));
    let udp_memory = PreparedAgentMemory::prepare(boot_info, verified_image)
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_MEMORY_ERROR"));
    if !neighbor_memory.is_disjoint_from(&udp_memory) {
        fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_MEMORY_ERROR");
    }
    let (local_apic_base, physical_offset, initial_count) = smp_bootstrap
        .bsp_quantum_timer()
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_CPU_ERROR"));
    let cpu_runtime = AgentCpuRuntime::install(
        &privilege_boundary,
        neighbor_memory.roots(),
        smp_bootstrap.bsp_index(),
        local_apic_base,
        physical_offset,
        initial_count,
    )
    .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_DRIVER_CPU_ERROR"));
    interrupts::install_gates().unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IDT_ERROR"));
    exception_runtime::freeze_for_smp()
        .unwrap_or_else(|| fatal_boot("AGENT_KERNEL_NATIVE_UDP_IDT_FREEZE_ERROR"));
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_RING3_READY_OK");

    platform::run(
        boot_info,
        smp_bootstrap,
        hardware,
        booted,
        authority,
        driver,
        image_contract,
        cpu_runtime,
        neighbor_memory,
        udp_memory,
        guest_mac,
    )
}
