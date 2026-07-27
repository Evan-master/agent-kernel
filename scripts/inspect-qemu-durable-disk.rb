#!/usr/bin/env ruby
# frozen_string_literal: true

# Validates the two fixed native durable slots in a raw QEMU ATA image.

require "digest"
require "openssl"
require "optparse"

SLOT_BYTES = 64 * 1024
HEADER_BYTES = 64
FOOTER_BYTES = 64
BODY_BYTES = SLOT_BYTES - HEADER_BYTES - FOOTER_BYTES
SECTOR_BYTES = 512
MANIFEST_BYTES = 285
SIGNATURE_BYTES = 64
MAX_EVENTS = 64
MAX_PAYLOAD_BYTES = 64 * 1024 - 512
HEADER_MAGIC = "AKDHDR13".b
FOOTER_MAGIC = "AKDCMT13".b
MANIFEST_DOMAIN = "AGENT-KERNEL-DURABLE-ARCHIVE\0".b
SIGNER_DOMAIN = "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".b
ZERO_DIGEST = "\0".b * 32
P256_ORDER = Integer("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551", 16)
U64_MAX = (1 << 64) - 1

def fail_with(message)
  warn "inspect QEMU durable disk failed: #{message}"
  exit 1
end

def parse_integer(name, raw, range)
  value = Integer(raw, 0)
  fail_with("#{name} is outside #{range}") unless range.cover?(value)
  value
rescue ArgumentError
  fail_with("#{name} must be an integer")
end

def input_path(name, raw)
  path = File.expand_path(raw)
  fail_with("#{name} does not exist: #{path}") unless File.file?(path)
  File.realpath(path)
end

def bytes_at(bytes, offset, length, field)
  value = bytes.byteslice(offset, length)
  fail_with("#{field} is truncated") unless value&.bytesize == length
  value
end

def u16(bytes, offset, field)
  bytes_at(bytes, offset, 2, field).unpack1("v")
end

def u32(bytes, offset, field)
  bytes_at(bytes, offset, 4, field).unpack1("V")
end

def u64(bytes, offset, field)
  bytes_at(bytes, offset, 8, field).unpack1("Q<")
end

def require_zero(bytes, field)
  fail_with("#{field} must be zero") unless bytes.each_byte.all?(&:zero?)
end

def public_key(path)
  key = OpenSSL::PKey.read(File.binread(path))
  fail_with("public key must be an EC key") unless key.is_a?(OpenSSL::PKey::EC)
  fail_with("public key input contains private material") if key.private?
  fail_with("public key must use NIST P-256") unless key.group.curve_name == "prime256v1"
  compressed = key.public_key.to_octet_string(:compressed)
  fail_with("public key has a noncanonical compressed point") unless
    compressed.bytesize == 33 && [2, 3].include?(compressed.getbyte(0))
  [key, compressed]
rescue OpenSSL::PKey::PKeyError, OpenSSL::PKey::ECError => error
  fail_with("unable to parse public key: #{error.message}")
end

def signature_der(signature)
  fail_with("signature must contain 64 bytes") unless signature.bytesize == SIGNATURE_BYTES
  r = signature.byteslice(0, 32).unpack1("H*").to_i(16)
  s = signature.byteslice(32, 32).unpack1("H*").to_i(16)
  fail_with("signature scalar is outside P-256") unless
    (1...P256_ORDER).cover?(r) && (1...P256_ORDER).cover?(s)
  fail_with("signature is not low-S canonical") if s > P256_ORDER / 2
  OpenSSL::ASN1::Sequence([
    OpenSSL::ASN1::Integer(r),
    OpenSSL::ASN1::Integer(s)
  ]).to_der
end

