//! Two-invocation ARP and UDP proof plus complete authority teardown.

mod invocation;

use agent_kernel_core::{
    DeviceEventPayload, DmaMappingStatus, DriverCommandKind, DriverCommandPayload,
    DriverCommandResult, NetworkDatagramDescriptor, NetworkIpv4Address, NetworkMacAddress,
    NetworkTransferDirection, NetworkTransferStatus, NetworkUdpPort,
};
use agent_kernel_x86_64::{
    iommu::{IntelVtd, VolatileVtdMmio, VtdDomainId, VtdLegacyTablePages},
    virtio_net::{
        build_arp_request, build_udp_ipv4_frame, UdpIpv4Route, ARP_FRAME_BYTES,
        QEMU_USER_GATEWAY_IPV4, QEMU_USER_GUEST_IPV4,
    },
};
use sha2::{Digest, Sha256};

use crate::{
    agent_cpu::AgentCpuRuntime, agent_memory::PreparedAgentMemory,
    boot_agent_images::BootNetworkDriverImage, exit_qemu, halt_forever,
    native_agent_runtime::NativeAgentRuntime, native_driver_executor::DriverRecoveryAuthority,
    serial_write_line, smp_boot::SmpBootstrap, X86BootedKernel,
};

use super::{
    super::{
        authority::NativeNetAuthority,
        network_proof::frame_descriptor,
        pci::PreparedNativeNetHardware,
        proof::{fatal_after_enable, publish_dma_memory, require_no_fault},
        NET_RX_METADATA_IOVA, NET_RX_PACKET_IOVA, NET_TX_METADATA_IOVA, NET_TX_PACKET_IOVA,
    },
    admission::NetworkDriverAdmission,
    backend::NetworkDriverBackend,
    platform::NativeUdpNet,
    NETWORK_COMMAND_EXCHANGE_UDP, NETWORK_COMMAND_RESOLVE_NEIGHBOR, NETWORK_RESULT_OK,
    UDP_DESTINATION_PORT, UDP_PAYLOAD, UDP_SOURCE_PORT,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    booted: &mut X86BootedKernel,
    authority: NativeNetAuthority,
    driver: NetworkDriverAdmission,
    contract: BootNetworkDriverImage,
    cpu_runtime: AgentCpuRuntime,
    neighbor_memory: PreparedAgentMemory,
    udp_memory: PreparedAgentMemory,
    guest_mac: NetworkMacAddress,
    smp: &mut SmpBootstrap,
    hardware: &mut PreparedNativeNetHardware,
    net: &mut NativeUdpNet<'_>,
    iommu: &mut IntelVtd<VolatileVtdMmio>,
    tables: &mut VtdLegacyTablePages<'_>,
) -> ! {
    let device = authority.device();
    let report = *booted.report();
    let recovery_authority =
        DriverRecoveryAuthority::new(report.bootstrap_agent, device.capability).unwrap_or_else(
            || fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_DRIVER_ERROR"),
        );
    let mut runtime = NativeAgentRuntime::new();
    let mut backend = NetworkDriverBackend::new(net, smp, guest_mac);

    let mut arp = [0; ARP_FRAME_BYTES];
    build_arp_request(&mut arp, guest_mac);
    let arp_transfer = authority
        .prepare_transmit(booted, frame_descriptor(&arp))
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ARP_AUTHORITY_ERROR")
        });
    let neighbor = match invocation::run(
        booted,
        &mut runtime,
        cpu_runtime,
        neighbor_memory,
        device,
        driver,
        contract,
        recovery_authority,
        DeviceEventPayload {
            code: NETWORK_COMMAND_RESOLVE_NEIGHBOR,
            value: 0,
        },
        &mut backend,
    ) {
        Ok(evidence) => evidence,
        Err(invocation::InvocationError::Executor) => {
            fatal_after_enable(hardware, backend.failure_marker())
        }
        Err(error) => fatal_after_enable(hardware, error.diagnostic_marker()),
    };
    verify_command(
        booted,
        driver,
        &neighbor,
        DriverCommandKind::Configure,
        DriverCommandPayload {
            opcode: NETWORK_COMMAND_RESOLVE_NEIGHBOR,
            value: 0,
        },
    )
    .unwrap_or_else(|| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_NEIGHBOR_EVIDENCE_ERROR")
    });
    authority
        .complete_transmit(booted, arp_transfer)
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ARP_AUTHORITY_ERROR")
        });
    authority
        .record_receive(
            booted,
            backend.arp_receive().unwrap_or_else(|| {
                fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ARP_REPLY_ERROR")
            }),
        )
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ARP_AUTHORITY_ERROR")
        });
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_NEIGHBOR_DRIVER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_ARP_REPLY_OK");

    let gateway_mac = backend.gateway_mac().unwrap_or_else(|| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_GATEWAY_MAC_ERROR")
    });
    let (outbound, inbound) = datagram_descriptors();
    let mut udp_frame = [0; 60];
    let udp_length = build_udp_ipv4_frame(
        &mut udp_frame,
        UdpIpv4Route::new(
            guest_mac,
            gateway_mac,
            outbound.source(),
            outbound.destination(),
            outbound.source_port(),
            outbound.destination_port(),
        ),
        0x3002,
        UDP_PAYLOAD,
    )
    .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ENCODE_ERROR"));
    let udp_transfer = authority
        .prepare_datagram_transmit(booted, frame_descriptor(&udp_frame[..udp_length]), outbound)
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_DATAGRAM_AUTHORITY_ERROR")
        });
    let exchange = match invocation::run(
        booted,
        &mut runtime,
        cpu_runtime,
        udp_memory,
        device,
        driver,
        contract,
        recovery_authority,
        DeviceEventPayload {
            code: NETWORK_COMMAND_EXCHANGE_UDP,
            value: UDP_PAYLOAD.len() as u64,
        },
        &mut backend,
    ) {
        Ok(evidence) => evidence,
        Err(invocation::InvocationError::Executor) => {
            fatal_after_enable(hardware, backend.failure_marker())
        }
        Err(error) => fatal_after_enable(hardware, error.diagnostic_marker()),
    };
    verify_command(
        booted,
        driver,
        &exchange,
        DriverCommandKind::Write,
        DriverCommandPayload {
            opcode: NETWORK_COMMAND_EXCHANGE_UDP,
            value: UDP_PAYLOAD.len() as u64,
        },
    )
    .unwrap_or_else(|| {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_EXCHANGE_EVIDENCE_ERROR")
    });
    authority
        .complete_transmit(booted, udp_transfer)
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_DATAGRAM_AUTHORITY_ERROR")
        });
    let receive = authority
        .record_datagram_receive(
            booted,
            backend.udp_receive().unwrap_or_else(|| {
                fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_ECHO_ERROR")
            }),
            inbound,
        )
        .unwrap_or_else(|_| {
            fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_DATAGRAM_AUTHORITY_ERROR")
        });
    let datagram_evidence = match (
        booted.kernel().network_transfer(udp_transfer),
        booted.kernel().network_transfer(receive),
    ) {
        (Ok(transmit), Ok(received)) => {
            transmit.endpoint() == authority.endpoint_resource()
                && transmit.direction() == NetworkTransferDirection::Transmit
                && transmit.status() == NetworkTransferStatus::Completed
                && transmit.datagram() == Some(outbound)
                && received.endpoint() == authority.endpoint_resource()
                && received.direction() == NetworkTransferDirection::Receive
                && received.status() == NetworkTransferStatus::Completed
                && received.datagram() == Some(inbound)
        }
        _ => false,
    };
    if backend.command_count() != 2 || !datagram_evidence {
        fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_CORE_EVIDENCE_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_EXCHANGE_DRIVER_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_ECHO_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_MSIX_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_CORE_EVIDENCE_OK");
    drop(backend);

    teardown(booted, authority, hardware, net, iommu, tables);
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_PROOF_OK");
    exit_qemu(0x10);
    halt_forever()
}

