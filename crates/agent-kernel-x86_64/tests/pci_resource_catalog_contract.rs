use agent_kernel::AgentKernel;
use agent_kernel_core::{
    AgentId, DriverEndpointDescriptor, DriverResourceTreeSpec, Operation, OperationSet,
    ResourceKind,
};
use agent_kernel_x86_64::pci::{
    discover_pci_functions, probe_pci_resource_catalog, PciBarIndex, PciBarProbeError,
    PciConfigAccess, PciConfigMutationAccess, PciConfigRegister, PciFunctionAddress,
    PciFunctionClaim, PciFunctionClaimError, PciResourceCatalogError,
};

type ClaimKernel = AgentKernel<1, 8, 8, 64, 0, 0, 0, 0, 0, 0>;

#[derive(Clone)]
struct FunctionModel {
    address: PciFunctionAddress,
    identity: u32,
    command_status: u32,
    class_revision: u32,
    header: u32,
    bars: [u32; 6],
    masks: [u32; 6],
}

impl FunctionModel {
    fn endpoint(
        address: PciFunctionAddress,
        class: (u8, u8, u8),
        bars: [u32; 6],
        masks: [u32; 6],
    ) -> Self {
        Self {
            address,
            identity: 0x1000_1af4,
            command_status: 0x0100_0007,
            class_revision: 1
                | (u32::from(class.2) << 8)
                | (u32::from(class.1) << 16)
                | (u32::from(class.0) << 24),
            header: 0,
            bars,
            masks,
        }
    }

    fn bridge(address: PciFunctionAddress) -> Self {
        let mut model = Self::endpoint(address, (0x06, 0x04, 0), [0; 6], [0; 6]);
        model.header = 1 << 16;
        model
    }
}

struct FabricConfig {
    functions: Vec<FunctionModel>,
}

impl FabricConfig {
    fn representative() -> Self {
        Self {
            functions: vec![
                FunctionModel::bridge(PciFunctionAddress::new(0, 0, 0).unwrap()),
                FunctionModel::endpoint(
                    PciFunctionAddress::new(0, 1, 0).unwrap(),
                    (0x03, 0x00, 0),
                    [0x0000_0008, 0, 0, 0, 0, 0],
                    [0xffff_f008, 0, 0, 0, 0, 0],
                ),
                FunctionModel::endpoint(
                    PciFunctionAddress::new(0, 2, 0).unwrap(),
                    (0x02, 0x00, 0),
                    [0x9000_0000, 0x0000_c001, 0, 0, 0, 0],
                    [0xffff_f000, 0xffff_ff01, 0, 0, 0, 0],
                ),
                FunctionModel::endpoint(
                    PciFunctionAddress::new(2, 0, 0).unwrap(),
                    (0x0c, 0x03, 0x30),
                    [0xa000_0000, 0, 0, 0, 0, 0],
                    [0xffff_0000, 0, 0, 0, 0, 0],
                ),
            ],
        }
    }

    fn function(&self, address: PciFunctionAddress) -> Option<&FunctionModel> {
        self.functions
            .iter()
            .find(|function| function.address == address)
    }

    fn function_mut(&mut self, address: PciFunctionAddress) -> Option<&mut FunctionModel> {
        self.functions
            .iter_mut()
            .find(|function| function.address == address)
    }
}

impl PciConfigAccess for FabricConfig {
    fn read_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister) -> u32 {
        let offset = register.offset();
        let Some(function) = self.function(address) else {
            return if offset == 0 { u32::MAX } else { 0 };
        };
        match offset {
            0x00 => function.identity,
            0x04 => function.command_status,
            0x08 => function.class_revision,
            0x0c => function.header,
            0x10..=0x24 if offset & 3 == 0 => {
                let index = usize::from((offset - 0x10) / 4);
                if function.bars[index] == u32::MAX {
                    function.masks[index]
                } else {
                    function.bars[index]
                }
            }
            _ => 0,
        }
    }
}

impl PciConfigMutationAccess for FabricConfig {
    fn write_u32(&mut self, address: PciFunctionAddress, register: PciConfigRegister, value: u32) {
        let function = self.function_mut(address).unwrap();
        match register.offset() {
            0x04 => {
                let status = (function.command_status >> 16) as u16 & !((value >> 16) as u16);
                function.command_status = (u32::from(status) << 16) | u32::from(value as u16);
            }
            offset @ 0x10..=0x24 if offset & 3 == 0 => {
                function.bars[usize::from((offset - 0x10) / 4)] = value;
            }
            _ => panic!("unexpected configuration mutation"),
        }
    }
}

#[test]
fn catalog_preserves_bdf_order_and_selects_first_fully_assigned_endpoint() {
    let mut fabric = FabricConfig::representative();
    let inventory = discover_pci_functions::<_, 8>(&mut fabric).unwrap();

    let catalog = probe_pci_resource_catalog::<_, 8, 8>(&mut fabric, &inventory).unwrap();

    assert_eq!(catalog.len(), 3);
    assert_eq!(
        catalog
            .functions()
            .iter()
            .map(|resources| resources.function().address().coordinates())
            .collect::<Vec<_>>(),
        [(0, 1, 0), (0, 2, 0), (2, 0, 0)]
    );
    assert!(!catalog.functions()[0].bars().all_assigned());
    let candidate = catalog.claim_candidate().unwrap();
    assert_eq!(candidate.function().address().coordinates(), (0, 2, 0));
    let spec = candidate.driver_resource_spec().unwrap();
    assert_eq!(spec.root_kind(), ResourceKind::Network);
    assert_eq!(
        spec.regions()[0],
        Some(DriverEndpointDescriptor::mmio(0x9000_0000, 0x1000))
    );
    assert_eq!(
        spec.regions()[1],
        Some(DriverEndpointDescriptor::port(0xc000, 0x100))
    );
}

