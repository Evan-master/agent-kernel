//! Runtime admission syscall facade.

use agent_kernel_core::{
    AgentId, CapabilityId, KernelError, RuntimeAdmissionFailure, RuntimeAdmissionId,
    RuntimeAdmissionPermit, RuntimeAdmissionRecord, TaskId,
};

use crate::AgentKernel;

impl<
        const AGENTS: usize,
        const RESOURCES: usize,
        const CAPS: usize,
        const EVENTS: usize,
        const ACTIONS: usize,
        const OBSERVATIONS: usize,
        const CHECKPOINTS: usize,
        const INTENTS: usize,
        const TASKS: usize,
        const RUN_QUEUE: usize,
        const MESSAGES: usize,
        const MEMORY_CELLS: usize,
        const NAMESPACE_ENTRIES: usize,
        const FAULTS: usize,
        const FAULT_HANDLERS: usize,
        const FAULT_POLICIES: usize,
        const WAITERS: usize,
        const AGENT_IMAGES: usize,
        const DRIVER_BINDINGS: usize,
        const DEVICE_EVENTS: usize,
        const DRIVER_COMMANDS: usize,
        const DRIVER_INVOCATIONS: usize,
    >
    AgentKernel<
        AGENTS,
        RESOURCES,
        CAPS,
        EVENTS,
        ACTIONS,
        OBSERVATIONS,
        CHECKPOINTS,
        INTENTS,
        TASKS,
        RUN_QUEUE,
        MESSAGES,
        MEMORY_CELLS,
        NAMESPACE_ENTRIES,
        FAULTS,
        FAULT_HANDLERS,
        FAULT_POLICIES,
        WAITERS,
        AGENT_IMAGES,
        DRIVER_BINDINGS,
        DEVICE_EVENTS,
        DRIVER_COMMANDS,
        DRIVER_INVOCATIONS,
    >
{
    pub fn sys_request_runtime_admission(
        &mut self,
        requester: AgentId,
        authority: CapabilityId,
        target: AgentId,
        task: TaskId,
    ) -> Result<RuntimeAdmissionId, KernelError> {
        self.core
            .request_runtime_admission(requester, authority, target, task)
    }

    pub fn sys_prepare_next_runtime_admission(
        &self,
    ) -> Result<RuntimeAdmissionPermit, KernelError> {
        self.core.prepare_next_runtime_admission()
    }

    pub fn sys_commit_runtime_admission(
        &mut self,
        permit: RuntimeAdmissionPermit,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        self.core.commit_runtime_admission(permit)
    }

    pub fn sys_reject_runtime_admission(
        &mut self,
        permit: RuntimeAdmissionPermit,
        failure: RuntimeAdmissionFailure,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        self.core.reject_runtime_admission(permit, failure)
    }

    pub fn runtime_admissions(&self) -> &[RuntimeAdmissionRecord] {
        self.core.runtime_admissions()
    }

    pub fn runtime_admission(
        &self,
        admission: RuntimeAdmissionId,
    ) -> Result<RuntimeAdmissionRecord, KernelError> {
        self.core.runtime_admission(admission)
    }
}
