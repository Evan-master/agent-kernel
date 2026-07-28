use std::cell::Cell;
use std::rc::Rc;

use agent_kernel_x86_64::virtio_rng::{
    VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo, VirtioRngDevice, VirtioRngDeviceError,
    VirtioRngQueueLayout, VirtioRngTransport, VirtioRngTransportError, VIRTIO_F_ACCESS_PLATFORM,
    VIRTIO_F_VERSION_1, VIRTIO_PCI_STATUS_ACKNOWLEDGE, VIRTIO_PCI_STATUS_DRIVER,
    VIRTIO_PCI_STATUS_DRIVER_OK, VIRTIO_PCI_STATUS_FAILED, VIRTIO_PCI_STATUS_FEATURES_OK,
    VIRTIO_RNG_USED_OFFSET,
};

const QUEUE_IOVA: u64 = 0x0100_1000;
const ENTROPY_IOVA: u64 = 0x0100_2000;

#[test]
fn initializes_one_modern_rng_queue_with_access_platform_and_msix() {
    let common = Common::ready();
    let notify = Notify::new(64, 4);
    let isr = Isr::new(1);
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut transport = VirtioRngTransport::bind(common, notify, isr, 8).unwrap();

    transport.initialize(layout, 0).unwrap();
    transport.notify_queue().unwrap();
    let interrupt = transport.acknowledge_interrupt().unwrap();
    assert!(interrupt.queue_used());
    assert!(!interrupt.configuration_changed());

    let (common, notify, isr) = transport.into_parts();
    assert_eq!(
        common.driver_features,
        VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM
    );
    assert_eq!(common.queue_select, 0);
    assert_eq!(common.queue_size, 1);
    assert_eq!(common.queue_msix_vector, 0);
    assert_eq!(common.queue_desc, QUEUE_IOVA);
    assert_eq!(common.queue_driver, QUEUE_IOVA + 0x100);
    assert_eq!(common.queue_device, QUEUE_IOVA + 0x200);
    assert_eq!(common.queue_enable, 1);
    assert_eq!(
        common.status,
        VIRTIO_PCI_STATUS_ACKNOWLEDGE
            | VIRTIO_PCI_STATUS_DRIVER
            | VIRTIO_PCI_STATUS_FEATURES_OK
            | VIRTIO_PCI_STATUS_DRIVER_OK
    );
    assert_eq!(notify.writes, vec![(12, 0)]);
    assert_eq!(isr.value.get(), 0);
}

#[test]
fn missing_access_platform_fails_before_queue_configuration() {
    let mut common = Common::ready();
    common.device_features = VIRTIO_F_VERSION_1;
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut transport =
        VirtioRngTransport::bind(common, Notify::new(64, 4), Isr::new(0), 8).unwrap();

    assert_eq!(
        transport.initialize(layout, 0),
        Err(VirtioRngTransportError::MissingRequiredFeatures {
            required: VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM,
            observed: VIRTIO_F_VERSION_1,
        })
    );
    let (common, notify, _) = transport.into_parts();
    assert_ne!(common.status & VIRTIO_PCI_STATUS_FAILED, 0);
    assert_eq!(common.queue_enable, 0);
    assert!(notify.writes.is_empty());
}

#[test]
fn device_feature_rejection_and_notify_bounds_fail_closed() {
    let mut rejecting = Common::ready();
    rejecting.reject_features = true;
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut transport =
        VirtioRngTransport::bind(rejecting, Notify::new(64, 4), Isr::new(0), 8).unwrap();
    assert_eq!(
        transport.initialize(layout, 0),
        Err(VirtioRngTransportError::FeaturesRejected)
    );

    let mut outside = Common::ready();
    outside.queue_notify_offset = 20;
    let mut transport =
        VirtioRngTransport::bind(outside, Notify::new(64, 4), Isr::new(0), 8).unwrap();
    assert_eq!(
        transport.initialize(layout, 0),
        Err(VirtioRngTransportError::NotifyOutsideRegion {
            offset: 80,
            region_bytes: 64,
        })
    );
}

#[test]
fn notify_address_uses_the_capability_multiplier() {
    let common = Common::ready();
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut transport =
        VirtioRngTransport::bind(common, Notify::new(64, 8), Isr::new(0), 8).unwrap();

    transport.initialize(layout, 0).unwrap();
    transport.notify_queue().unwrap();

    let (_, notify, _) = transport.into_parts();
    assert_eq!(notify.writes, vec![(24, 0)]);
}

#[test]
fn shutdown_resets_the_device_before_transport_authority_is_released() {
    let common = Common::ready();
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut transport =
        VirtioRngTransport::bind(common, Notify::new(64, 4), Isr::new(0), 8).unwrap();
    transport.initialize(layout, 0).unwrap();

    transport.shutdown().unwrap();

    assert_eq!(
        transport.notify_queue(),
        Err(VirtioRngTransportError::NotInitialized)
    );
    let (common, _, _) = transport.into_parts();
    assert_eq!(common.status, 0);
    assert_eq!(common.queue_enable, 1);
}

