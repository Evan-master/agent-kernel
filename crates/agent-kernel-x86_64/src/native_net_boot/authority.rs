//! Core authority graph shared by the V29 and V30 network proofs.
//!
//! This boot child owns one Device, four DMA mappings, two MSI-X routes, and
//! one Network endpoint. Hardware siblings execute every two-phase transition.

use agent_kernel_boot::BootConfig;
use agent_kernel_core::{
    ActionId, AgentId, DmaAccess, DmaAttachmentStatus, DmaMappingId, DmaRequesterId, InterruptMode,
    InterruptRouteStatus, InterruptTarget, KernelError, NetworkDatagramDescriptor,
    NetworkEndpointConfig, NetworkEndpointStatus, NetworkFrameDescriptor, NetworkMacAddress,
    NetworkTransferId, Operation, OperationSet, ResourceCreateOutcome, ResourceId, ResourceKind,
};

use crate::X86BootedKernel;

use super::{
    NET_RX_METADATA_IOVA, NET_RX_MSIX_VECTOR, NET_RX_PACKET_IOVA, NET_TX_METADATA_IOVA,
    NET_TX_MSIX_VECTOR, NET_TX_PACKET_IOVA,
};

#[derive(Copy, Clone)]
pub(super) struct NativeNetAuthority {
    agent: AgentId,
    domain: ResourceCreateOutcome,
    device: ResourceCreateOutcome,
    routes: [ResourceCreateOutcome; 2],
    endpoint: ResourceCreateOutcome,
    mappings: [DmaMappingId; 4],
}

pub(super) fn reserve(
    destination: u32,
    source_id: u16,
    mac: NetworkMacAddress,
) -> Result<(X86BootedKernel, NativeNetAuthority), KernelError> {
    reserve_with_device_verification(destination, source_id, mac, false)
}

pub(super) fn reserve_for_driver(
    destination: u32,
    source_id: u16,
    mac: NetworkMacAddress,
) -> Result<(X86BootedKernel, NativeNetAuthority), KernelError> {
    reserve_with_device_verification(destination, source_id, mac, true)
}

fn reserve_with_device_verification(
    destination: u32,
    source_id: u16,
    mac: NetworkMacAddress,
    verify_device_images: bool,
) -> Result<(X86BootedKernel, NativeNetAuthority), KernelError> {
    let config = BootConfig::new(AgentId::new(1), ResourceKind::Workspace, ActionId::new(1));
    let mut booted = X86BootedKernel::boot(config)?;
    let report = *booted.report();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Observe)
        .with(Operation::Rollback)
        .with(Operation::Delegate);
    let device_operations = if verify_device_images {
        operations.with(Operation::Verify)
    } else {
        operations
    };
    let iommu = child_resource(&mut booted, report, ResourceKind::Iommu, operations)?;
    let device = child_resource(&mut booted, report, ResourceKind::Device, device_operations)?;
    let memories = [
        child_resource(&mut booted, report, ResourceKind::Memory, operations)?,
        child_resource(&mut booted, report, ResourceKind::Memory, operations)?,
        child_resource(&mut booted, report, ResourceKind::Memory, operations)?,
        child_resource(&mut booted, report, ResourceKind::Memory, operations)?,
    ];
    let domain = booted.kernel_mut().sys_create_dma_domain(
        report.bootstrap_agent,
        iommu.capability,
        iommu.resource,
        operations,
    )?;
    booted.kernel_mut().sys_attach_dma_device(
        report.bootstrap_agent,
        domain.capability,
        domain.resource,
        device.capability,
        device.resource,
        DmaRequesterId::new(u32::from(source_id)),
    )?;
    let mappings = [
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            memories[0],
            NET_RX_METADATA_IOVA,
            DmaAccess::ReadWrite,
        )?,
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            memories[1],
            NET_RX_PACKET_IOVA,
            DmaAccess::Write,
        )?,
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            memories[2],
            NET_TX_METADATA_IOVA,
            DmaAccess::ReadWrite,
        )?,
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            memories[3],
            NET_TX_PACKET_IOVA,
            DmaAccess::Read,
        )?,
    ];
    let routes = [
        booted.kernel_mut().sys_create_interrupt_route(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            InterruptMode::MsiX { table_entry: 0 },
            InterruptTarget::new(destination, NET_RX_MSIX_VECTOR).expect("fixed Rx vector"),
            operations,
        )?,
        booted.kernel_mut().sys_create_interrupt_route(
            report.bootstrap_agent,
            device.capability,
            device.resource,
            InterruptMode::MsiX { table_entry: 1 },
            InterruptTarget::new(destination, NET_TX_MSIX_VECTOR).expect("fixed Tx vector"),
            operations,
        )?,
    ];
    let endpoint = booted.kernel_mut().sys_create_network_endpoint(
        report.bootstrap_agent,
        device.capability,
        device.resource,
        NetworkEndpointConfig::new(mac, 1500)?,
        operations,
    )?;

    Ok((
        booted,
        NativeNetAuthority {
            agent: report.bootstrap_agent,
            domain,
            device,
            routes,
            endpoint,
            mappings,
        },
    ))
}

impl NativeNetAuthority {
    pub(super) const fn device(self) -> ResourceCreateOutcome {
        self.device
    }

