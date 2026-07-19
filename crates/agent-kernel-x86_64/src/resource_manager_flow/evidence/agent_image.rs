//! Terminal state and Event proof for Resource Manager Image reclamation.

use agent_kernel_core::{AgentImageKind, Event, EventKind, KernelError, Operation};

use super::super::{ResourceManagerTask, RESOURCE_MANAGER};
use crate::{boot_agent_images::BootResourceManagerImage, X86BootedKernel};

pub(super) fn state_valid(
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    manager.retired_image == image.retired_image()
        && booted.kernel().agent_images().len() == 8
        && booted
            .kernel()
            .agent_images()
            .iter()
            .enumerate()
            .all(|(index, record)| record.id.raw() == index as u64 + 1)
        && booted.kernel().agent_image(manager.retired_image)
            == Err(KernelError::AgentImageNotFound)
}

pub(super) fn events_valid(
    event: &Event,
    booted: &X86BootedKernel,
    manager: ResourceManagerTask,
    image: BootResourceManagerImage,
) -> bool {
    event.kind == EventKind::AgentImageRecordRetired
        && event.agent == RESOURCE_MANAGER
        && event.target_agent == Some(booted.report().bootstrap_agent)
        && event.resource == Some(booted.report().bootstrap_resource)
        && event.capability == Some(manager.resource_authority)
        && event.source_capability.is_none()
        && event.operation == Some(Operation::Rollback)
        && event.agent_image == Some(manager.retired_image)
        && event.agent_image_kind == Some(AgentImageKind::Worker)
        && event.agent_image_digest == Some(image.retired_image_digest())
        && event.agent_image_abi_version == Some(1)
        && event.agent_image_entry_version == Some(1)
}
