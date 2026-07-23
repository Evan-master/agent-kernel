#[allow(dead_code)]
mod durable_state_support;

use agent_kernel_core::{
    DurableArchiveAnchor, DurableArchiveManifestError, DurableSlot, DurableStateDigest,
};
use agent_kernel_hal::{
    DURABLE_SLOT_BODY_BYTES, DURABLE_SLOT_BYTES, DURABLE_SLOT_FOOTER_BYTES,
    DURABLE_SLOT_HEADER_BYTES,
};
use agent_kernel_x86_64::durable_state::{
    decode_durable_archive_manifest, durable_archive_manifest_digest, encode_durable_archive_body,
    encode_durable_archive_commit_footer, encode_durable_archive_manifest,
    encode_durable_archive_prepared_header, parse_durable_archive_slot, DecodedDurableArchiveSlot,
    DurableArchiveCapsuleError, DurableArchiveManifestDecodeError,
};

use durable_state_support::{
    payload_and_manifest, signature, signing_key, POLICY_GENERATION, ROOT, STORAGE,
};

#[test]
fn canonical_manifest_bytes_decode_through_core_validation() {
    let signing_key = signing_key(0x31);
    let (_, manifest) = payload_and_manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let encoded = encode_durable_archive_manifest(manifest);

    assert_eq!(decode_durable_archive_manifest(&encoded), Ok(manifest));

    let mut unknown_flags = encoded;
    unknown_flags[31..33].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        decode_durable_archive_manifest(&unknown_flags),
        Err(DurableArchiveManifestDecodeError::UnsupportedFlags { flags: 2 })
    );

    let mut nonzero_reserved = encoded;
    nonzero_reserved[33] = 1;
    assert_eq!(
        decode_durable_archive_manifest(&nonzero_reserved),
        Err(DurableArchiveManifestDecodeError::ReservedNotZero)
    );

    let mut broken_range = encoded;
    broken_range[53..61].copy_from_slice(&2u64.to_le_bytes());
    assert_eq!(
        decode_durable_archive_manifest(&broken_range),
        Err(DurableArchiveManifestDecodeError::Manifest(
            DurableArchiveManifestError::SequenceRangeMismatch
        ))
    );
}

#[test]
fn prepared_and_committed_slots_round_trip_one_canonical_capsule() {
    let signing_key = signing_key(0x32);
    let (payload, manifest) = payload_and_manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&signing_key, manifest);
    let mut slot = vec![0; DURABLE_SLOT_BYTES];
    let header = encode_durable_archive_prepared_header(manifest);
    let body_length = encode_durable_archive_body(
        &payload,
        manifest,
        signature,
        &mut slot[DURABLE_SLOT_HEADER_BYTES..DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES],
    )
    .unwrap();
    slot[..DURABLE_SLOT_HEADER_BYTES].copy_from_slice(&header);

    assert_eq!(&header[..8], b"AKDHDR13");
    assert_eq!(&header[8..10], &1u16.to_le_bytes());
    assert_eq!(&header[10..12], &1u16.to_le_bytes());
    assert_eq!(&header[12..14], &0u16.to_le_bytes());
    assert_eq!(header[14], DurableSlot::A as u8);
    assert_eq!(header[15], 0);
    assert_eq!(&header[16..24], &manifest.generation().to_le_bytes());
    assert_eq!(&header[24..28], &(body_length as u32).to_le_bytes());
    assert_eq!(&header[28..32], &(payload.len() as u32).to_le_bytes());
    assert!(header[36..].iter().all(|byte| *byte == 0));

    let prepared = parse_durable_archive_slot(&slot, STORAGE, DurableSlot::A).unwrap();
    let DecodedDurableArchiveSlot::Prepared(capsule) = prepared else {
        panic!("expected prepared capsule");
    };
    assert_eq!(capsule.payload(), payload);
    assert_eq!(capsule.manifest(), manifest);
    assert_eq!(capsule.signature(), signature);
    assert_eq!(
        capsule.manifest_digest(),
        durable_archive_manifest_digest(manifest)
    );

    let footer = encode_durable_archive_commit_footer(manifest);
    slot[DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES..].copy_from_slice(&footer);
    assert_eq!(&footer[..8], b"AKDCMT13");
    assert_eq!(&footer[10..12], &2u16.to_le_bytes());
    assert_eq!(footer[12], DurableSlot::A as u8);
    assert_eq!(&footer[16..24], &manifest.generation().to_le_bytes());
    assert_eq!(
        &footer[24..56],
        &durable_archive_manifest_digest(manifest).bytes()
    );

    let committed = parse_durable_archive_slot(&slot, STORAGE, DurableSlot::A).unwrap();
    let DecodedDurableArchiveSlot::Committed(capsule) = committed else {
        panic!("expected committed capsule");
    };
    assert_eq!(capsule.payload(), payload);
    assert_eq!(capsule.manifest(), manifest);
}

