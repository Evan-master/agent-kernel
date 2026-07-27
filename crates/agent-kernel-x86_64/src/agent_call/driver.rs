//! Strict register decoding for native Driver Agent Calls.
//!
//! This x86 ABI child converts bounded semantic Driver payloads into typed
//! requests. Resource, Capability, endpoint, and physical coordinates remain
//! outside the wire contract and are recovered from trusted kernel state.

use agent_kernel_core::{
    AgentId, AgentImageId, DeviceEventId, DriverCommandKind, DriverCommandPayload,
    DriverInvocationId,
};

use super::{
    ensure_extended_reserved_zero, ensure_reserved_zero, AgentCallDecodeError, AgentCallOperation,
    AgentCallRequest, AGENT_CALL_DRIVER_COMMAND_CONFIGURE, AGENT_CALL_DRIVER_COMMAND_WRITE,
};
use crate::context::PrivilegeInterruptStackFrame;

pub(super) fn decode(
    frame: &PrivilegeInterruptStackFrame,
    operation: AgentCallOperation,
) -> Result<AgentCallRequest, AgentCallDecodeError> {
    let (agent, invocation, image, nonce) = decode_context(frame)?;
    match operation {
        AgentCallOperation::InspectDriverInvocation => {
            ensure_reserved_zero(frame)?;
            Ok(AgentCallRequest::InspectDriverInvocation {
                agent,
                invocation,
                image,
                nonce,
            })
        }
        AgentCallOperation::AcknowledgeDeviceEvent => {
            if frame.r11 != 0 {
                return Err(AgentCallDecodeError::ReservedNotZero);
            }
            ensure_extended_reserved_zero(frame)?;
            if frame.r10 == 0 {
                return Err(AgentCallDecodeError::InvalidPayload);
            }
            Ok(AgentCallRequest::AcknowledgeDeviceEvent {
                agent,
                invocation,
                image,
                nonce,
                event: DeviceEventId::new(frame.r10),
            })
        }
        AgentCallOperation::SubmitDriverCommand => {
            if frame.r14 != 0 || frame.r15 != 0 || frame.rbp != 0 {
                return Err(AgentCallDecodeError::ReservedNotZero);
            }
            if frame.r10 == 0 {
                return Err(AgentCallDecodeError::InvalidPayload);
            }
            let kind = match frame.r11 {
                AGENT_CALL_DRIVER_COMMAND_WRITE => DriverCommandKind::Write,
                AGENT_CALL_DRIVER_COMMAND_CONFIGURE => DriverCommandKind::Configure,
                _ => return Err(AgentCallDecodeError::InvalidPayload),
            };
            let opcode =
                u16::try_from(frame.r12).map_err(|_| AgentCallDecodeError::InvalidPayload)?;
            Ok(AgentCallRequest::SubmitDriverCommand {
                agent,
                invocation,
                image,
                nonce,
                event: DeviceEventId::new(frame.r10),
                kind,
                payload: DriverCommandPayload {
                    opcode,
                    value: frame.r13,
                },
            })
        }
        AgentCallOperation::CompleteDriverInvocation => {
            ensure_reserved_zero(frame)?;
            Ok(AgentCallRequest::CompleteDriverInvocation {
                agent,
                invocation,
                image,
                nonce,
            })
        }
        _ => Err(AgentCallDecodeError::UnsupportedOperation),
    }
}

fn decode_context(
    frame: &PrivilegeInterruptStackFrame,
) -> Result<(AgentId, DriverInvocationId, AgentImageId, u64), AgentCallDecodeError> {
    if frame.rsi == 0 || frame.rdi == 0 || frame.r8 == 0 || frame.r9 == 0 {
        Err(AgentCallDecodeError::InvalidPayload)
    } else {
        Ok((
            AgentId::new(frame.rsi),
            DriverInvocationId::new(frame.rdi),
            AgentImageId::new(frame.r8),
            frame.r9,
        ))
    }
}
