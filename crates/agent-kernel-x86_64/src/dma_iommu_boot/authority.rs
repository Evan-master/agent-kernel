//! Core DMA Capability graph for the native proof.

use agent_kernel_boot::BootConfig;
use agent_kernel_core::{
    ActionId, AgentId, DmaAccess, DmaMappingId, DmaRequesterId, KernelError, Operation,
    OperationSet, ResourceKind,
};

use crate::X86BootedKernel;

#[derive(Copy, Clone)]
pub(super) struct DmaBinding {
    agent: AgentId,
    domain_capability: agent_kernel_core::CapabilityId,
    mapping: DmaMappingId,
}

pub(super) fn reserve(
    source_id: u16,
    iova: u64,
) -> Result<(X86BootedKernel, DmaBinding), KernelError> {
    let config = BootConfig::new(AgentId::new(1), ResourceKind::Workspace, ActionId::new(1));
    let mut booted = X86BootedKernel::boot(config)?;
    let report = *booted.report();
    let operations = OperationSet::only(Operation::Act)
        .with(Operation::Observe)
        .with(Operation::Delegate);
    let iommu = booted.kernel_mut().sys_create_resource(
        report.bootstrap_agent,
        ResourceKind::Iommu,
        Some((report.bootstrap_resource, report.bootstrap_capability)),
        operations,
    )?;
    let device = booted.kernel_mut().sys_create_resource(
        report.bootstrap_agent,
        ResourceKind::Device,
        Some((report.bootstrap_resource, report.bootstrap_capability)),
        operations,
    )?;
    let memory = booted.kernel_mut().sys_create_resource(
        report.bootstrap_agent,
        ResourceKind::Memory,
        Some((report.bootstrap_resource, report.bootstrap_capability)),
        operations,
    )?;
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
    let mapping = booted.kernel_mut().sys_reserve_dma_mapping(
        report.bootstrap_agent,
        domain.capability,
        domain.resource,
        memory.capability,
        memory.resource,
        iova,
        1,
        DmaAccess::ReadWrite,
    )?;
    Ok((
        booted,
        DmaBinding {
            agent: report.bootstrap_agent,
            domain_capability: domain.capability,
            mapping,
        },
    ))
}

pub(super) fn activate(
    booted: &mut X86BootedKernel,
    binding: DmaBinding,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_activate_dma_mapping(
        binding.agent,
        binding.domain_capability,
        binding.mapping,
    )?;
    Ok(())
}

pub(super) fn begin_revoke(
    booted: &mut X86BootedKernel,
    binding: DmaBinding,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_begin_dma_unmap(
        binding.agent,
        binding.domain_capability,
        binding.mapping,
    )?;
    Ok(())
}

pub(super) fn complete_revoke(
    booted: &mut X86BootedKernel,
    binding: DmaBinding,
) -> Result<(), KernelError> {
    booted.kernel_mut().sys_complete_dma_unmap(
        binding.agent,
        binding.domain_capability,
        binding.mapping,
    )?;
    Ok(())
}
