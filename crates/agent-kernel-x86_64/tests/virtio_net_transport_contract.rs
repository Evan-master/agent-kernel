use agent_kernel_x86_64::{
    virtio_net::{
        VirtioNetQueueLayout, VirtioNetTransport, VirtioNetTransportError,
        VIRTIO_F_ACCESS_PLATFORM, VIRTIO_F_VERSION_1, VIRTIO_NET_F_MAC, VIRTIO_NET_F_MRG_RXBUF,
    },
    virtio_rng::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo},
};

const REQUIRED_FEATURES: u64 =
    VIRTIO_NET_F_MAC | VIRTIO_NET_F_MRG_RXBUF | VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM;

#[test]
fn initializes_receive_and_transmit_queues_with_exact_features_and_vectors() {
    let common = Common::new(REQUIRED_FEATURES, 2);
    let notify = Notify::new(64, 4);
    let isr = Isr;
    let rx = VirtioNetQueueLayout::new(0x0200_0000, 0x0200_1000).unwrap();
    let tx = VirtioNetQueueLayout::new(0x0200_2000, 0x0200_3000).unwrap();
    let mut transport = VirtioNetTransport::bind(common, notify, isr, 8).unwrap();

    transport.initialize(rx, tx, 0, 1).unwrap();
    transport.notify_receive().unwrap();
    transport.notify_transmit().unwrap();
    let (common, notify, _) = transport.into_parts();

    assert_eq!(common.driver_features, REQUIRED_FEATURES);
    assert_eq!(common.queue_size, [1, 1]);
    assert_eq!(common.queue_vector, [0, 1]);
    assert_eq!(common.queue_enable, [1, 1]);
    assert_eq!(
        common.queue_desc,
        [rx.descriptor_iova(), tx.descriptor_iova()]
    );
    assert_eq!(notify.writes, [(12, 0), (28, 1)]);
}

#[test]
fn rejects_missing_mac_feature_or_second_queue_and_marks_failed() {
    let rx = VirtioNetQueueLayout::new(0x0200_0000, 0x0200_1000).unwrap();
    let tx = VirtioNetQueueLayout::new(0x0200_2000, 0x0200_3000).unwrap();
    let common = Common::new(VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM, 2);
    let mut missing = VirtioNetTransport::bind(common, Notify::new(64, 4), Isr, 8).unwrap();
    assert_eq!(
        missing.initialize(rx, tx, 0, 1),
        Err(VirtioNetTransportError::MissingRequiredFeatures {
            required: REQUIRED_FEATURES,
            observed: VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM,
        })
    );
    assert_ne!(missing.into_parts().0.status & 0x80, 0);

    let common = Common::new(REQUIRED_FEATURES, 1);
    let mut one_queue = VirtioNetTransport::bind(common, Notify::new(64, 4), Isr, 8).unwrap();
    assert_eq!(
        one_queue.initialize(rx, tx, 0, 1),
        Err(VirtioNetTransportError::QueueUnavailable(1))
    );
}

struct Common {
    device_features: u64,
    driver_features: u64,
    feature_select: u32,
    driver_select: u32,
    status: u8,
    num_queues: u16,
    selected: usize,
    queue_size: [u16; 2],
    queue_vector: [u16; 2],
    queue_enable: [u16; 2],
    queue_notify: [u16; 2],
    queue_desc: [u64; 2],
    queue_driver: [u64; 2],
    queue_device: [u64; 2],
}

impl Common {
    fn new(device_features: u64, num_queues: u16) -> Self {
        Self {
            device_features,
            driver_features: 0,
            feature_select: 0,
            driver_select: 0,
            status: 0,
            num_queues,
            selected: 0,
            queue_size: [8, 8],
            queue_vector: [u16::MAX; 2],
            queue_enable: [0; 2],
            queue_notify: [3, 7],
            queue_desc: [0; 2],
            queue_driver: [0; 2],
            queue_device: [0; 2],
        }
    }
}

impl VirtioCommonConfigIo for Common {
    fn read_u8(&mut self, offset: u16) -> u8 {
        if offset == 0x14 {
            self.status
        } else {
            0
        }
    }

    fn read_u16(&mut self, offset: u16) -> u16 {
        match offset {
            0x12 => self.num_queues,
            0x18 => self.queue_size[self.selected],
            0x1a => self.queue_vector[self.selected],
            0x1c => self.queue_enable[self.selected],
            0x1e => self.queue_notify[self.selected],
            _ => 0,
        }
    }

    fn read_u32(&mut self, offset: u16) -> u32 {
        match (offset, self.feature_select) {
            (0x04, 0) => self.device_features as u32,
            (0x04, _) => (self.device_features >> 32) as u32,
            _ => 0,
        }
    }

    fn write_u8(&mut self, offset: u16, value: u8) {
        if offset == 0x14 {
            self.status = value;
        }
    }

    fn write_u16(&mut self, offset: u16, value: u16) {
        match offset {
            0x16 => self.selected = usize::from(value.min(1)),
            0x18 => self.queue_size[self.selected] = value,
            0x1a => self.queue_vector[self.selected] = value,
            0x1c => self.queue_enable[self.selected] = value,
            _ => {}
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        match offset {
            0x00 => self.feature_select = value,
            0x08 => self.driver_select = value,
            0x0c if self.driver_select == 0 => {
                self.driver_features =
                    (self.driver_features & !u64::from(u32::MAX)) | u64::from(value);
            }
            0x0c => {
                self.driver_features =
                    (self.driver_features & u64::from(u32::MAX)) | (u64::from(value) << 32);
            }
            _ => {}
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        match offset {
            0x20 => self.queue_desc[self.selected] = value,
            0x28 => self.queue_driver[self.selected] = value,
            0x30 => self.queue_device[self.selected] = value,
            _ => {}
        }
    }
}

struct Notify {
    region_bytes: u32,
    multiplier: u32,
    writes: Vec<(u32, u16)>,
}

impl Notify {
    fn new(region_bytes: u32, multiplier: u32) -> Self {
        Self {
            region_bytes,
            multiplier,
            writes: Vec::new(),
        }
    }
}

impl VirtioNotifyIo for Notify {
    fn region_bytes(&self) -> u32 {
        self.region_bytes
    }

    fn offset_multiplier(&self) -> u32 {
        self.multiplier
    }

    fn write_u16(&mut self, byte_offset: u32, value: u16) {
        self.writes.push((byte_offset, value));
    }
}

struct Isr;

impl VirtioIsrIo for Isr {
    fn read_and_acknowledge(&mut self) -> u8 {
        1
    }
}