fn datagram_descriptors() -> (NetworkDatagramDescriptor, NetworkDatagramDescriptor) {
    let guest = NetworkIpv4Address::new(QEMU_USER_GUEST_IPV4).expect("fixed guest IPv4");
    let gateway = NetworkIpv4Address::new(QEMU_USER_GATEWAY_IPV4).expect("fixed gateway IPv4");
    let guest_port = NetworkUdpPort::new(UDP_SOURCE_PORT).expect("fixed guest port");
    let echo_port = NetworkUdpPort::new(UDP_DESTINATION_PORT).expect("fixed echo port");
    let digest: [u8; 32] = Sha256::digest(UDP_PAYLOAD).into();
    (
        NetworkDatagramDescriptor::new(
            guest,
            gateway,
            guest_port,
            echo_port,
            UDP_PAYLOAD.len() as u16,
            digest,
        )
        .expect("fixed outbound datagram"),
        NetworkDatagramDescriptor::new(
            gateway,
            guest,
            echo_port,
            guest_port,
            UDP_PAYLOAD.len() as u16,
            digest,
        )
        .expect("fixed inbound datagram"),
    )
}

fn verify_command(
    booted: &X86BootedKernel,
    driver: NetworkDriverAdmission,
    evidence: &invocation::InvocationEvidence,
    kind: DriverCommandKind,
    payload: DriverCommandPayload,
) -> Option<()> {
    let command = booted
        .kernel()
        .driver_commands()
        .iter()
        .find(|record| record.id == evidence.command)?;
    let expected_result = DriverCommandResult {
        code: NETWORK_RESULT_OK,
        value: payload.value,
    };
    (command.binding == driver.binding
        && command.cause == Some(evidence.event)
        && command.invocation == Some(evidence.invocation)
        && command.kind == kind
        && command.payload == payload
        && command.result == Some(expected_result)
        && evidence.result == expected_result)
        .then_some(())
}

