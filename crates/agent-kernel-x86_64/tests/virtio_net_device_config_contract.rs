use agent_kernel_x86_64::{
    pci::{PciBarIndex, VirtioPciBarRegion},
    virtio_net::{
        VirtioNetDeviceConfigError, VirtioNetDeviceConfigIo, VolatileVirtioNetDeviceConfig,
    },
};

#[test]
fn volatile_device_configuration_reads_the_six_byte_mac_region() {
    let mut mapped = [0_u64; 64];
    let base = mapped.as_mut_ptr().cast::<u8>();
    let region = VirtioPciBarRegion::new(PciBarIndex::new(4).unwrap(), 0x80, 8).unwrap();
    unsafe {
        for (index, byte) in [0x52, 0x54, 0, 0x12, 0x34, 0x56]
            .iter()
            .copied()
            .enumerate()
        {
            base.add(0x80 + index).write(byte);
        }
    }
    let mut config = unsafe { VolatileVirtioNetDeviceConfig::bind(base, 512, region).unwrap() };
    assert_eq!(config.read_mac(), [0x52, 0x54, 0, 0x12, 0x34, 0x56]);
}

#[test]
fn device_configuration_rejects_short_regions_and_mappings() {
    let mut mapped = [0_u64; 64];
    let base = mapped.as_mut_ptr().cast::<u8>();
    let short = VirtioPciBarRegion::new(PciBarIndex::new(4).unwrap(), 0x80, 5).unwrap();
    assert_eq!(
        unsafe { VolatileVirtioNetDeviceConfig::bind(base, 512, short) }.err(),
        Some(VirtioNetDeviceConfigError::RegionTooSmall {
            required: 6,
            actual: 5,
        })
    );
    let outside = VirtioPciBarRegion::new(PciBarIndex::new(4).unwrap(), 0x80, 8).unwrap();
    assert_eq!(
        unsafe { VolatileVirtioNetDeviceConfig::bind(base, 0x87, outside) }.err(),
        Some(VirtioNetDeviceConfigError::MappedBarTooSmall {
            required: 0x88,
            actual: 0x87,
        })
    );
}
