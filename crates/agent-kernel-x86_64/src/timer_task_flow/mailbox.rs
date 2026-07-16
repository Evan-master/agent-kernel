//! Type-state transitions for the two-Worker mailbox exchange.
//!
//! This boot-semantic child orders CPU evidence and public facade mutations;
//! each state can advance only once through the fixed V0 workflow.

use crate::{
    agent_cpu::{
        AcknowledgedMessageAcknowledgementCpu, AcknowledgedMessageReceiveCpu,
        AcknowledgedMessageSendCpu, AcknowledgedReceiverResultCpu, AcknowledgedSenderResultCpu,
        CompletedMailboxReceiverCpu, CompletedMailboxSenderCpu, RequestedMessageAcknowledgementCpu,
        RequestedMessageReceiveCpu, RequestedMessageSendCpu, RequestedReceiverResultCpu,
        RequestedSenderResultCpu, RequestedSenderYieldCpu, WaitingMessageReceiveCpu,
    },
    X86BootedKernel,
};

use super::{
    message_transition, result_transition, transitions, wait_transition, yield_transition,
    CompletedWorkerTasks, FirstMessageSentFlow, FirstResultSubmittedFlow, FirstResumedFlow,
    FirstYieldRedispatchedFlow, SecondMessageAcknowledgedFlow, SecondMessageReceivedFlow,
    SecondRedispatchedFlow, SecondResultSubmittedFlow, SecondResumedFlow, SecondWaitingFlow,
};

impl SecondResumedFlow {
    pub(crate) fn wait_for_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedMessageReceiveCpu,
    ) -> Option<(SecondWaitingFlow, WaitingMessageReceiveCpu)> {
        let (waiter, waiting) = wait_transition::wait(booted, self.second, self.first, cpu)?;
        Some((
            SecondWaitingFlow {
                first: self.first,
                second: self.second,
                waiter,
            },
            waiting,
        ))
    }
}

impl SecondWaitingFlow {
    pub(crate) fn dispatch_first(
        self,
        booted: &mut X86BootedKernel,
        runtime: &crate::native_agent_runtime::NativeAgentRuntime,
    ) -> Option<(FirstResumedFlow, agent_kernel_core::RunQueueEntry)> {
        let dispatched = wait_transition::dispatch_sender(
            booted,
            self.first,
            self.second,
            self.waiter,
            runtime,
        )?;
        Some((
            FirstResumedFlow {
                first: self.first,
                second: self.second,
                waiter: self.waiter,
            },
            dispatched,
        ))
    }
}

impl FirstResumedFlow {
    pub(crate) fn submit_first_result(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedSenderResultCpu,
    ) -> Option<(FirstResultSubmittedFlow, AcknowledgedSenderResultCpu)> {
        let acknowledged = result_transition::submit(
            booted,
            self.first,
            None,
            Some((self.second, self.waiter)),
            None,
            cpu,
        )?;
        Some((
            FirstResultSubmittedFlow {
                first: self.first,
                second: self.second,
                waiter: self.waiter,
            },
            acknowledged,
        ))
    }
}

impl FirstResultSubmittedFlow {
    pub(crate) fn send_to_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedMessageSendCpu,
    ) -> Option<(FirstMessageSentFlow, AcknowledgedMessageSendCpu)> {
        let acknowledged =
            message_transition::send(booted, self.first, self.second, self.waiter, cpu)?;
        Some((
            FirstMessageSentFlow {
                first: self.first,
                second: self.second,
                waiter: self.waiter,
            },
            acknowledged,
        ))
    }
}

impl FirstMessageSentFlow {
    pub(crate) fn yield_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedSenderYieldCpu,
        runtime: &mut crate::native_agent_runtime::NativeAgentRuntime,
    ) -> Option<(SecondRedispatchedFlow, agent_kernel_core::RunQueueEntry)> {
        let dispatched =
            yield_transition::yield_and_dispatch(booted, self.first, self.second, cpu, runtime)?;
        Some((
            SecondRedispatchedFlow {
                first: self.first,
                second: self.second,
                waiter: self.waiter,
            },
            dispatched,
        ))
    }
}

impl SecondRedispatchedFlow {
    pub(crate) fn receive_from_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: WaitingMessageReceiveCpu,
    ) -> Option<(SecondMessageReceivedFlow, AcknowledgedMessageReceiveCpu)> {
        let acknowledged =
            message_transition::receive(booted, self.second, self.first, self.waiter, cpu)?;
        Some((
            SecondMessageReceivedFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl SecondMessageReceivedFlow {
    pub(crate) fn acknowledge_from_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedMessageAcknowledgementCpu,
    ) -> Option<(
        SecondMessageAcknowledgedFlow,
        AcknowledgedMessageAcknowledgementCpu,
    )> {
        let acknowledged = message_transition::acknowledge(booted, self.second, self.first, cpu)?;
        Some((
            SecondMessageAcknowledgedFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl SecondMessageAcknowledgedFlow {
    pub(crate) fn submit_second_result(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedReceiverResultCpu,
    ) -> Option<(SecondResultSubmittedFlow, AcknowledgedReceiverResultCpu)> {
        let acknowledged =
            result_transition::submit(booted, self.second, Some(self.first), None, None, cpu)?;
        Some((
            SecondResultSubmittedFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl SecondResultSubmittedFlow {
    pub(crate) fn complete_second_and_dispatch_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedMailboxReceiverCpu,
        runtime: &crate::native_agent_runtime::NativeAgentRuntime,
    ) -> Option<(FirstYieldRedispatchedFlow, agent_kernel_core::RunQueueEntry)> {
        let dispatched = transitions::complete_and_dispatch(
            booted,
            self.second,
            self.first,
            cpu,
            runtime,
            crate::native_agent_runtime::NativeAgentContextKind::Yielded,
            Some(self.first.result),
            1,
        )?;
        Some((
            FirstYieldRedispatchedFlow {
                first: self.first,
                second: self.second,
            },
            dispatched,
        ))
    }
}

impl FirstYieldRedispatchedFlow {
    pub(crate) fn record_first_completion(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedMailboxSenderCpu,
    ) -> Option<CompletedWorkerTasks> {
        transitions::record_final_completion(booted, self.first, self.second, cpu)
            .then_some(CompletedWorkerTasks::new(self.first, self.second))
    }
}
