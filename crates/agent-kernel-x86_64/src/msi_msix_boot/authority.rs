//! Core authority graph for the native MSI/MSI-X proof.
//!
//! This boot child owns two Device Resources, one shared DMA Domain, three
//! Memory mappings, and one interrupt route per device. Hardware transitions
//! are performed by sibling architecture owners between each two-phase call.

use agent_kernel_boot::BootConfig;
use agent_kernel_core::{
    ActionId, AgentId, DmaAccess, DmaAttachmentStatus, DmaMappingId, DmaRequesterId, InterruptMode,
    InterruptRouteStatus, InterruptTarget, KernelError, Operation, OperationSet,
    ResourceCreateOutcome, ResourceKind,
};

use crate::X86BootedKernel;

use super::{EDU_IOVA, EDU_MSI_VECTOR, RNG_ENTROPY_IOVA, RNG_MSIX_VECTOR, RNG_QUEUE_IOVA};

#[derive(Copy, Clone)]
struct DeviceAuthority {
    device: ResourceCreateOutcome,
    route: ResourceCreateOutcome,
}

#[derive(Copy, Clone)]
pub(super) struct MsiMsixAuthority {
    agent: AgentId,
    domain: ResourceCreateOutcome,
    edu: DeviceAuthority,
    rng: DeviceAuthority,
    mappings: [DmaMappingId; 3],
}

pub(super) fn reserve(
    destination: u32,
    edu_source_id: u16,
    rng_source_id: u16,
) -> Result<(X86BootedKernel, MsiMsixAuthority), KernelError> {
    let config = BootConfig::new(AgentId::new(1), ResourceKind::Workspace, ActionId::new(1));
    let mut booted = X86BootedKernel::boot(config)?;
    let report = *booted.report();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Observe)
        .with(Operation::Rollback)
        .with(Operation::Delegate);
    let iommu = child_resource(&mut booted, report, ResourceKind::Iommu, operations)?;
    let edu_device = child_resource(&mut booted, report, ResourceKind::Device, operations)?;
    let rng_device = child_resource(&mut booted, report, ResourceKind::Device, operations)?;
    let edu_memory = child_resource(&mut booted, report, ResourceKind::Memory, operations)?;
    let queue_memory = child_resource(&mut booted, report, ResourceKind::Memory, operations)?;
    let entropy_memory = child_resource(&mut booted, report, ResourceKind::Memory, operations)?;
    let domain = booted.kernel_mut().sys_create_dma_domain(
        report.bootstrap_agent,
        iommu.capability,
        iommu.resource,
        operations,
    )?;

    attach(
        &mut booted,
        report.bootstrap_agent,
        domain,
        edu_device,
        edu_source_id,
    )?;
    attach(
        &mut booted,
        report.bootstrap_agent,
        domain,
        rng_device,
        rng_source_id,
    )?;

    let mappings = [
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            edu_memory,
            EDU_IOVA,
            DmaAccess::ReadWrite,
        )?,
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            queue_memory,
            RNG_QUEUE_IOVA,
            DmaAccess::ReadWrite,
        )?,
        reserve_mapping(
            &mut booted,
            report.bootstrap_agent,
            domain,
            entropy_memory,
            RNG_ENTROPY_IOVA,
            DmaAccess::Write,
        )?,
    ];
    let edu_route = booted.kernel_mut().sys_create_interrupt_route(
        report.bootstrap_agent,
        edu_device.capability,
        edu_device.resource,
        InterruptMode::Msi,
        InterruptTarget::new(destination, EDU_MSI_VECTOR).expect("fixed EDU vector"),
        operations,
    )?;
    let rng_route = booted.kernel_mut().sys_create_interrupt_route(
        report.bootstrap_agent,
        rng_device.capability,
        rng_device.resource,
        InterruptMode::MsiX { table_entry: 0 },
        InterruptTarget::new(destination, RNG_MSIX_VECTOR).expect("fixed RNG vector"),
        operations,
    )?;

    Ok((
        booted,
        MsiMsixAuthority {
            agent: report.bootstrap_agent,
            domain,
            edu: DeviceAuthority {
                device: edu_device,
                route: edu_route,
            },
            rng: DeviceAuthority {
                device: rng_device,
                route: rng_route,
            },
            mappings,
        },
    ))
}