#[test]
fn empty_and_corrupted_slots_have_precise_results() {
    let empty = vec![0; DURABLE_SLOT_BYTES];
    assert!(matches!(
        parse_durable_archive_slot(&empty, STORAGE, DurableSlot::A),
        Ok(DecodedDurableArchiveSlot::Empty)
    ));

    let signing_key = signing_key(0x33);
    let (payload, manifest) = payload_and_manifest(
        &signing_key,
        ROOT,
        STORAGE,
        POLICY_GENERATION,
        DurableArchiveAnchor::unanchored(),
    );
    let signature = signature(&signing_key, manifest);
    let mut slot = vec![0; DURABLE_SLOT_BYTES];
    slot[..DURABLE_SLOT_HEADER_BYTES]
        .copy_from_slice(&encode_durable_archive_prepared_header(manifest));
    let body_length = encode_durable_archive_body(
        &payload,
        manifest,
        signature,
        &mut slot[DURABLE_SLOT_HEADER_BYTES..DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES],
    )
    .unwrap();

    assert_eq!(
        parse_durable_archive_slot(&slot, STORAGE, DurableSlot::B),
        Err(DurableArchiveCapsuleError::PhysicalSlotMismatch {
            encoded: DurableSlot::A,
            physical: DurableSlot::B,
        })
    );

    let mut bad_reserved = slot.clone();
    bad_reserved[36] = 1;
    assert_eq!(
        parse_durable_archive_slot(&bad_reserved, STORAGE, DurableSlot::A),
        Err(DurableArchiveCapsuleError::HeaderReservedNotZero)
    );

    let mut bad_payload = slot.clone();
    bad_payload[DURABLE_SLOT_HEADER_BYTES] ^= 0x80;
    assert_eq!(
        parse_durable_archive_slot(&bad_payload, STORAGE, DurableSlot::A),
        Err(DurableArchiveCapsuleError::PayloadDigestMismatch)
    );

    let mut bad_padding = slot.clone();
    bad_padding[DURABLE_SLOT_HEADER_BYTES + body_length] = 1;
    assert_eq!(
        parse_durable_archive_slot(&bad_padding, STORAGE, DurableSlot::A),
        Err(DurableArchiveCapsuleError::BodyPaddingNotZero)
    );

    let mut bad_footer = slot;
    let mut footer = encode_durable_archive_commit_footer(manifest);
    footer[24..56].copy_from_slice(&DurableStateDigest::new([0x44; 32]).bytes());
    bad_footer[DURABLE_SLOT_BYTES - DURABLE_SLOT_FOOTER_BYTES..].copy_from_slice(&footer);
    assert_eq!(
        parse_durable_archive_slot(&bad_footer, STORAGE, DurableSlot::A),
        Err(DurableArchiveCapsuleError::FooterManifestDigestMismatch)
    );

    assert!(body_length <= DURABLE_SLOT_BODY_BYTES);
}
