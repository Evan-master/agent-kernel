use agent_kernel_core::DmaAccess;
use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    iommu::{
        IntelVtd, VtdControllerError, VtdDomainId, VtdLegacyTableAddresses, VtdLegacyTablePages,
        VtdOperation, VtdRegisterIo, VtdTableError, DMAR_CAP_REG, DMAR_CCMD_REG, DMAR_ECAP_REG,
        DMAR_FRCD_HIGH_REG, DMAR_FRCD_LOW_REG, DMAR_FSTS_REG, DMAR_GCMD_REG, DMAR_GSTS_REG,
        DMAR_IOTLB_REG, DMAR_RTADDR_REG, DMAR_VER_REG,
    },
};

const IOVA: u64 = 0x0100_0000;
const SOURCE_ID: u16 = 0x28;
const QEMU_CAP: u64 = (1 << 9) | (38 << 16) | (0x22 << 24);

#[test]
fn legacy_tables_encode_one_39_bit_translation_and_remove_its_leaf() {
    let mut root = [0_u64; 512];
    let mut context = [0_u64; 512];
    let mut level3 = [0_u64; 512];
    let mut level2 = [0_u64; 512];
    let mut level1 = [0_u64; 512];
    let addresses = VtdLegacyTableAddresses::new(0x1000, 0x2000, 0x3000, 0x4000, 0x5000).unwrap();
    let requester = DmarPciRequester::new(0, 0, 5, 0).unwrap();
    let domain = VtdDomainId::new(1).unwrap();
    let mut tables = VtdLegacyTablePages::new(
        &mut root,
        &mut context,
        &mut level3,
        &mut level2,
        &mut level1,
        addresses,
    );

    tables
        .install(requester, domain, IOVA, 0x6000, DmaAccess::ReadWrite)
        .unwrap();

    assert_eq!(tables.root_entries()[0], 0x2001);
    assert_eq!(tables.root_entries()[1], 0);
    assert_eq!(tables.context_entries()[80], 0x3001);
    assert_eq!(tables.context_entries()[81], 0x0101);
    assert_eq!(tables.level3_entries()[0], 0x4003);
    assert_eq!(tables.level2_entries()[8], 0x5003);
    assert_eq!(tables.level1_entries()[0], 0x6003);
    assert_eq!(
        tables.install(requester, domain, IOVA, 0x7000, DmaAccess::Read),
        Err(VtdTableError::MappingAlreadyPresent)
    );

    tables.remove(IOVA).unwrap();
    assert_eq!(tables.level1_entries()[0], 0);
    assert_eq!(
        tables.install(
            DmarPciRequester::new(0, 0, 6, 0).unwrap(),
            domain,
            IOVA,
            0x7000,
            DmaAccess::Read,
        ),
        Err(VtdTableError::RequesterMismatch)
    );
    assert_eq!(
        tables.install(
            requester,
            VtdDomainId::new(2).unwrap(),
            IOVA,
            0x7000,
            DmaAccess::Read,
        ),
        Err(VtdTableError::DomainMismatch)
    );
}

#[test]
fn controller_programs_root_invalidations_and_translation_in_order() {
    let io = ScriptedRegisters::new();
    let mut controller = IntelVtd::bind(io, 8).unwrap();

    controller.activate(0x1000).unwrap();
    let io = controller.into_io();

    assert_eq!(
        io.writes,
        [
            Write::U64(DMAR_RTADDR_REG, 0x1000),
            Write::U32(DMAR_GCMD_REG, 1 << 30),
            Write::U64(DMAR_CCMD_REG, (1_u64 << 63) | (1_u64 << 61)),
            Write::U64(DMAR_IOTLB_REG, (1_u64 << 63) | (1_u64 << 60)),
            Write::U32(DMAR_GCMD_REG, 1 << 31),
        ]
    );
}

#[test]
fn controller_decodes_and_clears_a_dma_fault_record() {
    let mut io = ScriptedRegisters::new();
    io.fault_status = 1 << 1;
    io.fault_low = IOVA;
    io.fault_high = (1_u64 << 63) | (7_u64 << 32) | u64::from(SOURCE_ID);
    let mut controller = IntelVtd::bind(io, 8).unwrap();

    let fault = controller.fault_record().unwrap().unwrap();
    assert_eq!(fault.source_id(), SOURCE_ID);
    assert_eq!(fault.reason(), 7);
    assert_eq!(fault.address(), IOVA);
    assert!(fault.write());
    controller.clear_fault().unwrap();
    assert_eq!(controller.fault_record().unwrap(), None);
}