impl MsiMsixAuthority {
    pub(super) fn activate(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        for mapping in self.mappings {
            booted.kernel_mut().sys_activate_dma_mapping(
                self.agent,
                self.domain.capability,
                mapping,
            )?;
        }
        activate_route(booted, self.agent, self.edu)?;
        activate_route(booted, self.agent, self.rng)
    }

    pub(super) fn has_two_active_attachments(self, booted: &X86BootedKernel) -> bool {
        booted
            .kernel()
            .dma_attachments()
            .iter()
            .filter(|record| {
                record.domain == self.domain.resource
                    && record.status() == DmaAttachmentStatus::Attached
            })
            .count()
            == 2
    }

    pub(super) fn begin_rng_detach(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        begin_route_revoke(booted, self.agent, self.rng)?;
        booted.kernel_mut().sys_begin_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.rng.device.capability,
            self.rng.device.resource,
        )?;
        Ok(())
    }

    pub(super) fn complete_rng_detach(
        self,
        booted: &mut X86BootedKernel,
    ) -> Result<(), KernelError> {
        booted.kernel_mut().sys_complete_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.rng.device.capability,
            self.rng.device.resource,
        )?;
        complete_route_revoke(booted, self.agent, self.rng)
    }

    pub(super) fn begin_edu_detach(self, booted: &mut X86BootedKernel) -> Result<(), KernelError> {
        begin_route_revoke(booted, self.agent, self.edu)?;
        booted.kernel_mut().sys_begin_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.edu.device.capability,
            self.edu.device.resource,
        )?;
        Ok(())
    }

    pub(super) fn complete_edu_detach(
        self,
        booted: &mut X86BootedKernel,
    ) -> Result<(), KernelError> {
        booted.kernel_mut().sys_complete_dma_device_detach(
            self.agent,
            self.domain.capability,
            self.domain.resource,
            self.edu.device.capability,
            self.edu.device.resource,
        )?;
        complete_route_revoke(booted, self.agent, self.edu)
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

    pub(super) fn routes_released(self, booted: &X86BootedKernel) -> bool {
        [self.edu.route.resource, self.rng.route.resource]
            .into_iter()
            .all(|route| {
                booted
                    .kernel()
                    .interrupt_route(route)
                    .is_ok_and(|record| record.status() == InterruptRouteStatus::Released)
            })
    }

    pub(super) fn rng_detached_with_edu_survivor(self, booted: &X86BootedKernel) -> bool {
        let mut edu_attached = false;
        let mut rng_detached = false;
        for record in booted.kernel().dma_attachments() {
            if record.domain != self.domain.resource {
                continue;
            }
            if record.device == self.edu.device.resource {
                edu_attached = record.status() == DmaAttachmentStatus::Attached;
            }
            if record.device == self.rng.device.resource {
                rng_detached = record.status() == DmaAttachmentStatus::Detached;
            }
        }
        edu_attached && rng_detached
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

fn attach(
    booted: &mut X86BootedKernel,
    agent: AgentId,
    domain: ResourceCreateOutcome,
    device: ResourceCreateOutcome,
    source_id: u16,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_attach_dma_device(
        agent,
        domain.capability,
        domain.resource,
        device.capability,
        device.resource,
        DmaRequesterId::new(u32::from(source_id)),
    )?;
    Ok(())
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

fn activate_route(
    booted: &mut X86BootedKernel,
    agent: AgentId,
    device: DeviceAuthority,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_activate_interrupt_route(
        agent,
        device.route.capability,
        device.route.resource,
    )?;
    Ok(())
}

fn begin_route_revoke(
    booted: &mut X86BootedKernel,
    agent: AgentId,
    device: DeviceAuthority,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_begin_interrupt_route_revoke(
        agent,
        device.route.capability,
        device.route.resource,
    )?;
    Ok(())
}

fn complete_route_revoke(
    booted: &mut X86BootedKernel,
    agent: AgentId,
    device: DeviceAuthority,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_complete_interrupt_route_revoke(
        agent,
        device.route.capability,
        device.route.resource,
    )?;
    Ok(())
}
