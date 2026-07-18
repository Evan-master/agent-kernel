//! Scheduler-owned identity authentication across Agent Call operations.

use super::AgentCallContext;
use crate::agent_call::AgentCallRequest;

impl AgentCallContext {
    pub fn authenticates(self, request: AgentCallRequest, expected_nonce: u64) -> bool {
        match request {
            AgentCallRequest::DescribeContext { .. } => false,
            AgentCallRequest::Yield {
                agent,
                task,
                image,
                nonce,
            }
            | AgentCallRequest::CompleteTask {
                agent,
                task,
                image,
                nonce,
            }
            | AgentCallRequest::SubmitTaskResult {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::InspectTaskResult {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::VerifyTask {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::SendMessage {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::ReceiveMessage {
                agent,
                task,
                image,
                nonce,
            }
            | AgentCallRequest::AcknowledgeMessage {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::CreateResource {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::RetireResource {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::DeriveCapability {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::RevokeDerivedCapability {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::DeclareIntent {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::CreateTask {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::DelegateTask {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::RegisterManagedAgent {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::SuspendManagedAgent {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::ResumeManagedAgent {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::RetireManagedAgent {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::AllocateMemoryPage {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::InspectMemoryPage {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::ReleaseMemoryPage {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::AllocateMemoryRegion {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::InspectMemoryRegion {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::ReleaseMemoryRegion {
                agent,
                task,
                image,
                nonce,
                ..
            }
            | AgentCallRequest::RequestRuntimeAdmission {
                agent,
                task,
                image,
                nonce,
                ..
            } => self.matches_identity(agent, task, image, nonce, expected_nonce),
        }
    }
}
