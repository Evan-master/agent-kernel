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
        RequestedSenderResultCpu,
    },
    X86BootedKernel,
};

use super::{
    message_transition, result_transition, transitions, CompletedWorkerTasks, FirstMessageSentFlow,
    FirstResultSubmittedFlow, FirstResumedFlow, SecondMessageAcknowledgedFlow,
    SecondMessageReceivedFlow, SecondResultSubmittedFlow, SecondResumedFlow,
};

impl FirstResumedFlow {
    pub(crate) fn submit_first_result(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedSenderResultCpu,
    ) -> Option<(FirstResultSubmittedFlow, AcknowledgedSenderResultCpu)> {
        let acknowledged =
            result_transition::submit(booted, self.first, Some(self.second), None, cpu)?;
        Some((
            FirstResultSubmittedFlow {
                first: self.first,
                second: self.second,
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
        let acknowledged = message_transition::send(booted, self.first, self.second, cpu)?;
        Some((
            FirstMessageSentFlow {
                first: self.first,
                second: self.second,
            },
            acknowledged,
        ))
    }
}

impl FirstMessageSentFlow {
    pub(crate) fn complete_first_and_dispatch_second(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedMailboxSenderCpu,
    ) -> Option<SecondResumedFlow> {
        transitions::complete_and_dispatch(booted, self.first, self.second, cpu, 1)?;
        Some(SecondResumedFlow {
            first: self.first,
            second: self.second,
        })
    }
}

impl SecondResumedFlow {
    pub(crate) fn receive_from_first(
        self,
        booted: &mut X86BootedKernel,
        cpu: RequestedMessageReceiveCpu,
    ) -> Option<(SecondMessageReceivedFlow, AcknowledgedMessageReceiveCpu)> {
        let acknowledged = message_transition::receive(booted, self.second, self.first, cpu)?;
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
            result_transition::submit(booted, self.second, None, Some(self.first), cpu)?;
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
    pub(crate) fn record_second_completion(
        self,
        booted: &mut X86BootedKernel,
        cpu: CompletedMailboxReceiverCpu,
    ) -> Option<CompletedWorkerTasks> {
        transitions::record_final_completion(booted, self.second, self.first, cpu)
            .then_some(CompletedWorkerTasks::new(self.first, self.second))
    }
}