    pub(super) const fn endpoint_resource(self) -> ResourceId {
        self.endpoint.resource
    }

    pub(super) const fn endpoint_capability(self) -> agent_kernel_core::CapabilityId {
        self.endpoint.capability
    }

    pub(super) fn activate(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        for mapping in self.mappings {
            booted.kernel_mut().sys_activate_dma_mapping(
                self.agent,
                self.domain.capability,
                mapping,
            )?;
        }
        for route in self.routes {
            booted.kernel_mut().sys_activate_interrupt_route(
                self.agent,
                route.capability,
                route.resource,
            )?;
        }
        booted.kernel_mut().sys_activate_network_endpoint(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
        )?;
        Ok(())
    }

    pub(super) fn prepare_transmit(
        self,
        booted: &mut X86BootedKernel,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        booted.kernel_mut().sys_prepare_network_transmit(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
            frame,
        )
    }

    pub(super) fn complete_transmit(
        self,
        booted: &mut X86BootedKernel,
        transfer: NetworkTransferId,
    ) -> Result<(), KernelError> {
        booted.kernel_mut().sys_complete_network_transmit(
            self.agent,
            self.endpoint.capability,
            transfer,
        )
    }

    pub(super) fn prepare_datagram_transmit(
        self,
        booted: &mut X86BootedKernel,
        frame: NetworkFrameDescriptor,
        datagram: NetworkDatagramDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        booted.kernel_mut().sys_prepare_network_datagram_transmit(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
            frame,
            datagram,
        )
    }

    pub(super) fn record_receive(
        self,
        booted: &mut X86BootedKernel,
        frame: NetworkFrameDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        booted.kernel_mut().sys_record_network_receive(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
            frame,
        )
    }

    pub(super) fn record_datagram_receive(
        self,
        booted: &mut X86BootedKernel,
        frame: NetworkFrameDescriptor,
        datagram: NetworkDatagramDescriptor,
    ) -> Result<NetworkTransferId, KernelError> {
        booted.kernel_mut().sys_record_network_datagram_receive(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
            frame,
            datagram,
        )
    }

    pub(super) fn begin_release(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        booted.kernel_mut().sys_begin_network_endpoint_revoke(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
        )?;
        for route in self.routes {
            booted.kernel_mut().sys_begin_interrupt_route_revoke(
                self.agent,
                route.capability,
                route.resource,
            )?;
        }
        booted.kernel_mut().sys_begin_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.device.capability,
            self.device.resource,
        )?;
        Ok(())
    }

    pub(super) fn complete_release(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        booted.kernel_mut().sys_complete_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.device.capability,
            self.device.resource,
        )?;
        for route in self.routes {
            booted.kernel_mut().sys_complete_interrupt_route_revoke(
                self.agent,
                route.capability,
                route.resource,
            )?;
        }
        booted.kernel_mut().sys_complete_network_endpoint_revoke(
            self.agent,
            self.endpoint.capability,
            self.endpoint.resource,
        )?;
        Ok(())
    }

    pub(super) fn begin_mapping_revoke(
        self,
        booted: &mut X86BootedKernel,
    ) -> Result<(), KernelError> {
        for mapping in self.mappings {
            booted
                .kernel_mut()
                .sys_begin_dma_unmap(self.agent, self.domain.capability, mapping)?;
        }
        Ok(())
    }

    pub(super) fn complete_mapping_revoke(
        self,
        booted: &mut X86BootedKernel,
    ) -> Result<(), KernelError> {
        for mapping in self.mappings {
            booted.kernel_mut().sys_complete_dma_unmap(
                self.agent,
                self.domain.capability,
                mapping,
            )?;
        }
        Ok(())
    }

    pub(super) fn released(self, booted: &X86BootedKernel) -> bool {
        let endpoint_released = booted
            .kernel()
            .network_endpoint(self.endpoint.resource)
            .is_ok_and(|record| record.status() == NetworkEndpointStatus::Released);
        let routes_released = self.routes.into_iter().all(|route| {
            booted
                .kernel()
                .interrupt_route(route.resource)
                .is_ok_and(|record| record.status() == InterruptRouteStatus::Released)
        });
        let detached = booted.kernel().dma_attachments().iter().any(|record| {
            record.domain == self.domain.resource
                && record.device == self.device.resource
                && record.status() == DmaAttachmentStatus::Detached
        });
        endpoint_released && routes_released && detached
    }
}

fn child_resource(
    booted: &mut X86BootedKernel,
    report: agent_kernel_boot::BootReport,
    kind: ResourceKind,
    operations: OperationSet,
) -> Result<ResourceCreateOutcome, KernelError> {
    booted.kernel_mut().sys_create_resource(
        report.bootstrap_agent,
        kind,
        Some((report.bootstrap_resource, report.bootstrap_capability)),
        operations,
    )
}

fn reserve_mapping(
    booted: &mut X86BootedKernel,
    agent: AgentId,
    domain: ResourceCreateOutcome,
    memory: ResourceCreateOutcome,
    iova: u64,
    access: DmaAccess,
) -> Result<DmaMappingId, KernelError> {
    booted.kernel_mut().sys_reserve_dma_mapping(
        agent,
        domain.capability,
        domain.resource,
        memory.capability,
        memory.resource,
        iova,
        1,
        access,
    )
}
