mod ata_support;

use agent_kernel_x86_64::ata::{
    AtaDrive, AtaPioConfig, AtaPioConfigError, AtaPioDevice, AtaPioError, ATA_COMMAND_FLUSH_EXT,
    ATA_COMMAND_IDENTIFY, ATA_COMMAND_READ_EXT, ATA_COMMAND_WRITE_EXT, ATA_SECTOR_BYTES,
};

use ata_support::{identify_words, RegisterDouble, COMMAND_BASE, CONTROL_BASE};

const DEVICE_SECTORS: u64 = 0x0000_0200_0000_0000;

fn identified_device() -> AtaPioDevice<RegisterDouble> {
    let config =
        AtaPioConfig::new(COMMAND_BASE, CONTROL_BASE, AtaDrive::Master, 8).expect("valid config");
    let mut io = RegisterDouble::ready();
    io.queue_data(identify_words(DEVICE_SECTORS));
    let mut device = AtaPioDevice::new(config, io);
    assert_eq!(
        device.identify().expect("identify"),
        agent_kernel_x86_64::ata::AtaDeviceIdentity::new(DEVICE_SECTORS).expect("valid identity")
    );
    device.io_mut().clear_operations();
    device
}

#[test]
fn config_rejects_zero_poll_budget_and_wrapped_command_span() {
    assert_eq!(
        AtaPioConfig::new(COMMAND_BASE, CONTROL_BASE, AtaDrive::Master, 0),
        Err(AtaPioConfigError::ZeroPollBudget)
    );
    assert_eq!(
        AtaPioConfig::new(0, CONTROL_BASE, AtaDrive::Master, 1),
        Err(AtaPioConfigError::ZeroCommandBase)
    );
    assert_eq!(
        AtaPioConfig::new(COMMAND_BASE, 0, AtaDrive::Master, 1),
        Err(AtaPioConfigError::ZeroControlBase)
    );
    assert_eq!(
        AtaPioConfig::new(u16::MAX - 6, CONTROL_BASE, AtaDrive::Master, 1),
        Err(AtaPioConfigError::CommandSpanOverflow)
    );
    assert_eq!(
        AtaPioConfig::new(COMMAND_BASE, COMMAND_BASE + 3, AtaDrive::Master, 1),
        Err(AtaPioConfigError::ControlOverlapsCommandSpan)
    );
    assert!(agent_kernel_x86_64::ata::AtaDeviceIdentity::new(
        agent_kernel_x86_64::ata::ATA_LBA48_SECTOR_LIMIT + 1
    )
    .is_none());
}

#[test]
fn identify_uses_ata_command_and_freezes_lba48_capacity() {
    let config =
        AtaPioConfig::new(COMMAND_BASE, CONTROL_BASE, AtaDrive::Slave, 8).expect("valid config");
    let mut io = RegisterDouble::ready();
    io.queue_data(identify_words(DEVICE_SECTORS));
    let mut device = AtaPioDevice::new(config, io);

    let identity = device.identify().expect("identify");

    assert_eq!(identity.sector_count(), DEVICE_SECTORS);
    assert_eq!(identity.logical_sector_bytes(), ATA_SECTOR_BYTES);
    assert!(device
        .io()
        .writes_u8()
        .contains(&(COMMAND_BASE + 7, ATA_COMMAND_IDENTIFY)));
}

#[test]
fn read_sector_programs_lba48_high_bytes_before_low_bytes() {
    let mut device = identified_device();
    let lba = 0x0000_0102_0304_0506;
    let words = (0..256).map(|word| 0x8000 | word);
    device.io_mut().queue_data(words);
    let mut sector = [0_u8; ATA_SECTOR_BYTES];

    device.read_sector(lba, &mut sector).expect("sector read");

    assert_eq!(
        device.io().writes_u8(),
        vec![
            (COMMAND_BASE + 6, 0x40),
            (COMMAND_BASE + 1, 0),
            (COMMAND_BASE + 2, 0),
            (COMMAND_BASE + 3, 0x03),
            (COMMAND_BASE + 4, 0x02),
            (COMMAND_BASE + 5, 0x01),
            (COMMAND_BASE + 1, 0),
            (COMMAND_BASE + 2, 1),
            (COMMAND_BASE + 3, 0x06),
            (COMMAND_BASE + 4, 0x05),
            (COMMAND_BASE + 5, 0x04),
            (COMMAND_BASE + 7, ATA_COMMAND_READ_EXT),
        ]
    );
    assert_eq!(&sector[..4], &[0x00, 0x80, 0x01, 0x80]);
    assert_eq!(&sector[ATA_SECTOR_BYTES - 2..], &[0xff, 0x80]);
}

#[test]
fn write_sector_uses_word_wide_data_port_and_flush_ext() {
    let mut device = identified_device();
    let mut sector = [0_u8; ATA_SECTOR_BYTES];
    for (index, byte) in sector.iter_mut().enumerate() {
        *byte = index as u8;
    }

    device.write_sector(9, &sector).expect("sector write");
    device.flush_cache().expect("cache flush");

    let words = device.io().writes_u16();
    assert_eq!(words.len(), ATA_SECTOR_BYTES / 2);
    assert_eq!(words[0], (COMMAND_BASE, 0x0100));
    assert_eq!(words[255], (COMMAND_BASE, 0xfffe));
    let byte_writes = device.io().writes_u8();
    assert!(byte_writes.contains(&(COMMAND_BASE + 7, ATA_COMMAND_WRITE_EXT)));
    assert!(byte_writes.contains(&(COMMAND_BASE + 7, ATA_COMMAND_FLUSH_EXT)));
}

#[test]
fn transport_rejects_unidentified_and_out_of_range_access() {
    let config =
        AtaPioConfig::new(COMMAND_BASE, CONTROL_BASE, AtaDrive::Master, 8).expect("valid config");
    let mut device = AtaPioDevice::new(config, RegisterDouble::ready());
    let mut sector = [0_u8; ATA_SECTOR_BYTES];

    assert_eq!(
        device.read_sector(0, &mut sector),
        Err(AtaPioError::DeviceNotIdentified)
    );

    device.io_mut().queue_data(identify_words(16));
    device.identify().expect("identify");
    assert_eq!(
        device.read_sector(16, &mut sector),
        Err(AtaPioError::LbaOutOfRange {
            lba: 16,
            sector_count: 16,
        })
    );
}

#[test]
fn identify_rejects_missing_lba48_and_busy_timeout() {
    let config =
        AtaPioConfig::new(COMMAND_BASE, CONTROL_BASE, AtaDrive::Master, 3).expect("valid config");
    let mut unsupported = RegisterDouble::ready();
    unsupported.queue_data([0_u16; 256]);
    let mut unsupported = AtaPioDevice::new(config, unsupported);
    assert_eq!(unsupported.identify(), Err(AtaPioError::Lba48Unsupported));

    let mut busy = AtaPioDevice::new(config, RegisterDouble::with_status(0x80));
    assert_eq!(busy.identify(), Err(AtaPioError::BusyTimeout));
}

#[test]
fn command_surfaces_device_fault_after_issue() {
    let mut device = identified_device();
    device.io_mut().queue_statuses([0x40, 0x41]);
    let mut sector = [0_u8; ATA_SECTOR_BYTES];

    assert_eq!(
        device.read_sector(1, &mut sector),
        Err(AtaPioError::DeviceFault {
            status: 0x41,
            error: 0,
        })
    );
}