def parse_manifest(bytes, expected_storage, expected_signer_id)
  fail_with("manifest length mismatch") unless bytes.bytesize == MANIFEST_BYTES
  fail_with("manifest domain mismatch") unless bytes.start_with?(MANIFEST_DOMAIN)

  version = u16(bytes, 29, "manifest version")
  flags = u16(bytes, 31, "manifest flags")
  algorithm = u16(bytes, 33, "manifest signature algorithm")
  fail_with("manifest version must be algorithm-bound v2") unless version == 2
  fail_with("manifest flags are unsupported") unless [0, 1].include?(flags)
  fail_with("manifest signature algorithm must be ECDSA P-256 SHA-256") unless algorithm == 2
  require_zero(bytes_at(bytes, 35, 2, "manifest algorithm reserved"), "manifest algorithm reserved")

  generation = u64(bytes, 37, "manifest generation")
  first_sequence = u64(bytes, 45, "manifest first sequence")
  through_sequence = u64(bytes, 53, "manifest through sequence")
  event_count = u16(bytes, 61, "manifest event count")
  require_zero(bytes_at(bytes, 63, 6, "manifest sequence reserved"), "manifest sequence reserved")
  previous_digest = bytes_at(bytes, 69, 32, "manifest previous digest")
  archive_digest = bytes_at(bytes, 101, 32, "manifest archive digest")
  actor = u64(bytes, 133, "manifest actor")
  archive_authority = u64(bytes, 141, "manifest archive authority")
  root = u64(bytes, 149, "manifest root")
  storage = u64(bytes, 157, "manifest storage")
  payload_length = u32(bytes, 165, "manifest payload length")
  require_zero(bytes_at(bytes, 169, 4, "manifest payload reserved"), "manifest payload reserved")
  payload_digest = bytes_at(bytes, 173, 32, "manifest payload digest")
  signer_id = bytes_at(bytes, 205, 32, "manifest signer ID")
  policy_generation = u64(bytes, 237, "manifest policy generation")
  anchor_generation = u64(bytes, 245, "manifest anchor generation")
  anchor_digest = bytes_at(bytes, 253, 32, "manifest anchor digest")

  fail_with("manifest generation must be nonzero") if generation.zero?
  fail_with("manifest event count is outside 1..#{MAX_EVENTS}") unless
    (1..MAX_EVENTS).cover?(event_count)
  expected_through = first_sequence + event_count - 1
  fail_with("manifest sequence range mismatch") unless
    first_sequence.positive? && expected_through <= U64_MAX && through_sequence == expected_through
  if generation == 1
    fail_with("manifest genesis sequence mismatch") unless first_sequence == 1
    fail_with("manifest genesis previous digest must be zero") unless previous_digest == ZERO_DIGEST
  end
  fail_with("manifest actor must be nonzero") if actor.zero?
  fail_with("manifest archive authority must be nonzero") if archive_authority.zero?
  fail_with("manifest root must be nonzero") if root.zero?
  fail_with("manifest storage mismatch") unless storage == expected_storage
  fail_with("manifest payload length is outside bounds") unless
    (1..MAX_PAYLOAD_BYTES).cover?(payload_length)
  fail_with("manifest payload digest and archive digest differ") unless
    payload_digest == archive_digest
  fail_with("manifest signer ID mismatch") unless signer_id == expected_signer_id
  fail_with("manifest signer policy generation must be nonzero") if policy_generation.zero?

  if flags.zero?
    fail_with("unanchored manifest carries anchor state") unless
      anchor_generation.zero? && anchor_digest == ZERO_DIGEST
  else
    anchor_shape_valid = anchor_generation.zero? == (anchor_digest == ZERO_DIGEST)
    fail_with("trusted anchor encoding is invalid") unless anchor_shape_valid
    fail_with("trusted anchor generation mismatch") unless
      anchor_generation < U64_MAX && anchor_generation + 1 == generation
    fail_with("trusted anchor digest mismatch") unless anchor_digest == previous_digest
  end

  {
    bytes: bytes,
    generation: generation,
    first_sequence: first_sequence,
    through_sequence: through_sequence,
    event_count: event_count,
    previous_digest: previous_digest,
    archive_digest: archive_digest,
    actor: actor,
    archive_authority: archive_authority,
    root: root,
    storage: storage,
    payload_length: payload_length,
    payload_digest: payload_digest,
    signer_id: signer_id,
    policy_generation: policy_generation,
    anchor_trusted: flags == 1
  }
end

