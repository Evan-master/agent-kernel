use agent_kernel_core::{
    AgentId, DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandRequest, DriverCommandResult, DriverEndpointDescriptor, DriverEndpointRecord,
    ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::{
    pci_serial::{
        PciSerialBackend, PciSerialBackendError, PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT,
        PCI_SERIAL_RESULT_INVALID_REGISTER, PCI_SERIAL_RESULT_OK,
        PCI_SERIAL_RESULT_RESOURCE_MISMATCH, PCI_SERIAL_RESULT_TRANSMIT_TIMEOUT,
        PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND, PCI_SERIAL_RESULT_VALUE_OUT_OF_RANGE,
    },
    port::PortIo,
};

#[derive(Debug, Default)]
struct RecordingPortIo {
    read_values: Vec<u8>,
    read_index: usize,
    reads: Vec<u16>,
    writes: Vec<(u16, u8)>,
}

impl RecordingPortIo {
    fn with_reads(read_values: &[u8]) -> Self {
        Self {
            read_values: read_values.to_vec(),
            ..Self::default()
        }
    }
}

impl PortIo for RecordingPortIo {
    fn read_u8(&mut self, port: u16) -> u8 {
        self.reads.push(port);
        let value = self.read_values.get(self.read_index).copied().unwrap_or(0);
        self.read_index += 1;
        value
    }

    fn write_u8(&mut self, port: u16, value: u8) {
        self.writes.push((port, value));
    }
}

fn endpoint(resource: ResourceId, descriptor: DriverEndpointDescriptor) -> DriverEndpointRecord {
    DriverEndpointRecord {
        resource,
        installer: AgentId::new(1),
        descriptor,
    }
}

fn request(
    resource: ResourceId,
    kind: DriverCommandKind,
    opcode: u16,
    value: u64,
) -> DriverCommandRequest {
    DriverCommandRequest {
        command: DriverCommandId::new(1),
        binding: DriverBindingId::new(1),
        resource,
        driver: AgentId::new(2),
        cause: None,
        invocation: None,
        kind,
        payload: DriverCommandPayload { opcode, value },
    }
}

#[test]
fn constructor_accepts_only_a_complete_port_endpoint_and_nonzero_budget() {
    let resource = ResourceId::new(1);
    let backend = PciSerialBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0xd000, 8)),
        RecordingPortIo::default(),
        3,
    )
    .unwrap();
    assert_eq!(backend.resource(), resource);
    assert_eq!(backend.base(), 0xd000);
    assert_eq!(backend.transmit_poll_budget(), 3);

    let invalid = [
        (
            DriverEndpointDescriptor::mmio(0x8000_0000, 8),
            1,
            PciSerialBackendError::EndpointKindMismatch,
        ),
        (
            DriverEndpointDescriptor::port(0xd000, 7),
            1,
            PciSerialBackendError::EndpointDescriptorInvalid,
        ),
        (
            DriverEndpointDescriptor::port(0xffff, 8),
            1,
            PciSerialBackendError::EndpointDescriptorInvalid,
        ),
        (
            DriverEndpointDescriptor::port(0xd000, 8),
            0,
            PciSerialBackendError::PollBudgetInvalid,
        ),
    ];
    for (descriptor, budget, expected) in invalid {
        assert!(matches!(
            PciSerialBackend::new(
                endpoint(resource, descriptor),
                RecordingPortIo::default(),
                budget,
            ),
            Err(error) if error == expected
        ));
    }
}

#[test]
fn configure_arms_only_the_thre_interrupt_after_reading_iir() {
    let resource = ResourceId::new(1);
    let mut backend = PciSerialBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0xd000, 8)),
        RecordingPortIo::with_reads(&[0x01]),
        3,
    )
    .unwrap();

    assert_eq!(
        backend.execute(request(
            resource,
            DriverCommandKind::Configure,
            PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT,
            0,
        )),
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PCI_SERIAL_RESULT_OK,
            value: 0,
        })
    );
    assert_eq!(backend.io().reads, vec![0xd002]);
    assert_eq!(backend.io().writes, vec![(0xd001, 0x02)]);
}

#[test]
fn transmit_polls_until_thre_then_writes_exactly_one_byte() {
    let resource = ResourceId::new(1);
    let mut backend = PciSerialBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0xd000, 8)),
        RecordingPortIo::with_reads(&[0, 0, 0x20]),
        3,
    )
    .unwrap();

    assert_eq!(
        backend.execute(request(resource, DriverCommandKind::Write, 0, 0x50)),
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PCI_SERIAL_RESULT_OK,
            value: 0x50,
        })
    );
    assert_eq!(backend.io().reads, vec![0xd005, 0xd005, 0xd005]);
    assert_eq!(backend.io().writes, vec![(0xd000, 0x50)]);
}

#[test]
fn transmit_timeout_performs_only_the_bounded_status_reads() {
    let resource = ResourceId::new(1);
    let mut backend = PciSerialBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0xd000, 8)),
        RecordingPortIo::with_reads(&[0, 0, 0]),
        3,
    )
    .unwrap();

    assert_eq!(
        backend.execute(request(resource, DriverCommandKind::Write, 0, 0x50)),
        DriverCommandOutcome::Failed(DriverCommandResult {
            code: PCI_SERIAL_RESULT_TRANSMIT_TIMEOUT,
            value: 0,
        })
    );
    assert_eq!(backend.io().reads, vec![0xd005, 0xd005, 0xd005]);
    assert!(backend.io().writes.is_empty());
}

#[test]
fn rejected_requests_touch_no_port() {
    let resource = ResourceId::new(1);
    let mut backend = PciSerialBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0xd000, 8)),
        RecordingPortIo::with_reads(&[0x20]),
        1,
    )
    .unwrap();
    let rejected = [
        (
            request(ResourceId::new(2), DriverCommandKind::Write, 0, 0x50),
            PCI_SERIAL_RESULT_RESOURCE_MISMATCH,
        ),
        (
            request(resource, DriverCommandKind::Read, 0, 0),
            PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND,
        ),
        (
            request(
                resource,
                DriverCommandKind::Configure,
                PCI_SERIAL_COMMAND_ARM_THRE_INTERRUPT + 1,
                0,
            ),
            PCI_SERIAL_RESULT_UNSUPPORTED_COMMAND,
        ),
        (
            request(resource, DriverCommandKind::Write, 1, 0x50),
            PCI_SERIAL_RESULT_INVALID_REGISTER,
        ),
        (
            request(resource, DriverCommandKind::Write, 0, 0x100),
            PCI_SERIAL_RESULT_VALUE_OUT_OF_RANGE,
        ),
    ];

    for (request, code) in rejected {
        assert_eq!(
            backend.execute(request),
            DriverCommandOutcome::Failed(DriverCommandResult { code, value: 0 })
        );
    }
    assert!(backend.io().reads.is_empty());
    assert!(backend.io().writes.is_empty());
}