#[test]
fn controller_rejects_iotlb_registers_outside_the_mapped_page() {
    let mut missing = ScriptedRegisters::new();
    missing.ecap = 0;
    assert_eq!(
        IntelVtd::bind(missing, 8).err(),
        Some(agent_kernel_x86_64::iommu::VtdControllerError::InvalidIotlbOffset(8))
    );

    let mut outside = ScriptedRegisters::new();
    outside.ecap = 0x100_u64 << 8;
    assert_eq!(
        IntelVtd::bind(outside, 8).err(),
        Some(agent_kernel_x86_64::iommu::VtdControllerError::InvalidIotlbOffset(0x1008))
    );
}

#[test]
fn controller_rejects_incompatible_width_and_fault_register_layout() {
    let mut narrow = ScriptedRegisters::new();
    narrow.cap = (1 << 9) | (37 << 16) | (0x22 << 24);
    assert_eq!(
        IntelVtd::bind(narrow, 8).err(),
        Some(VtdControllerError::InsufficientGuestAddressWidth(38))
    );

    let mut misplaced = ScriptedRegisters::new();
    misplaced.cap = (1 << 9) | (38 << 16);
    assert_eq!(
        IntelVtd::bind(misplaced, 8).err(),
        Some(VtdControllerError::UnsupportedFaultRecording {
            offset: 0,
            records: 1,
        })
    );

    let mut multiple = ScriptedRegisters::new();
    multiple.cap |= 1 << 40;
    assert_eq!(
        IntelVtd::bind(multiple, 8).err(),
        Some(VtdControllerError::UnsupportedFaultRecording {
            offset: 0x220,
            records: 2,
        })
    );
}

#[test]
fn controller_rejects_invalidation_without_global_completion() {
    let mut context = ScriptedRegisters::new();
    context.context_actual = 0;
    let mut controller = IntelVtd::bind(context, 8).unwrap();
    assert_eq!(
        controller.activate(0x1000),
        Err(VtdControllerError::UnexpectedInvalidationGranularity(
            VtdOperation::ContextInvalidation
        ))
    );

    let mut iotlb = ScriptedRegisters::new();
    iotlb.iotlb_actual = 0;
    let mut controller = IntelVtd::bind(iotlb, 8).unwrap();
    assert_eq!(
        controller.activate(0x1000),
        Err(VtdControllerError::UnexpectedInvalidationGranularity(
            VtdOperation::IotlbInvalidation
        ))
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Write {
    U32(u16, u32),
    U64(u16, u64),
}

struct ScriptedRegisters {
    writes: Vec<Write>,
    status: u32,
    cap: u64,
    context_command: u64,
    context_actual: u64,
    iotlb_command: u64,
    iotlb_actual: u64,
    ecap: u64,
    fault_status: u32,
    fault_low: u64,
    fault_high: u64,
}

impl ScriptedRegisters {
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            status: 0,
            cap: QEMU_CAP,
            context_command: 0,
            context_actual: 1 << 59,
            iotlb_command: 0,
            iotlb_actual: 1 << 57,
            ecap: 0x0f00,
            fault_status: 0,
            fault_low: 0,
            fault_high: 0,
        }
    }
}

impl VtdRegisterIo for ScriptedRegisters {
    fn read_u32(&mut self, offset: u16) -> u32 {
        match offset {
            DMAR_VER_REG => 0x10,
            DMAR_GSTS_REG => self.status,
            DMAR_FSTS_REG => self.fault_status,
            _ => 0,
        }
    }

    fn write_u32(&mut self, offset: u16, value: u32) {
        self.writes.push(Write::U32(offset, value));
        if offset == DMAR_GCMD_REG {
            self.status &= !((1 << 31) | (1 << 30));
            self.status |= value & ((1 << 31) | (1 << 30));
        } else if offset == DMAR_FSTS_REG {
            self.fault_status &= !value;
        }
    }

    fn read_u64(&mut self, offset: u16) -> u64 {
        match offset {
            DMAR_CAP_REG => self.cap,
            DMAR_ECAP_REG => self.ecap,
            DMAR_CCMD_REG => self.context_command,
            DMAR_IOTLB_REG => self.iotlb_command,
            DMAR_FRCD_LOW_REG => self.fault_low,
            DMAR_FRCD_HIGH_REG => self.fault_high,
            _ => 0,
        }
    }

    fn write_u64(&mut self, offset: u16, value: u64) {
        self.writes.push(Write::U64(offset, value));
        match offset {
            DMAR_CCMD_REG => self.context_command = self.context_actual,
            DMAR_IOTLB_REG => self.iotlb_command = self.iotlb_actual,
            DMAR_FRCD_HIGH_REG => {
                self.fault_high &= !value;
                if self.fault_high & (1 << 63) == 0 {
                    self.fault_status &= !(1 << 1);
                }
            }
            _ => {}
        }
    }
}
