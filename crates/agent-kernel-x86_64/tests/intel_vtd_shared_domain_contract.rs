use agent_kernel_core::DmaAccess;
use agent_kernel_x86_64::{
    acpi_topology::DmarPciRequester,
    iommu::{
        VtdDomainId, VtdLegacyTableAddresses, VtdLegacyTablePages, VtdTableError,
        VTD_MAPPING_CAPACITY, VTD_REQUESTER_CAPACITY,
    },
};

const EDU_IOVA: u64 = 0x0100_0000;
const QUEUE_IOVA: u64 = 0x0100_1000;
const ENTROPY_IOVA: u64 = 0x0100_2000;

#[test]
fn two_requesters_share_one_bus_domain_and_translation_hierarchy() {
    let mut storage = TableStorage::new();
    storage.with_tables(|tables| {
        let domain = VtdDomainId::new(1).unwrap();
        let edu = requester(5, 0);
        let rng = requester(6, 0);

        tables.attach_requester(edu, domain).unwrap();
        tables.attach_requester(rng, domain).unwrap();

        assert_eq!(tables.active_requester_count(), 2);
        assert_eq!(tables.root_entries()[0], 0x2001);
        assert_eq!(&tables.context_entries()[80..82], &[0x3001, 0x0101]);
        assert_eq!(&tables.context_entries()[96..98], &[0x3001, 0x0101]);
        assert_eq!(
            tables.attach_requester(edu, domain),
            Err(VtdTableError::RequesterAlreadyPresent)
        );
        assert_eq!(
            tables.attach_requester(requester_on_bus(1, 5, 0), domain),
            Err(VtdTableError::PciBusMismatch {
                expected: 0,
                actual: 1,
            })
        );
        assert_eq!(
            tables.attach_requester(requester(7, 0), VtdDomainId::new(2).unwrap()),
            Err(VtdTableError::DomainMismatch)
        );
        assert_eq!(tables.active_requester_count(), 2);
    });
}

#[test]
fn multiple_leaves_are_installed_and_removed_independently() {
    let mut storage = TableStorage::new();
    storage.with_tables(|tables| {
        tables
            .attach_requester(requester(5, 0), VtdDomainId::new(1).unwrap())
            .unwrap();
        tables
            .install_mapping(EDU_IOVA, 0x6000, DmaAccess::ReadWrite)
            .unwrap();
        tables
            .install_mapping(QUEUE_IOVA, 0x7000, DmaAccess::ReadWrite)
            .unwrap();
        tables
            .install_mapping(ENTROPY_IOVA, 0x8000, DmaAccess::Write)
            .unwrap();

        assert_eq!(tables.active_mapping_count(), 3);
        assert_eq!(&tables.level1_entries()[0..3], &[0x6003, 0x7003, 0x8002]);
        assert_eq!(
            tables.install_mapping(QUEUE_IOVA, 0x9000, DmaAccess::Read),
            Err(VtdTableError::MappingAlreadyPresent)
        );
        assert_eq!(
            tables.install_mapping(EDU_IOVA + 0x20_0000, 0x9000, DmaAccess::Read),
            Err(VtdTableError::MappingWindowMismatch {
                expected: EDU_IOVA,
                actual: EDU_IOVA + 0x20_0000,
            })
        );

        tables.remove_mapping(QUEUE_IOVA).unwrap();
        assert_eq!(tables.active_mapping_count(), 2);
        assert_eq!(&tables.level1_entries()[0..3], &[0x6003, 0, 0x8002]);
        assert_eq!(tables.level3_entries()[0], 0x4003);
        assert_eq!(tables.level2_entries()[8], 0x5003);

        tables.remove_mapping(EDU_IOVA).unwrap();
        tables.remove_mapping(ENTROPY_IOVA).unwrap();
        assert_eq!(tables.active_mapping_count(), 0);
        assert_eq!(tables.level3_entries()[0], 0);
        assert_eq!(tables.level2_entries()[8], 0);

        let next_window = EDU_IOVA + 0x20_0000;
        tables
            .install_mapping(next_window, 0x9000, DmaAccess::Read)
            .unwrap();
        assert_eq!(tables.level1_entries()[0], 0x9001);
    });
}