#[test]
fn device_owner_runs_one_entropy_request_from_notify_to_used_ring() {
    let common = Common::ready();
    let notify = Notify::new(64, 4);
    let isr = Isr::new(0);
    let isr_value = Rc::clone(&isr.value);
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut metadata = [0u8; 4096];
    let mut entropy = [0u8; 4096];
    let metadata_pointer = metadata.as_mut_ptr();
    let entropy_pointer = entropy.as_mut_ptr();
    let mut device =
        VirtioRngDevice::bind(common, notify, isr, 8, &mut metadata, &mut entropy, layout).unwrap();
    device.initialize(0).unwrap();

    assert_eq!(
        device.notify_entropy_request(),
        Err(VirtioRngDeviceError::NoRequestPending)
    );
    device.prepare_entropy_request(8).unwrap();
    assert_eq!(
        device.complete_interrupt(),
        Err(VirtioRngDeviceError::RequestNotNotified)
    );
    assert_eq!(
        device.prepare_entropy_request(8),
        Err(VirtioRngDeviceError::RequestPending)
    );
    device.notify_entropy_request().unwrap();
    assert_eq!(
        device.notify_entropy_request(),
        Err(VirtioRngDeviceError::RequestAlreadyNotified)
    );
    unsafe {
        for index in 0..8 {
            entropy_pointer.add(index).write(0x80 + index as u8);
        }
        write_device_u32(metadata_pointer, VIRTIO_RNG_USED_OFFSET + 4, 0);
        write_device_u32(metadata_pointer, VIRTIO_RNG_USED_OFFSET + 8, 8);
        write_device_u16(metadata_pointer, VIRTIO_RNG_USED_OFFSET + 2, 1);
        isr_value.set(1);
    }
    let completion = device.complete_interrupt().unwrap();

    assert_eq!(
        device.entropy(&completion),
        &[0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87]
    );
    device.shutdown().unwrap();
}

#[derive(Debug)]
struct Common {
    device_features: u64,
    device_feature_select: u32,
    driver_feature_select: u32,
    driver_features: u64,
    status: u8,
    num_queues: u16,
    queue_select: u16,
    queue_size: u16,
    queue_msix_vector: u16,
    queue_enable: u16,
    queue_notify_offset: u16,
    queue_desc: u64,
    queue_driver: u64,
    queue_device: u64,
    reject_features: bool,
}

impl Common {
    fn ready() -> Self {
        Self {
            device_features: VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM,
            device_feature_select: 0,
            driver_feature_select: 0,
            driver_features: 0,
            status: 0,
            num_queues: 1,
            queue_select: 0,
            queue_size: 8,
            queue_msix_vector: u16::MAX,
            queue_enable: 0,
            queue_notify_offset: 3,
            queue_desc: 0,
            queue_driver: 0,
            queue_device: 0,
            reject_features: false,
        }
    }
}

impl VirtioCommonConfigIo for Common {
    fn read_u8(&mut self, offset: u16) -> u8 {
        match offset {
            0x14 => self.status,
            _ => 0,
        }
    }

    fn read_u16(&mut self, offset: u16) -> u16 {
        match offset {
            0x12 => self.num_queues,
            0x18 => self.queue_size,
            0x1a => self.queue_msix_vector,
            0x1c => self.queue_enable,
            0x1e => self.queue_notify_offset,
            _ => 0,
        }
    }

    fn read_u32(&mut self, offset: u16) -> u32 {
        match offset {
            0x04 if self.device_feature_select == 0 => self.device_features as u32,
            0x04 => (self.device_features >> 32) as u32,
            _ => 0,
        }
    }

    fn write_u8(&mut self, offset: u16, value: u8) {
        if offset == 0x14 {
            self.status = if self.reject_features && value & VIRTIO_PCI_STATUS_FEATURES_OK != 0 {
                value & !VIRTIO_PCI_STATUS_FEATURES_OK
            } else {
                value
            };
        }
    }

    fn write_u16(&mut self, offset: u16, value: u16) {
        match offset {
            0x16 => self.queue_select = value,
            0x18 => self.queue_size = value,
            0x1a => self.queue_msix_vector = value,
            0x1c => self.queue_enable = value,
            _ => {}
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        match offset {
            0x00 => self.device_feature_select = value,
            0x08 => self.driver_feature_select = value,
            0x0c if self.driver_feature_select == 0 => {
                self.driver_features =
                    (self.driver_features & !u64::from(u32::MAX)) | u64::from(value)
            }
            0x0c => {
                self.driver_features =
                    (self.driver_features & u64::from(u32::MAX)) | (u64::from(value) << 32)
            }
            _ => {}
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        match offset {
            0x20 => self.queue_desc = value,
            0x28 => self.queue_driver = value,
            0x30 => self.queue_device = value,
            _ => {}
        }
    }
}

#[derive(Debug)]
struct Notify {
    region_bytes: u32,
    offset_multiplier: u32,
    writes: Vec<(u32, u16)>,
}

impl Notify {
    fn new(region_bytes: u32, offset_multiplier: u32) -> Self {
        Self {
            region_bytes,
            offset_multiplier,
            writes: Vec::new(),
        }
    }
}

impl VirtioNotifyIo for Notify {
    fn region_bytes(&self) -> u32 {
        self.region_bytes
    }

    fn offset_multiplier(&self) -> u32 {
        self.offset_multiplier
    }

    fn write_u16(&mut self, byte_offset: u32, value: u16) {
        self.writes.push((byte_offset, value));
    }
}

#[derive(Debug)]
struct Isr {
    value: Rc<Cell<u8>>,
}

impl Isr {
    fn new(value: u8) -> Self {
        Self {
            value: Rc::new(Cell::new(value)),
        }
    }
}

impl VirtioIsrIo for Isr {
    fn read_and_acknowledge(&mut self) -> u8 {
        let value = self.value.get();
        self.value.set(0);
        value
    }
}

unsafe fn write_device_u16(base: *mut u8, offset: u16, value: u16) {
    let bytes = value.to_le_bytes();
    unsafe {
        base.add(usize::from(offset)).write(bytes[0]);
        base.add(usize::from(offset) + 1).write(bytes[1]);
    }
}

unsafe fn write_device_u32(base: *mut u8, offset: u16, value: u32) {
    let bytes = value.to_le_bytes();
    unsafe {
        for (index, byte) in bytes.iter().copied().enumerate() {
            base.add(usize::from(offset) + index).write(byte);
        }
    }
}