#[test]
fn catalog_capacity_and_probe_failures_publish_no_partial_catalog() {
    let mut full_fabric = FabricConfig::representative();
    let inventory = discover_pci_functions::<_, 8>(&mut full_fabric).unwrap();
    assert_eq!(
        probe_pci_resource_catalog::<_, 8, 2>(&mut full_fabric, &inventory),
        Err(PciResourceCatalogError::CatalogFull {
            capacity: 2,
            address: PciFunctionAddress::new(2, 0, 0).unwrap(),
        })
    );

    let mut malformed = FabricConfig::representative();
    malformed.functions[1].bars[0] = 0x8000_0006;
    malformed.functions[1].masks[0] = 0xffff_f006;
    let inventory = discover_pci_functions::<_, 8>(&mut malformed).unwrap();
    let original_command = malformed.functions[1].command_status;
    assert_eq!(
        probe_pci_resource_catalog::<_, 8, 8>(&mut malformed, &inventory),
        Err(PciResourceCatalogError::Probe {
            address: PciFunctionAddress::new(0, 1, 0).unwrap(),
            error: PciBarProbeError::ReservedMemoryType {
                index: PciBarIndex::new(0).unwrap(),
            },
        })
    );
    assert_eq!(malformed.functions[1].command_status, original_command);
    assert_eq!(malformed.functions[1].bars[0], 0x8000_0006);
}

#[test]
fn function_claim_binds_each_bar_slot_to_the_exact_kernel_authority() {
    let mut fabric = FabricConfig::representative();
    let inventory = discover_pci_functions::<_, 8>(&mut fabric).unwrap();
    let catalog = probe_pci_resource_catalog::<_, 8, 8>(&mut fabric, &inventory).unwrap();
    let candidate = catalog.claim_candidate().unwrap();
    let mut kernel = prepared_kernel();
    let parent = kernel.resources()[0].id;
    let authority = kernel
        .capability(agent_kernel_core::CapabilityId::new(1))
        .unwrap();
    let tree = kernel
        .sys_create_driver_resource_tree(
            AgentId::new(1),
            Some((parent, authority.id)),
            owner_operations(),
            candidate.driver_resource_spec().unwrap(),
        )
        .unwrap();

    let claim = PciFunctionClaim::new(candidate, tree).unwrap();

    assert_eq!(claim.function().address().coordinates(), (0, 2, 0));
    assert_eq!(claim.root(), tree.root());
    for bar in claim.bars().bars() {
        let region = claim.bar_region(bar.index()).unwrap();
        assert_eq!(region.slot(), bar.index().number());
        assert_eq!(
            region.descriptor(),
            candidate.driver_resource_spec().unwrap().regions()[usize::from(bar.index().number())]
                .unwrap()
        );
        assert_eq!(
            kernel.capability(region.capability()).unwrap().resource,
            region.resource()
        );
    }
}

#[test]
fn function_claim_rejects_a_resource_tree_with_different_physical_regions() {
    let mut fabric = FabricConfig::representative();
    let inventory = discover_pci_functions::<_, 8>(&mut fabric).unwrap();
    let catalog = probe_pci_resource_catalog::<_, 8, 8>(&mut fabric, &inventory).unwrap();
    let candidate = catalog.claim_candidate().unwrap();
    let mut kernel = prepared_kernel();
    let parent = kernel.resources()[0].id;
    let authority = agent_kernel_core::CapabilityId::new(1);
    let mismatched = DriverResourceTreeSpec::new(
        ResourceKind::Network,
        [
            Some(DriverEndpointDescriptor::mmio(0x9100_0000, 0x1000)),
            Some(DriverEndpointDescriptor::port(0xc000, 0x100)),
            None,
            None,
            None,
            None,
        ],
    );
    let tree = kernel
        .sys_create_driver_resource_tree(
            AgentId::new(1),
            Some((parent, authority)),
            owner_operations(),
            mismatched,
        )
        .unwrap();

    assert_eq!(
        PciFunctionClaim::new(candidate, tree),
        Err(PciFunctionClaimError::ResourceTreeMismatch { slot: 0 })
    );
}

fn prepared_kernel() -> ClaimKernel {
    let mut kernel = ClaimKernel::new();
    let owner = AgentId::new(1);
    kernel.sys_register_agent(owner).unwrap();
    let parent = kernel
        .sys_register_resource(ResourceKind::Workspace, None)
        .unwrap();
    assert_eq!(
        kernel.sys_grant(owner, parent, owner_operations()).unwrap(),
        agent_kernel_core::CapabilityId::new(1)
    );
    kernel
}

const fn owner_operations() -> OperationSet {
    OperationSet::only(Operation::Observe)
        .with(Operation::Act)
        .with(Operation::Rollback)
        .with(Operation::Delegate)
}
