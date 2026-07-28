//! Native modern Virtio entropy-device support.
//!
//! This x86 architecture layer owns the Virtio 1.x PCI transport state machine,
//! one fixed split virtqueue, DMA-visible queue memory encoding, volatile MMIO
//! adapters, and bounded completion validation. PCI authority and VT-d mapping
//! lifecycle remain with their dedicated owners.

mod device;
mod mmio;
mod pci;
mod queue;
mod transport;

pub use device::{VirtioRngDevice, VirtioRngDeviceError};
pub use mmio::{
    VirtioMmioError, VirtioMmioRegionKind, VolatileVirtioCommonConfig, VolatileVirtioIsr,
    VolatileVirtioNotify,
};
pub use pci::{
    virtio_rng_selector, VirtioRngPciCapabilities, VirtioRngPciCapabilityError, VirtioRngPciRegion,
    VirtioRngPciRegionError, VirtioRngPciRegions, VirtioRngRequiredCapability,
};
pub use queue::{
    VirtioRngCompletion, VirtioRngQueueError, VirtioRngQueueLayout, VirtioRngQueueMemory,
    VirtioRngRequest, VIRTIO_RNG_AVAILABLE_OFFSET, VIRTIO_RNG_DESCRIPTOR_OFFSET,
    VIRTIO_RNG_ENTROPY_BYTES, VIRTIO_RNG_USED_OFFSET,
};
pub use transport::{
    VirtioCommonConfigIo, VirtioIsrIo, VirtioNotifyIo, VirtioRngInterrupt, VirtioRngTransport,
    VirtioRngTransportError, VIRTIO_F_ACCESS_PLATFORM, VIRTIO_F_VERSION_1,
    VIRTIO_PCI_STATUS_ACKNOWLEDGE, VIRTIO_PCI_STATUS_DRIVER, VIRTIO_PCI_STATUS_DRIVER_OK,
    VIRTIO_PCI_STATUS_FAILED, VIRTIO_PCI_STATUS_FEATURES_OK, VIRTIO_RNG_DEVICE_ID,
    VIRTIO_RNG_VENDOR_ID,
};
