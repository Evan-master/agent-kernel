use agent_kernel_x86_64::{
    pci::{PciBarIndex, VirtioPciBarRegion},
    virtio_rng::{
        VirtioCommonConfigIo, VirtioIsrIo, VirtioMmioError, VirtioMmioRegionKind, VirtioNotifyIo,
        VolatileVirtioCommonConfig, VolatileVirtioIsr, VolatileVirtioNotify,
    },
};

#[test]
fn volatile_views_access_only_their_validated_bar_regions() {
    let mut mapped = [0u64; 128];
    let base = mapped.as_mut_ptr().cast::<u8>();
    let common_region = region(0x00, 0x38);
    let notify_region = region(0x100, 0x40);
    let isr_region = region(0x200, 1);
    let mut common =
        unsafe { VolatileVirtioCommonConfig::bind(base, 1024, common_region).unwrap() };
    let mut notify = unsafe { VolatileVirtioNotify::bind(base, 1024, notify_region, 8).unwrap() };
    let mut isr = unsafe { VolatileVirtioIsr::bind(base, 1024, isr_region).unwrap() };

    common.write_u8(0x14, 0x0f);
    common.write_u16(0x18, 1);
    common.write_u32(0x08, 1);
    common.write_u64(0x20, 0x0100_1000);
    assert_eq!(common.read_u8(0x14), 0x0f);
    assert_eq!(common.read_u16(0x18), 1);
    assert_eq!(common.read_u32(0x08), 1);
    unsafe {
        assert_eq!(base.add(0x20).cast::<u64>().read(), 0x0100_1000);
    }

    assert_eq!(notify.region_bytes(), 0x40);
    assert_eq!(notify.offset_multiplier(), 8);
    notify.write_u16(24, 0);
    unsafe {
        assert_eq!(base.add(0x118).cast::<u16>().read(), 0);
        base.add(0x200).write(3);
    }
    assert_eq!(isr.read_and_acknowledge(), 3);
}

#[test]
fn volatile_views_reject_short_mappings_regions_and_alignment() {
    let mut mapped = [0u64; 128];
    let base = mapped.as_mut_ptr().cast::<u8>();
    assert_eq!(
        unsafe { VolatileVirtioNotify::bind(base, 0x13f, region(0x100, 0x40), 4) }.err(),
        Some(VirtioMmioError::MappedBarTooSmall {
            required: 0x140,
            actual: 0x13f,
        })
    );
    assert_eq!(
        unsafe { VolatileVirtioCommonConfig::bind(base, 1024, region(0, 0x37)) }.err(),
        Some(VirtioMmioError::RegionTooSmall {
            region: VirtioMmioRegionKind::Common,
            required: 0x38,
            actual: 0x37,
        })
    );
    assert_eq!(
        unsafe { VolatileVirtioCommonConfig::bind(base.add(1), 1023, region(0, 0x38)) }.err(),
        Some(VirtioMmioError::RegionUnaligned {
            region: VirtioMmioRegionKind::Common,
            address: base as usize + 1,
            required: 8,
        })
    );
}

#[test]
#[should_panic]
fn safe_common_view_rejects_access_past_its_region_in_release_builds() {
    let mut mapped = [0u64; 16];
    let base = mapped.as_mut_ptr().cast::<u8>();
    let mut common =
        unsafe { VolatileVirtioCommonConfig::bind(base, 128, region(0, 0x38)).unwrap() };

    let _ = common.read_u8(0x38);
}

fn region(offset: u32, length: u32) -> VirtioPciBarRegion {
    VirtioPciBarRegion::new(PciBarIndex::new(4).unwrap(), offset, length).unwrap()
}
