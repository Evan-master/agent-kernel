use agent_kernel_x86_64::virtio_rng::{
    VirtioRngQueueError, VirtioRngQueueLayout, VirtioRngQueueMemory, VIRTIO_RNG_AVAILABLE_OFFSET,
    VIRTIO_RNG_DESCRIPTOR_OFFSET, VIRTIO_RNG_USED_OFFSET,
};

const QUEUE_IOVA: u64 = 0x0100_1000;
const ENTROPY_IOVA: u64 = 0x0100_2000;

#[test]
fn one_descriptor_queue_has_disjoint_aligned_dma_regions() {
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();

    assert_eq!(
        layout.descriptor_iova(),
        QUEUE_IOVA + u64::from(VIRTIO_RNG_DESCRIPTOR_OFFSET)
    );
    assert_eq!(
        layout.driver_iova(),
        QUEUE_IOVA + u64::from(VIRTIO_RNG_AVAILABLE_OFFSET)
    );
    assert_eq!(
        layout.device_iova(),
        QUEUE_IOVA + u64::from(VIRTIO_RNG_USED_OFFSET)
    );
    assert_eq!(layout.entropy_iova(), ENTROPY_IOVA);
    assert_eq!(
        VirtioRngQueueLayout::new(QUEUE_IOVA, QUEUE_IOVA),
        Err(VirtioRngQueueError::AliasedPages)
    );
    assert_eq!(
        VirtioRngQueueLayout::new(QUEUE_IOVA + 1, ENTROPY_IOVA),
        Err(VirtioRngQueueError::PageMisaligned(QUEUE_IOVA + 1))
    );
}

#[test]
fn request_publication_encodes_one_device_writable_descriptor() {
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut metadata = [0xa5; 4096];
    let mut entropy = [0xa5; 4096];
    let mut queue = VirtioRngQueueMemory::bind(&mut metadata, &mut entropy, layout);

    let request = queue.prepare_request(32).unwrap();

    assert_eq!(
        read_u64(queue.metadata(), VIRTIO_RNG_DESCRIPTOR_OFFSET),
        ENTROPY_IOVA
    );
    assert_eq!(
        read_u32(queue.metadata(), VIRTIO_RNG_DESCRIPTOR_OFFSET + 8),
        32
    );
    assert_eq!(
        read_u16(queue.metadata(), VIRTIO_RNG_DESCRIPTOR_OFFSET + 12),
        2
    );
    assert_eq!(
        read_u16(queue.metadata(), VIRTIO_RNG_AVAILABLE_OFFSET + 2),
        1
    );
    assert_eq!(
        read_u16(queue.metadata(), VIRTIO_RNG_AVAILABLE_OFFSET + 4),
        0
    );
    assert_eq!(&queue.entropy_page()[..32], &[0; 32]);
    assert_eq!(
        queue.prepare_request(16),
        Err(VirtioRngQueueError::RequestOutstanding)
    );
    assert_eq!(request.requested_len(), 32);
}

#[test]
fn completion_acquires_the_used_ring_and_bounds_entropy_length() {
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut metadata = [0; 4096];
    let mut entropy = [0; 4096];
    let metadata_ptr = metadata.as_mut_ptr();
    let entropy_ptr = entropy.as_mut_ptr();
    let mut queue = VirtioRngQueueMemory::bind(&mut metadata, &mut entropy, layout);
    let request = queue.prepare_request(16).unwrap();

    unsafe {
        for index in 0..16 {
            entropy_ptr.add(index).write((index as u8) ^ 0x5a);
        }
        write_u32(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 4, 0);
        write_u32(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 8, 16);
        write_u16(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 2, 1);
    }

    let completion = queue.complete_request(request).unwrap();
    assert_eq!(completion.len(), 16);
    assert!(!completion.is_empty());
    assert_eq!(
        queue.entropy(&completion),
        &[
            0x5a, 0x5b, 0x58, 0x59, 0x5e, 0x5f, 0x5c, 0x5d, 0x52, 0x53, 0x50, 0x51, 0x56, 0x57,
            0x54, 0x55
        ]
    );
    assert_eq!(
        queue.complete_request(request),
        Err(VirtioRngQueueError::NoRequestOutstanding)
    );
}

#[test]
fn completion_rejects_pending_unknown_or_oversized_used_elements() {
    let layout = VirtioRngQueueLayout::new(QUEUE_IOVA, ENTROPY_IOVA).unwrap();
    let mut metadata = [0; 4096];
    let mut entropy = [0; 4096];
    let metadata_ptr = metadata.as_mut_ptr();
    let mut queue = VirtioRngQueueMemory::bind(&mut metadata, &mut entropy, layout);
    let request = queue.prepare_request(8).unwrap();

    assert_eq!(
        queue.complete_request(request),
        Err(VirtioRngQueueError::CompletionPending)
    );
    unsafe {
        write_u32(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 4, 1);
        write_u32(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 8, 9);
        write_u16(metadata_ptr, VIRTIO_RNG_USED_OFFSET + 2, 1);
    }
    assert_eq!(
        queue.complete_request(request),
        Err(VirtioRngQueueError::UnexpectedDescriptor { id: 1 })
    );
}

fn read_u16(bytes: &[u8], offset: u16) -> u16 {
    u16::from_le_bytes([bytes[usize::from(offset)], bytes[usize::from(offset) + 1]])
}

fn read_u32(bytes: &[u8], offset: u16) -> u32 {
    let offset = usize::from(offset);
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: u16) -> u64 {
    let offset = usize::from(offset);
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

unsafe fn write_u16(base: *mut u8, offset: u16, value: u16) {
    let bytes = value.to_le_bytes();
    unsafe {
        base.add(usize::from(offset)).write(bytes[0]);
        base.add(usize::from(offset) + 1).write(bytes[1]);
    }
}

unsafe fn write_u32(base: *mut u8, offset: u16, value: u32) {
    let bytes = value.to_le_bytes();
    unsafe {
        for (index, byte) in bytes.iter().copied().enumerate() {
            base.add(usize::from(offset) + index).write(byte);
        }
    }
}
