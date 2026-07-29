//! Split-queue register programming for the Virtio network transport.
//!
//! This x86 architecture helper validates queue presence, MSI-X binding, DMA
//! addresses, and Notify bounds. It performs no device lifecycle transitions.

use crate::virtio_rng::{VirtioCommonConfigIo, VirtioNotifyIo};

use super::{VirtioNetQueueLayout, VirtioNetTransportError};

const QUEUE_SELECT: u16 = 0x16;
const QUEUE_SIZE: u16 = 0x18;
const QUEUE_MSIX_VECTOR: u16 = 0x1a;
const QUEUE_ENABLE: u16 = 0x1c;
const QUEUE_NOTIFY_OFFSET: u16 = 0x1e;
const QUEUE_DESCRIPTOR: u16 = 0x20;
const QUEUE_DRIVER: u16 = 0x28;
const QUEUE_DEVICE: u16 = 0x30;
const QUEUE_ENTRY_COUNT: u16 = 1;

pub(super) fn configure_queue<C: VirtioCommonConfigIo, N: VirtioNotifyIo>(
    common: &mut C,
    notify: &N,
    queue: u16,
    queue_count: u16,
    layout: VirtioNetQueueLayout,
    vector: u16,
) -> Result<u32, VirtioNetTransportError> {
    if queue >= queue_count {
        return Err(VirtioNetTransportError::QueueUnavailable(queue));
    }
    common.write_u16(QUEUE_SELECT, queue);
    if common.read_u16(QUEUE_ENABLE) != 0 {
        return Err(VirtioNetTransportError::QueueAlreadyEnabled(queue));
    }
    if common.read_u16(QUEUE_SIZE) < QUEUE_ENTRY_COUNT {
        return Err(VirtioNetTransportError::QueueUnavailable(queue));
    }
    common.write_u16(QUEUE_SIZE, QUEUE_ENTRY_COUNT);
    let actual_size = common.read_u16(QUEUE_SIZE);
    if actual_size != QUEUE_ENTRY_COUNT {
        return Err(VirtioNetTransportError::QueueSizeRejected {
            queue,
            expected: QUEUE_ENTRY_COUNT,
            actual: actual_size,
        });
    }
    common.write_u16(QUEUE_MSIX_VECTOR, vector);
    let actual_vector = common.read_u16(QUEUE_MSIX_VECTOR);
    if actual_vector != vector {
        return Err(VirtioNetTransportError::QueueVectorRejected {
            queue,
            expected: vector,
            actual: actual_vector,
        });
    }
    common.write_u64(QUEUE_DESCRIPTOR, layout.descriptor_iova());
    common.write_u64(QUEUE_DRIVER, layout.driver_iova());
    common.write_u64(QUEUE_DEVICE, layout.device_iova());
    let notify_index = u32::from(common.read_u16(QUEUE_NOTIFY_OFFSET));
    let offset = notify_index
        .checked_mul(notify.offset_multiplier())
        .ok_or(VirtioNetTransportError::NotifyOffsetOverflow(queue))?;
    let region_bytes = notify.region_bytes();
    if offset.checked_add(2).is_none_or(|end| end > region_bytes) {
        return Err(VirtioNetTransportError::NotifyOutsideRegion {
            queue,
            offset,
            region_bytes,
        });
    }
    common.write_u16(QUEUE_ENABLE, 1);
    if common.read_u16(QUEUE_ENABLE) != 1 {
        return Err(VirtioNetTransportError::QueueEnableRejected(queue));
    }
    Ok(offset)
}
