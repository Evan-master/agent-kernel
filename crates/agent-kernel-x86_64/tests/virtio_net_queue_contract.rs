use agent_kernel_x86_64::virtio_net::{
    VirtioNetQueueError, VirtioNetQueueLayout, VirtioNetRxQueue, VirtioNetTxQueue,
    VIRTIO_NET_AVAILABLE_OFFSET, VIRTIO_NET_DESCRIPTOR_OFFSET, VIRTIO_NET_HEADER_BYTES,
    VIRTIO_NET_RX_BUFFER_BYTES, VIRTIO_NET_USED_OFFSET,
};

const RX_METADATA_IOVA: u64 = 0x0200_0000;
const RX_PACKET_IOVA: u64 = 0x0200_1000;
const TX_METADATA_IOVA: u64 = 0x0200_2000;
const TX_PACKET_IOVA: u64 = 0x0200_3000;

#[test]
fn queue_layouts_require_disjoint_aligned_pages() {
    let layout = VirtioNetQueueLayout::new(RX_METADATA_IOVA, RX_PACKET_IOVA).unwrap();
    assert_eq!(
        layout.descriptor_iova(),
        RX_METADATA_IOVA + u64::from(VIRTIO_NET_DESCRIPTOR_OFFSET)
    );
    assert_eq!(
        layout.driver_iova(),
        RX_METADATA_IOVA + u64::from(VIRTIO_NET_AVAILABLE_OFFSET)
    );
    assert_eq!(
        layout.device_iova(),
        RX_METADATA_IOVA + u64::from(VIRTIO_NET_USED_OFFSET)
    );
    assert_eq!(layout.packet_iova(), RX_PACKET_IOVA);
    assert_eq!(
        VirtioNetQueueLayout::new(RX_METADATA_IOVA, RX_METADATA_IOVA),
        Err(VirtioNetQueueError::AliasedPages)
    );
}

#[test]
fn receive_queue_posts_one_device_writable_buffer_and_validates_ethernet() {
    let layout = VirtioNetQueueLayout::new(RX_METADATA_IOVA, RX_PACKET_IOVA).unwrap();
    let mut metadata = [0xa5; 4096];
    let mut packet = [0xa5; 4096];
    let metadata_ptr = metadata.as_mut_ptr();
    let packet_ptr = packet.as_mut_ptr();
    let mut queue = VirtioNetRxQueue::bind(&mut metadata, &mut packet, layout);

    let request = queue.post_buffer().unwrap();
    assert_eq!(
        read_u64(queue.metadata(), VIRTIO_NET_DESCRIPTOR_OFFSET),
        RX_PACKET_IOVA
    );
    assert_eq!(
        read_u32(queue.metadata(), VIRTIO_NET_DESCRIPTOR_OFFSET + 8),
        VIRTIO_NET_RX_BUFFER_BYTES as u32
    );
    assert_eq!(
        read_u16(queue.metadata(), VIRTIO_NET_DESCRIPTOR_OFFSET + 12),
        2
    );
    assert_eq!(
        read_u16(queue.metadata(), VIRTIO_NET_AVAILABLE_OFFSET + 2),
        1
    );

    unsafe {
        packet_ptr.add(10).write(1);
        packet_ptr.add(11).write(0);
        let frame = packet_ptr.add(VIRTIO_NET_HEADER_BYTES);
        for index in 0..60 {
            frame.add(index).write(index as u8);
        }
        frame.add(12).write(0x08);
        frame.add(13).write(0x06);
        write_u32(metadata_ptr, VIRTIO_NET_USED_OFFSET + 4, 0);
        write_u32(
            metadata_ptr,
            VIRTIO_NET_USED_OFFSET + 8,
            (VIRTIO_NET_HEADER_BYTES + 60) as u32,
        );
        write_u16(metadata_ptr, VIRTIO_NET_USED_OFFSET + 2, 1);
    }

    let completion = queue.complete_buffer(request).unwrap();
    assert_eq!(completion.frame_len(), 60);
    assert_eq!(completion.ether_type(), 0x0806);
    assert_eq!(queue.frame(&completion).unwrap()[12..14], [0x08, 0x06]);

    queue.post_buffer().unwrap();
    assert_eq!(
        queue.frame(&completion),
        Err(VirtioNetQueueError::StaleCompletion)
    );
}

#[test]
fn transmit_queue_prepends_a_zeroed_modern_header_and_bounds_completion() {
    let layout = VirtioNetQueueLayout::new(TX_METADATA_IOVA, TX_PACKET_IOVA).unwrap();
    let mut metadata = [0; 4096];
    let mut packet = [0; 4096];
    let metadata_ptr = metadata.as_mut_ptr();
    let mut queue = VirtioNetTxQueue::bind(&mut metadata, &mut packet, layout);
    let mut frame = [0x5a; 60];
    frame[12..14].copy_from_slice(&0x0806_u16.to_be_bytes());

    let request = queue.prepare_frame(&frame).unwrap();
    assert_eq!(
        &queue.packet()[..VIRTIO_NET_HEADER_BYTES],
        &[0; VIRTIO_NET_HEADER_BYTES]
    );
    assert_eq!(
        &queue.packet()[VIRTIO_NET_HEADER_BYTES..VIRTIO_NET_HEADER_BYTES + frame.len()],
        &frame
    );
    assert_eq!(
        read_u32(queue.metadata(), VIRTIO_NET_DESCRIPTOR_OFFSET + 8),
        (VIRTIO_NET_HEADER_BYTES + frame.len()) as u32
    );
    assert_eq!(
        queue.prepare_frame(&frame),
        Err(VirtioNetQueueError::RequestOutstanding)
    );

    unsafe {
        write_u32(metadata_ptr, VIRTIO_NET_USED_OFFSET + 4, 0);
        write_u32(metadata_ptr, VIRTIO_NET_USED_OFFSET + 8, 0);
        write_u16(metadata_ptr, VIRTIO_NET_USED_OFFSET + 2, 1);
    }
    let completion = queue.complete_frame(request).unwrap();
    assert_eq!(completion.frame_len(), 60);
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