#[test]
fn detaching_one_requester_preserves_other_contexts_and_all_leaves() {
    let mut storage = TableStorage::new();
    storage.with_tables(|tables| {
        let domain = VtdDomainId::new(1).unwrap();
        let edu = requester(5, 0);
        let rng = requester(6, 0);
        tables.attach_requester(edu, domain).unwrap();
        tables.attach_requester(rng, domain).unwrap();
        tables
            .install_mapping(EDU_IOVA, 0x6000, DmaAccess::ReadWrite)
            .unwrap();
        tables
            .install_mapping(ENTROPY_IOVA, 0x8000, DmaAccess::Write)
            .unwrap();

        tables.detach_requester(rng, domain).unwrap();

        assert_eq!(tables.active_requester_count(), 1);
        assert_eq!(&tables.context_entries()[96..98], &[0, 0]);
        assert_eq!(&tables.context_entries()[80..82], &[0x3001, 0x0101]);
        assert_eq!(&tables.level1_entries()[0..3], &[0x6003, 0, 0x8002]);
        assert_eq!(
            tables.detach_requester(rng, domain),
            Err(VtdTableError::RequesterNotPresent)
        );
    });
}

#[test]
fn one_table_set_accepts_every_function_on_its_bound_bus() {
    assert_eq!(VTD_REQUESTER_CAPACITY, 256);
    assert_eq!(VTD_MAPPING_CAPACITY, 512);
    let mut storage = TableStorage::new();
    storage.with_tables(|tables| {
        let domain = VtdDomainId::new(1).unwrap();
        for device in 0..32 {
            for function in 0..8 {
                tables
                    .attach_requester(requester(device, function), domain)
                    .unwrap();
            }
        }

        assert_eq!(tables.active_requester_count(), VTD_REQUESTER_CAPACITY);
        assert!(tables
            .context_entries()
            .as_chunks::<2>()
            .0
            .iter()
            .all(|entry| *entry == [0x3001, 0x0101]));
    });
}

#[test]
fn one_iova_window_accepts_all_512_leaves_and_reuses_a_removed_slot() {
    let mut storage = TableStorage::new();
    storage.with_tables(|tables| {
        assert_eq!(
            tables.install_mapping(EDU_IOVA, 0x2000_0000, DmaAccess::Read),
            Err(VtdTableError::NoRequesterPresent)
        );
        assert_eq!(tables.active_mapping_count(), 0);

        tables
            .attach_requester(requester(5, 0), VtdDomainId::new(1).unwrap())
            .unwrap();
        for leaf in 0..VTD_MAPPING_CAPACITY {
            let offset = leaf as u64 * 4096;
            tables
                .install_mapping(
                    EDU_IOVA + offset,
                    0x2000_0000 + offset,
                    DmaAccess::ReadWrite,
                )
                .unwrap();
        }
        assert_eq!(tables.active_mapping_count(), VTD_MAPPING_CAPACITY);
        assert!(tables.level1_entries().iter().all(|entry| *entry != 0));

        let reusable = EDU_IOVA + 255 * 4096;
        tables.remove_mapping(reusable).unwrap();
        tables
            .install_mapping(reusable, 0x3000_0000, DmaAccess::Write)
            .unwrap();
        assert_eq!(tables.level1_entries()[255], 0x3000_0002);
        assert_eq!(tables.active_mapping_count(), VTD_MAPPING_CAPACITY);
    });
}

fn requester(device: u8, function: u8) -> DmarPciRequester {
    requester_on_bus(0, device, function)
}

fn requester_on_bus(bus: u8, device: u8, function: u8) -> DmarPciRequester {
    DmarPciRequester::new(0, bus, device, function).unwrap()
}

struct TableStorage {
    root: [u64; 512],
    context: [u64; 512],
    level3: [u64; 512],
    level2: [u64; 512],
    level1: [u64; 512],
}

impl TableStorage {
    fn new() -> Self {
        Self {
            root: [0; 512],
            context: [0; 512],
            level3: [0; 512],
            level2: [0; 512],
            level1: [0; 512],
        }
    }

    fn with_tables<T>(&mut self, test: impl FnOnce(&mut VtdLegacyTablePages<'_>) -> T) -> T {
        let addresses =
            VtdLegacyTableAddresses::new(0x1000, 0x2000, 0x3000, 0x4000, 0x5000).unwrap();
        let mut tables = VtdLegacyTablePages::new(
            &mut self.root,
            &mut self.context,
            &mut self.level3,
            &mut self.level2,
            &mut self.level1,
            addresses,
        );
        test(&mut tables)
    }
}