fn teardown(
    booted: &mut X86BootedKernel,
    authority: NativeNetAuthority,
    hardware: &mut PreparedNativeNetHardware,
    net: &mut NativeUdpNet<'_>,
    iommu: &mut IntelVtd<VolatileVtdMmio>,
    tables: &mut VtdLegacyTablePages<'_>,
) {
    require_no_fault(hardware, iommu);
    hardware
        .disable_msix()
        .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    net.shutdown()
        .unwrap_or_else(|_| fatal_after_enable(hardware, "AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    hardware
        .quiesce()
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    authority
        .begin_release(booted)
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_RELEASE_ERROR"));
    let domain = VtdDomainId::new(1).expect("fixed nonzero VT-d domain");
    tables
        .detach_requester(hardware.requester(), domain)
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    authority
        .complete_release(booted)
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_RELEASE_ERROR"));
    authority
        .begin_mapping_revoke(booted)
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    for iova in [
        NET_RX_METADATA_IOVA,
        NET_RX_PACKET_IOVA,
        NET_TX_METADATA_IOVA,
        NET_TX_PACKET_IOVA,
    ] {
        tables
            .remove_mapping(iova)
            .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    }
    publish_dma_memory();
    iommu
        .invalidate()
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    authority
        .complete_mapping_revoke(booted)
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    iommu
        .disable()
        .unwrap_or_else(|_| crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR"));
    if !authority.released(booted)
        || tables.active_requester_count() != 0
        || booted
            .kernel()
            .dma_mappings()
            .iter()
            .any(|record| record.status != DmaMappingStatus::Released)
    {
        crate::fatal_boot("AGENT_KERNEL_NATIVE_UDP_TEARDOWN_ERROR");
    }
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_ENDPOINT_RELEASED_OK");
    serial_write_line("AGENT_KERNEL_NATIVE_UDP_VTD_TEARDOWN_OK");
}
