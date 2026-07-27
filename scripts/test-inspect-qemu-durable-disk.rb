#!/usr/bin/env ruby
# frozen_string_literal: true

require "digest"
require "open3"
require "openssl"
require "rbconfig"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)
INSPECTOR = File.join(ROOT, "scripts/inspect-qemu-durable-disk.rb")
SLOT_BYTES = 64 * 1024
HEADER_BYTES = 64
FOOTER_BYTES = 64
MANIFEST_BYTES = 285
SIGNATURE_BYTES = 64
MANIFEST_DOMAIN = "AGENT-KERNEL-DURABLE-ARCHIVE\0".b
SIGNER_DOMAIN = "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".b
P256_ORDER = Integer("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551", 16)

def assert(condition, message)
  raise message unless condition
end

def run(*command)
  Open3.capture3(*command)
end

def p1363_signature(key, manifest)
  der = key.sign(OpenSSL::Digest::SHA256.new, manifest)
  sequence = OpenSSL::ASN1.decode(der)
  scalars = sequence.value.map { |integer| integer.value.to_i }
  scalars[1] = P256_ORDER - scalars[1] if scalars[1] > P256_ORDER / 2
  scalars.map do |scalar|
    value = scalar.to_s(16)
    value = "0#{value}" if value.length.odd?
    bytes = [value].pack("H*")
    raise "ECDSA scalar exceeds P-256 width" if bytes.bytesize > 32

    ("\0".b * (32 - bytes.bytesize)) + bytes
  end.join
end

def manifest_for(key, payload)
  compressed = key.public_key.to_octet_string(:compressed)
  signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + [2].pack("v") + compressed)
  payload_digest = Digest::SHA256.digest(payload)
  bytes = MANIFEST_DOMAIN.dup
  bytes << [2, 1, 2, 0].pack("v4")
  bytes << [1, 1, 64, 64].pack("Q<Q<Q<v")
  bytes << ("\0".b * 6)
  bytes << ("\0".b * 32)
  bytes << payload_digest
  bytes << [15, 36, 1, 12].pack("Q<4")
  bytes << [payload.bytesize, 0].pack("V2")
  bytes << payload_digest
  bytes << signer_id
  bytes << [1, 0].pack("Q<2")
  bytes << ("\0".b * 32)
  raise "manifest fixture has wrong size" unless bytes.bytesize == MANIFEST_BYTES

  bytes
end

def committed_slot(key, payload)
  manifest = manifest_for(key, payload)
  signature = p1363_signature(key, manifest)
  body_length = payload.bytesize + MANIFEST_BYTES + SIGNATURE_BYTES

  header = "AKDHDR13".b
  header << [1, 1, 0, 0].pack("v4")
  header << [1].pack("Q<")
  header << [body_length, payload.bytesize].pack("V2")
  header << [MANIFEST_BYTES, SIGNATURE_BYTES].pack("v2")
  header << ("\0".b * (HEADER_BYTES - header.bytesize))

  footer = "AKDCMT13".b
  footer << [1, 2].pack("v2")
  footer << [0].pack("C")
  footer << ("\0".b * 3)
  footer << [1].pack("Q<")
  footer << Digest::SHA256.digest(manifest)
  footer << ("\0".b * (FOOTER_BYTES - footer.bytesize))

  slot = header + payload + manifest + signature
  slot << ("\0".b * (SLOT_BYTES - FOOTER_BYTES - slot.bytesize))
  slot << footer
  raise "slot fixture has wrong size" unless slot.bytesize == SLOT_BYTES

  slot
end

assert(File.file?(INSPECTOR), "QEMU durable disk inspector is missing")

Dir.mktmpdir("agent-kernel-durable-disk-test") do |directory|
  key = OpenSSL::PKey::EC.generate("prime256v1")
  private_key = File.join(directory, "private.pem")
  public_key = File.join(directory, "public.pem")
  disk = File.join(directory, "durable.raw")
  File.binwrite(private_key, key.to_pem)
  output, error, status = run(
    "openssl", "pkey", "-in", private_key, "-pubout", "-out", public_key
  )
  assert(status.success?, "OpenSSL public-key export failed\n#{output}#{error}")

  payload = (0...4096).map { |index| (index * 17) & 0xff }.pack("C*")
  slot = committed_slot(key, payload)
  File.binwrite(disk, slot + ("\0".b * SLOT_BYTES))

  output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--disk", disk,
    "--public-key", public_key,
    "--storage", "12",
    "--expect-generation", "1",
    "--expect-through-sequence", "64"
  )
  assert(status.success?, "inspector rejected a committed disk\n#{output}#{error}")
  assert(output.include?("head_slot=A"), "selected slot evidence missing")
  assert(output.include?("generation=1"), "generation evidence missing")
  assert(output.include?("first_sequence=1"), "first sequence evidence missing")
  assert(output.include?("through_sequence=64"), "through sequence evidence missing")
  assert(output.include?("event_count=64"), "event count evidence missing")
  assert(output.include?("storage=12"), "storage evidence missing")
  assert(output.include?("signature=ecdsa-p256-sha256"), "signature evidence missing")

  corrupt_payload = File.binread(disk)
  corrupt_payload.setbyte(HEADER_BYTES + 7, corrupt_payload.getbyte(HEADER_BYTES + 7) ^ 0xff)
  File.binwrite(disk, corrupt_payload)
  _output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--disk", disk,
    "--public-key", public_key,
    "--storage", "12"
  )
  assert(!status.success?, "inspector accepted a corrupt payload")
  assert(error.include?("payload digest"), "wrong corrupt-payload failure")

  corrupt_signature = slot.dup
  signature_offset = HEADER_BYTES + payload.bytesize + MANIFEST_BYTES
  corrupt_signature.setbyte(
    signature_offset,
    corrupt_signature.getbyte(signature_offset) ^ 0x01
  )
  File.binwrite(disk, corrupt_signature + ("\0".b * SLOT_BYTES))
  _output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--disk", disk,
    "--public-key", public_key,
    "--storage", "12"
  )
  assert(!status.success?, "inspector accepted an invalid signature")
  assert(error.include?("signature"), "wrong invalid-signature failure\n#{error}")

  torn = slot.dup
  torn[-FOOTER_BYTES, FOOTER_BYTES] = "\0".b * FOOTER_BYTES
  File.binwrite(disk, torn + ("\0".b * SLOT_BYTES))
  _output, error, status = run(
    RbConfig.ruby, INSPECTOR,
    "--disk", disk,
    "--public-key", public_key,
    "--storage", "12"
  )
  assert(!status.success?, "inspector accepted a prepared-only slot as committed")
  assert(error.include?("no committed slot"), "wrong torn-write failure")
end

puts "[ OK ] QEMU durable ATA dual-slot inspector"
