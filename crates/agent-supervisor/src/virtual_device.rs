//! Stateful host-side backend used by the supervisor demonstration.
//!
//! This supervisor-layer adapter implements the no_std HAL contract with a
//! private virtual register. It never mutates kernel state, and rejects requests
//! for a different resource without applying a device-side effect.

use agent_kernel_core::{
    DriverCommandKind, DriverCommandRequest, DriverCommandResult, DriverEndpointKind,
    DriverEndpointRecord, ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};

const RESULT_OK: u16 = 0;
const RESULT_RESOURCE_MISMATCH: u16 = 1;

pub struct VirtualRegisterDevice {
    resource: ResourceId,
    channel: u64,
    value: u64,
    executions: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VirtualDeviceError {
    EndpointKindMismatch,
}

impl VirtualRegisterDevice {
    pub fn new(endpoint: DriverEndpointRecord) -> Result<Self, VirtualDeviceError> {
        if endpoint.descriptor.kind != DriverEndpointKind::Virtual {
            return Err(VirtualDeviceError::EndpointKindMismatch);
        }
        Ok(Self {
            resource: endpoint.resource,
            channel: endpoint.descriptor.base,
            value: 0,
            executions: 0,
        })
    }

    pub const fn channel(&self) -> u64 {
        self.channel
    }

    pub const fn value(&self) -> u64 {
        self.value
    }

    pub const fn executions(&self) -> u64 {
        self.executions
    }
}

impl DriverBackend for VirtualRegisterDevice {
    fn execute(&mut self, request: DriverCommandRequest) -> DriverCommandOutcome {
        if request.resource != self.resource {
            return DriverCommandOutcome::Failed(DriverCommandResult {
                code: RESULT_RESOURCE_MISMATCH,
                value: self.value,
            });
        }

        self.executions += 1;
        let value = match request.kind {
            DriverCommandKind::Configure | DriverCommandKind::Write => {
                self.value = request.payload.value;
                self.value
            }
            DriverCommandKind::Read => self.value,
            DriverCommandKind::Reset => {
                self.value = 0;
                self.value
            }
        };
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: RESULT_OK,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_kernel_core::{
        AgentId, DriverBindingId, DriverCommandId, DriverCommandPayload, DriverEndpointDescriptor,
    };

    fn endpoint(
        resource: ResourceId,
        descriptor: DriverEndpointDescriptor,
    ) -> DriverEndpointRecord {
        DriverEndpointRecord {
            resource,
            installer: AgentId::new(1),
            descriptor,
        }
    }

    fn request(resource: ResourceId, kind: DriverCommandKind, value: u64) -> DriverCommandRequest {
        DriverCommandRequest {
            command: DriverCommandId::new(1),
            binding: DriverBindingId::new(1),
            resource,
            driver: AgentId::new(1),
            cause: None,
            invocation: None,
            kind,
            payload: DriverCommandPayload { opcode: 3, value },
        }
    }

    #[test]
    fn register_operations_mutate_private_device_state() {
        let resource = ResourceId::new(1);
        let mut device = VirtualRegisterDevice::new(endpoint(
            resource,
            DriverEndpointDescriptor::virtual_channel(7),
        ))
        .unwrap();

        assert_eq!(
            device.execute(request(resource, DriverCommandKind::Write, 11)),
            DriverCommandOutcome::Completed(DriverCommandResult { code: 0, value: 11 })
        );
        assert_eq!(
            device.execute(request(resource, DriverCommandKind::Read, 99)),
            DriverCommandOutcome::Completed(DriverCommandResult { code: 0, value: 11 })
        );
        assert_eq!(
            device.execute(request(resource, DriverCommandKind::Reset, 99)),
            DriverCommandOutcome::Completed(DriverCommandResult { code: 0, value: 0 })
        );
        assert_eq!(
            device.execute(request(resource, DriverCommandKind::Configure, 7)),
            DriverCommandOutcome::Completed(DriverCommandResult { code: 0, value: 7 })
        );
        assert_eq!(device.value(), 7);
        assert_eq!(device.executions(), 4);
        assert_eq!(device.channel(), 7);
    }

    #[test]
    fn resource_mismatch_fails_without_side_effect() {
        let mut device = VirtualRegisterDevice::new(endpoint(
            ResourceId::new(1),
            DriverEndpointDescriptor::virtual_channel(7),
        ))
        .unwrap();

        let outcome = device.execute(request(ResourceId::new(2), DriverCommandKind::Write, 11));

        assert_eq!(
            outcome,
            DriverCommandOutcome::Failed(DriverCommandResult { code: 1, value: 0 })
        );
        assert_eq!(device.value(), 0);
        assert_eq!(device.executions(), 0);
    }

    #[test]
    fn physical_endpoint_cannot_initialize_virtual_backend() {
        let result = VirtualRegisterDevice::new(endpoint(
            ResourceId::new(1),
            DriverEndpointDescriptor::mmio(0x1000, 0x100),
        ));

        assert!(matches!(
            result,
            Err(VirtualDeviceError::EndpointKindMismatch)
        ));
    }
}
