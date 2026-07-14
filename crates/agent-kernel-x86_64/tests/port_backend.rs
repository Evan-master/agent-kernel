use agent_kernel_core::{
    AgentId, DriverBindingId, DriverCommandId, DriverCommandKind, DriverCommandPayload,
    DriverCommandRequest, DriverCommandResult, DriverEndpointDescriptor, DriverEndpointKind,
    DriverEndpointRecord, ResourceId,
};
use agent_kernel_hal::{DriverBackend, DriverCommandOutcome};
use agent_kernel_x86_64::port::{
    PortIo, PortIoBackend, PortIoBackendError, PORT_IO_RESULT_OFFSET_OUT_OF_RANGE,
    PORT_IO_RESULT_OK, PORT_IO_RESULT_RESOURCE_MISMATCH, PORT_IO_RESULT_UNSUPPORTED_COMMAND,
    PORT_IO_RESULT_VALUE_OUT_OF_RANGE,
};

#[derive(Debug, Default)]
struct RecordingPortIo {
    read_value: u8,
    reads: Vec<u16>,
    writes: Vec<(u16, u8)>,
}

impl PortIo for RecordingPortIo {
    fn read_u8(&mut self, port: u16) -> u8 {
        self.reads.push(port);
        self.read_value
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
    offset: u16,
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
        payload: DriverCommandPayload {
            opcode: offset,
            value,
        },
    }
}

#[test]
fn constructor_accepts_only_valid_port_endpoints() {
    let resource = ResourceId::new(1);
    let backend = PortIoBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0x3f8, 8)),
        RecordingPortIo::default(),
    )
    .expect("validated port endpoint should construct backend");
    assert_eq!(backend.resource(), resource);
    assert_eq!(backend.base(), 0x3f8);
    assert_eq!(backend.span(), 8);

    let invalid = [
        (
            DriverEndpointDescriptor::virtual_channel(1),
            PortIoBackendError::EndpointKindMismatch,
        ),
        (
            DriverEndpointDescriptor {
                kind: DriverEndpointKind::Port,
                base: 0x3f8,
                span: 0,
            },
            PortIoBackendError::EndpointDescriptorInvalid,
        ),
        (
            DriverEndpointDescriptor::port(u64::MAX, 2),
            PortIoBackendError::EndpointDescriptorInvalid,
        ),
        (
            DriverEndpointDescriptor::port(u16::MAX as u64, 2),
            PortIoBackendError::EndpointDescriptorInvalid,
        ),
    ];

    for (descriptor, expected) in invalid {
        assert!(matches!(
            PortIoBackend::new(endpoint(resource, descriptor), RecordingPortIo::default()),
            Err(error) if error == expected
        ));
    }
}

#[test]
fn read_resolves_relative_offset_and_returns_byte() {
    let resource = ResourceId::new(1);
    let mut backend = PortIoBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0x3f8, 8)),
        RecordingPortIo {
            read_value: 0xa5,
            ..RecordingPortIo::default()
        },
    )
    .unwrap();

    let outcome = backend.execute(request(resource, DriverCommandKind::Read, 5, 0));

    assert_eq!(
        outcome,
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PORT_IO_RESULT_OK,
            value: 0xa5,
        })
    );
    assert_eq!(backend.io().reads, vec![0x3fd]);
    assert!(backend.io().writes.is_empty());
}

#[test]
fn write_resolves_relative_offset_and_emits_one_byte() {
    let resource = ResourceId::new(1);
    let mut backend = PortIoBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0x3f8, 8)),
        RecordingPortIo::default(),
    )
    .unwrap();

    let outcome = backend.execute(request(resource, DriverCommandKind::Write, 3, 0x7f));

    assert_eq!(
        outcome,
        DriverCommandOutcome::Completed(DriverCommandResult {
            code: PORT_IO_RESULT_OK,
            value: 0x7f,
        })
    );
    assert_eq!(backend.io().writes, vec![(0x3fb, 0x7f)]);
    assert!(backend.io().reads.is_empty());
}

#[test]
fn rejected_requests_never_touch_port_io() {
    let resource = ResourceId::new(1);
    let mut backend = PortIoBackend::new(
        endpoint(resource, DriverEndpointDescriptor::port(0x3f8, 8)),
        RecordingPortIo::default(),
    )
    .unwrap();

    let rejected = [
        (
            request(ResourceId::new(2), DriverCommandKind::Write, 0, 1),
            PORT_IO_RESULT_RESOURCE_MISMATCH,
        ),
        (
            request(resource, DriverCommandKind::Read, 8, 0),
            PORT_IO_RESULT_OFFSET_OUT_OF_RANGE,
        ),
        (
            request(resource, DriverCommandKind::Write, 0, 0x100),
            PORT_IO_RESULT_VALUE_OUT_OF_RANGE,
        ),
        (
            request(resource, DriverCommandKind::Configure, 0, 1),
            PORT_IO_RESULT_UNSUPPORTED_COMMAND,
        ),
        (
            request(resource, DriverCommandKind::Reset, 0, 0),
            PORT_IO_RESULT_UNSUPPORTED_COMMAND,
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
