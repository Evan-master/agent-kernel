use agent_kernel_x86_64::tpm2::{CrbIo, CrbMmioError, VolatileCrbIo};

const PHYSICAL_BASE: u64 = 0xfed4_0000;

#[repr(C, align(4096))]
struct MappedPage([u8; 4096]);

#[test]
fn volatile_crb_io_translates_only_the_registered_window() {
    let mut page = MappedPage([0; 4096]);
    let virtual_base = page.0.as_mut_ptr() as u64;
    let mut io = unsafe {
        VolatileCrbIo::new(PHYSICAL_BASE, virtual_base, page.0.len()).expect("valid MMIO window")
    };

    io.write_u32(PHYSICAL_BASE + 0x40, 0x1122_3344).unwrap();
    io.write_bytes(PHYSICAL_BASE + 0x80, &[0xaa, 0xbb, 0xcc])
        .unwrap();

    assert_eq!(io.read_u32(PHYSICAL_BASE + 0x40), Ok(0x1122_3344));
    let mut bytes = [0; 3];
    io.read_bytes(PHYSICAL_BASE + 0x80, &mut bytes).unwrap();
    assert_eq!(bytes, [0xaa, 0xbb, 0xcc]);
}

#[test]
fn volatile_crb_io_rejects_unaligned_and_out_of_window_access() {
    let mut page = MappedPage([0; 4096]);
    let virtual_base = page.0.as_mut_ptr() as u64;
    let mut io = unsafe { VolatileCrbIo::new(PHYSICAL_BASE, virtual_base, page.0.len()).unwrap() };

    assert_eq!(
        io.read_u32(PHYSICAL_BASE + 1),
        Err(CrbMmioError::UnalignedRegister {
            address: PHYSICAL_BASE + 1
        })
    );
    assert_eq!(
        io.write_bytes(PHYSICAL_BASE + 4095, &[1, 2]),
        Err(CrbMmioError::OutsideWindow {
            address: PHYSICAL_BASE + 4095,
            length: 2
        })
    );
    assert_eq!(
        io.read_u32(PHYSICAL_BASE - 4),
        Err(CrbMmioError::OutsideWindow {
            address: PHYSICAL_BASE - 4,
            length: 4
        })
    );
}

#[test]
fn volatile_crb_io_rejects_invalid_windows() {
    let page = MappedPage([0; 4096]);
    let virtual_base = page.0.as_ptr() as u64;

    assert_eq!(
        unsafe { VolatileCrbIo::new(PHYSICAL_BASE + 1, virtual_base, page.0.len()) },
        Err(CrbMmioError::InvalidWindow)
    );
    assert_eq!(
        unsafe { VolatileCrbIo::new(PHYSICAL_BASE, virtual_base, 0) },
        Err(CrbMmioError::InvalidWindow)
    );
}