def parse_slot(bytes, physical_slot, expected_storage, key, expected_signer_id)
  return { state: :empty, slot: physical_slot } if bytes.each_byte.all?(&:zero?)

  fail_with("slot #{physical_slot} header magic mismatch") unless
    bytes_at(bytes, 0, 8, "slot #{physical_slot} header magic") == HEADER_MAGIC
  fail_with("slot #{physical_slot} header version unsupported") unless
    u16(bytes, 8, "slot #{physical_slot} header version") == 1
  fail_with("slot #{physical_slot} header state is not prepared") unless
    u16(bytes, 10, "slot #{physical_slot} header state") == 1
  require_zero(bytes_at(bytes, 12, 2, "slot #{physical_slot} header flags"), "slot #{physical_slot} header flags")
  encoded_slot = bytes.getbyte(14)
  expected_slot = physical_slot == "A" ? 0 : 1
  fail_with("slot #{physical_slot} header slot mismatch") unless encoded_slot == expected_slot
  fail_with("slot #{physical_slot} header reserved byte must be zero") unless bytes.getbyte(15).zero?
  require_zero(bytes_at(bytes, 36, 28, "slot #{physical_slot} header reserved"), "slot #{physical_slot} header reserved")

  generation = u64(bytes, 16, "slot #{physical_slot} generation")
  fail_with("slot #{physical_slot} generation must be nonzero") if generation.zero?
  fail_with("slot #{physical_slot} generation parity mismatch") unless
    (generation.odd? ? 0 : 1) == expected_slot
  body_length = u32(bytes, 24, "slot #{physical_slot} body length")
  payload_length = u32(bytes, 28, "slot #{physical_slot} payload length")
  fail_with("slot #{physical_slot} manifest length mismatch") unless
    u16(bytes, 32, "slot #{physical_slot} manifest length") == MANIFEST_BYTES
  fail_with("slot #{physical_slot} signature length mismatch") unless
    u16(bytes, 34, "slot #{physical_slot} signature length") == SIGNATURE_BYTES
  expected_body = payload_length + MANIFEST_BYTES + SIGNATURE_BYTES
  fail_with("slot #{physical_slot} body length mismatch") unless
    body_length == expected_body && body_length <= BODY_BYTES

  payload = bytes_at(bytes, HEADER_BYTES, payload_length, "slot #{physical_slot} payload")
  manifest_offset = HEADER_BYTES + payload_length
  manifest_bytes = bytes_at(bytes, manifest_offset, MANIFEST_BYTES, "slot #{physical_slot} manifest")
  signature_offset = manifest_offset + MANIFEST_BYTES
  signature = bytes_at(bytes, signature_offset, SIGNATURE_BYTES, "slot #{physical_slot} signature")
  padding_offset = HEADER_BYTES + body_length
  padding_length = SLOT_BYTES - FOOTER_BYTES - padding_offset
  require_zero(
    bytes_at(bytes, padding_offset, padding_length, "slot #{physical_slot} body padding"),
    "slot #{physical_slot} body padding"
  )

  manifest = parse_manifest(manifest_bytes, expected_storage, expected_signer_id)
  fail_with("slot #{physical_slot} manifest generation mismatch") unless
    manifest[:generation] == generation
  fail_with("slot #{physical_slot} manifest payload length mismatch") unless
    manifest[:payload_length] == payload_length
  fail_with("slot #{physical_slot} payload digest mismatch") unless
    Digest::SHA256.digest(payload) == manifest[:payload_digest]
  verified = begin
    key.verify(OpenSSL::Digest::SHA256.new, signature_der(signature), manifest_bytes)
  rescue OpenSSL::PKey::PKeyError
    false
  end
  fail_with("slot #{physical_slot} signature verification failed") unless verified

  footer = bytes_at(bytes, SLOT_BYTES - FOOTER_BYTES, FOOTER_BYTES, "slot #{physical_slot} footer")
  return manifest.merge(state: :prepared, slot: physical_slot) if footer.each_byte.all?(&:zero?)

  fail_with("slot #{physical_slot} footer magic mismatch") unless footer.byteslice(0, 8) == FOOTER_MAGIC
  fail_with("slot #{physical_slot} footer version unsupported") unless u16(footer, 8, "footer version") == 1
  fail_with("slot #{physical_slot} footer state is not committed") unless u16(footer, 10, "footer state") == 2
  fail_with("slot #{physical_slot} footer slot mismatch") unless footer.getbyte(12) == expected_slot
  require_zero(footer.byteslice(13, 3), "slot #{physical_slot} footer reserved")
  fail_with("slot #{physical_slot} footer generation mismatch") unless
    u64(footer, 16, "footer generation") == generation
  fail_with("slot #{physical_slot} footer manifest digest mismatch") unless
    footer.byteslice(24, 32) == Digest::SHA256.digest(manifest_bytes)
  require_zero(footer.byteslice(56, 8), "slot #{physical_slot} footer reserved")
  manifest.merge(state: :committed, slot: physical_slot)
end

