use std::{cell::RefCell, rc::Rc};

use agent_kernel_x86_64::{
    virtio_net::{
        VirtioNetDevice, VirtioNetDeviceConfigIo, VirtioNetDeviceError, VirtioNetQueueLayout,
        VIRTIO_F_ACCESS_PLATFORM, VIRTIO_F_VERSION_1, VIRTIO_NET_F_MAC, VIRTIO_NET_F_MRG_RXBUF,
    },
    virtio_rng::{VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo},
};

const FEATURES: u64 =
    VIRTIO_NET_F_MAC | VIRTIO_NET_F_MRG_RXBUF | VIRTIO_F_VERSION_1 | VIRTIO_F_ACCESS_PLATFORM;

#[test]
fn ordered_device_owner_initializes_mac_and_publishes_both_queues() {
    let writes = Rc::new(RefCell::new(Vec::new()));
    let notify = Notify {
        writes: writes.clone(),
    };
    let mut rx_metadata = [0; 4096];
    let mut rx_packet = [0; 4096];
    let mut tx_metadata = [0; 4096];
    let mut tx_packet = [0; 4096];
    let mut device = VirtioNetDevice::bind(
        Common::new(),
        notify,
        Isr,
        DeviceConfig([0x52, 0x54, 0, 0x12, 0x34, 0x56]),
        8,
        &mut rx_metadata,
        &mut rx_packet,
        VirtioNetQueueLayout::new(0x0200_0000, 0x0200_1000).unwrap(),
        &mut tx_metadata,
        &mut tx_packet,
        VirtioNetQueueLayout::new(0x0200_2000, 0x0200_3000).unwrap(),
    )
    .unwrap();

    let mac = device.initialize(0, 1).unwrap();
    assert_eq!(mac.bytes(), [0x52, 0x54, 0, 0x12, 0x34, 0x56]);
    device.prepare_receive().unwrap();
    device.notify_receive().unwrap();
    let mut frame = [0; 60];
    frame[12..14].copy_from_slice(&0x0806_u16.to_be_bytes());
    device.prepare_transmit(&frame).unwrap();
    device.notify_transmit().unwrap();
    assert_eq!(&*writes.borrow(), &[(12, 0), (28, 1)]);
}

#[test]
fn device_owner_rejects_noncanonical_device_mac() {
    let mut pages = [[0_u8; 4096]; 4];
    let (rx_metadata, rest) = pages.split_at_mut(1);
    let (rx_packet, rest) = rest.split_at_mut(1);
    let (tx_metadata, tx_packet) = rest.split_at_mut(1);
    let mut device = VirtioNetDevice::bind(
        Common::new(),
        Notify {
            writes: Rc::new(RefCell::new(Vec::new())),
        },
        Isr,
        DeviceConfig([0xff; 6]),
        8,
        &mut rx_metadata[0],
        &mut rx_packet[0],
        VirtioNetQueueLayout::new(0x0200_0000, 0x0200_1000).unwrap(),
        &mut tx_metadata[0],
        &mut tx_packet[0],
        VirtioNetQueueLayout::new(0x0200_2000, 0x0200_3000).unwrap(),
    )
    .unwrap();
    assert_eq!(
        device.initialize(0, 1),
        Err(VirtioNetDeviceError::InvalidMac)
    );
}

#[test]
fn device_owner_rejects_cross_queue_iova_aliases() {
    let mut pages = [[0_u8; 4096]; 4];
    let (rx_metadata, rest) = pages.split_at_mut(1);
    let (rx_packet, rest) = rest.split_at_mut(1);
    let (tx_metadata, tx_packet) = rest.split_at_mut(1);
    let result = VirtioNetDevice::bind(
        Common::new(),
        Notify {
            writes: Rc::new(RefCell::new(Vec::new())),
        },
        Isr,
        DeviceConfig([0x52, 0x54, 0, 0x12, 0x34, 0x56]),
        8,
        &mut rx_metadata[0],
        &mut rx_packet[0],
        VirtioNetQueueLayout::new(0x0200_0000, 0x0200_1000).unwrap(),
        &mut tx_metadata[0],
        &mut tx_packet[0],
        VirtioNetQueueLayout::new(0x0200_1000, 0x0200_3000).unwrap(),
    );

    assert!(matches!(
        result,
        Err(VirtioNetDeviceError::OverlappingQueuePages)
    ));
}

struct DeviceConfig([u8; 6]);

impl VirtioNetDeviceConfigIo for DeviceConfig {
    fn read_mac(&mut self) -> [u8; 6] {
        self.0
    }
}

struct Common {
    feature_select: u32,
    driver_select: u32,
    status: u8,
    selected: usize,
    size: [u16; 2],
    vector: [u16; 2],
    enabled: [u16; 2],
}

impl Common {
    fn new() -> Self {
        Self {
            feature_select: 0,
            driver_select: 0,
            status: 0,
            selected: 0,
            size: [8; 2],
            vector: [u16::MAX; 2],
            enabled: [0; 2],
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
            0x12 => 2,
            0x18 => self.size[self.selected],
            0x1a => self.vector[self.selected],
            0x1c => self.enabled[self.selected],
            0x1e => [3, 7][self.selected],
            _ => 0,
        }
    }

    fn read_u32(&mut self, offset: u16) -> u32 {
        if offset != 0x04 {
            0
        } else if self.feature_select == 0 {
            FEATURES as u32
        } else {
            (FEATURES >> 32) as u32
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
            0x18 => self.size[self.selected] = value,
            0x1a => self.vector[self.selected] = value,
            0x1c => self.enabled[self.selected] = value,
            _ => {}
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        match offset {
            0x00 => self.feature_select = value,
            0x08 => self.driver_select = value,
            _ => {}
        }
    }

    fn write_u64(&mut self, _offset: u16, _value: u64) {}
}

struct Notify {
    writes: Rc<RefCell<Vec<(u32, u16)>>>,
}

impl VirtioNotifyIo for Notify {
    fn region_bytes(&self) -> u32 {
        64
    }

    fn offset_multiplier(&self) -> u32 {
        4
    }

    fn write_u16(&mut self, byte_offset: u32, value: u16) {
        self.writes.borrow_mut().push((byte_offset, value));
    }
}

struct Isr;

impl VirtioIsrIo for Isr {
    fn read_and_acknowledge(&mut self) -> u8 {
        1
    }
}
