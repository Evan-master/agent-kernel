//! Backend outcome, terminal syscall, and final record validation phase.

use agent_kernel_core::{
    AgentExecutionState, AgentId, CapabilityId, DeviceEventStatus, DriverCommandId,
    DriverCommandStatus, DriverInvocationStatus,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

use super::{PortCommandFlow, DRIVER};
use crate::X86BootedKernel;

impl PortCommandFlow {
    pub fn execute_and_record(&mut self, booted: &mut X86BootedKernel) -> bool {
        let outcome = self.backend.execute(self.request);
        let result = outcome.result();
        if !record_command_outcome(
            booted,
            DRIVER,
            self.driver_capability,
            self.command,
            outcome,
        ) || !matches!(outcome, DriverCommandOutcome::Completed(_))
        {
            return false;
        }
        if booted
            .kernel_mut()
            .sys_complete_driver_invocation(DRIVER, self.driver_capability, self.invocation)
            .is_err()
        {
            return false;
        }

        let kernel = booted.kernel();
        let command = kernel
            .driver_commands()
            .iter()
            .find(|record| record.id == self.command);
        let event = kernel
            .device_events()
            .iter()
            .find(|record| record.id == self.event);
        let invocation = kernel
            .driver_invocations()
            .iter()
            .find(|record| record.id == self.invocation);
        let context = kernel
            .execution_contexts()
            .iter()
            .find(|context| context.agent == DRIVER);

        command.is_some_and(|record| {
            record.status == DriverCommandStatus::Completed && record.result == Some(result)
        }) && event.is_some_and(|record| record.status == DeviceEventStatus::Acknowledged)
            && invocation.is_some_and(|record| {
                record.status == DriverInvocationStatus::Completed && record.run_ticks == 1
            })
            && context.is_some_and(|context| context.state == AgentExecutionState::Idle)
    }
}

pub(crate) fn record_command_outcome(
    booted: &mut X86BootedKernel,
    driver: AgentId,
    capability: CapabilityId,
    command: DriverCommandId,
    outcome: DriverCommandOutcome,
) -> bool {
    let result = outcome.result();
    let (status, transition) = match outcome {
        DriverCommandOutcome::Completed(_) => (
            DriverCommandStatus::Completed,
            booted
                .kernel_mut()
                .sys_complete_driver_command(driver, capability, command, result),
        ),
        DriverCommandOutcome::Failed(_) => (
            DriverCommandStatus::Failed,
            booted
                .kernel_mut()
                .sys_fail_driver_command(driver, capability, command, result),
        ),
    };
    if transition.is_err() {
        return false;
    }

    booted
        .kernel()
        .driver_commands()
        .iter()
        .find(|record| record.id == command)
        .is_some_and(|record| record.status == status && record.result == Some(result))
}