def select_head(slots)
  committed = slots.select { |slot| slot[:state] == :committed }
  fail_with("no committed slot") if committed.empty?
  return committed.first if committed.length == 1 && committed.first[:generation] == 1
  if committed.length == 1
    head = committed.first
    return head if head[:anchor_trusted]
    fail_with("disconnected committed head generation #{head[:generation]}")
  end

  first, second = committed.sort_by { |slot| slot[:generation] }
  fail_with("split-brain committed generation #{first[:generation]}") if
    first[:generation] == second[:generation]
  fail_with("durable generation exhausted") if second[:generation] == U64_MAX
  if first[:generation] + 1 == second[:generation]
    linked = first[:through_sequence] + 1 == second[:first_sequence] &&
      first[:archive_digest] == second[:previous_digest]
    return second if linked
    fail_with("trusted anchor mismatch at generation #{second[:generation]}") if
      second[:anchor_trusted]
    fail_with("disconnected committed head generation #{second[:generation]}")
  end
  return second if second[:anchor_trusted]

  fail_with("disconnected committed head generation #{second[:generation]}")
end

options = { base_lba: "0" }
OptionParser.new do |parser|
  parser.banner = "usage: scripts/inspect-qemu-durable-disk.rb [options]"
  parser.on("--disk PATH", "raw ATA durable image") { |value| options[:disk] = value }
  parser.on("--public-key PATH", "provisioned P-256 public key") do |value|
    options[:public_key] = value
  end
  parser.on("--storage VALUE", "expected storage Resource ID") { |value| options[:storage] = value }
  parser.on("--base-lba VALUE", "first durable slot LBA") { |value| options[:base_lba] = value }
  parser.on("--expect-generation VALUE", "required selected generation") do |value|
    options[:expect_generation] = value
  end
  parser.on("--expect-through-sequence VALUE", "required selected terminal Event") do |value|
    options[:expect_through] = value
  end
  parser.on_tail("-h", "--help", "show this help") do
    puts parser
    exit
  end
end.parse!

%i[disk public_key storage].each do |name|
  fail_with("--#{name.to_s.tr("_", "-")} is required") unless options[name]
end
fail_with("unexpected positional arguments") unless ARGV.empty?

disk_path = input_path("disk", options[:disk])
public_key_path = input_path("public key", options[:public_key])
storage = parse_integer("storage", options[:storage], 1..U64_MAX)
base_lba = parse_integer("base LBA", options[:base_lba], 0..U64_MAX)
expect_generation = if options[:expect_generation]
                      parse_integer("expected generation", options[:expect_generation], 1..U64_MAX)
                    end
expect_through = if options[:expect_through]
                   parse_integer("expected through sequence", options[:expect_through], 1..U64_MAX)
                 end
key, compressed = public_key(public_key_path)
expected_signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + [2].pack("v") + compressed)

disk = File.binread(disk_path)
slot_offset = base_lba * SECTOR_BYTES
required_length = slot_offset + SLOT_BYTES * 2
fail_with("disk is shorter than the configured two-slot range") if disk.bytesize < required_length
slot_a = parse_slot(
  disk.byteslice(slot_offset, SLOT_BYTES),
  "A",
  storage,
  key,
  expected_signer_id
)
slot_b = parse_slot(
  disk.byteslice(slot_offset + SLOT_BYTES, SLOT_BYTES),
  "B",
  storage,
  key,
  expected_signer_id
)
head = select_head([slot_a, slot_b])

fail_with("selected generation #{head[:generation]} does not match #{expect_generation}") if
  expect_generation && head[:generation] != expect_generation
fail_with("selected through sequence #{head[:through_sequence]} does not match #{expect_through}") if
  expect_through && head[:through_sequence] != expect_through

puts "profile=qemu-ata-durable-v26"
puts "head_slot=#{head[:slot]}"
puts "generation=#{head[:generation]}"
puts "first_sequence=#{head[:first_sequence]}"
puts "through_sequence=#{head[:through_sequence]}"
puts "event_count=#{head[:event_count]}"
puts "storage=#{head[:storage]}"
puts "actor=#{head[:actor]}"
puts "archive_authority=#{head[:archive_authority]}"
puts "policy_generation=#{head[:policy_generation]}"
puts "signature=ecdsa-p256-sha256"
puts "signer_id=#{head[:signer_id].unpack1("H*")}"
puts "archive_digest=#{head[:archive_digest].unpack1("H*")}"
puts "manifest_sha256=#{Digest::SHA256.hexdigest(head[:bytes])}"
puts "disk_sha256=#{Digest::SHA256.hexdigest(disk)}"
