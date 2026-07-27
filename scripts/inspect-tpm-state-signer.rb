#!/usr/bin/env ruby
# frozen_string_literal: true

# Validates public TPM provisioning artifacts and renders immutable V20 fields.

require "digest"
require "openssl"
require "optparse"

SIGNER_DOMAIN = "AGENT-KERNEL-DURABLE-STATE-SIGNER-V2\0".b
COMMANDS = {
  "sign-digest-v185" => "SignDigestV185",
  "sign-v184" => "SignV184"
}.freeze
COMMAND_CODES = {
  "sign-digest-v185" => 0x0000_01a6,
  "sign-v184" => 0x0000_015d
}.freeze
AUTHORIZATIONS = %w[empty-password pcr-policy].freeze
TPM_CC_POLICY_PCR = 0x0000_017f
TPM_CC_POLICY_COMMAND_CODE = 0x0000_016c
TPM_ALG_SHA256 = 0x000b
HANDLE_RANGE = (0x8100_0000..0x81ff_ffff)
U64_MAX = (1 << 64) - 1

def fail_with(message)
  warn "inspect TPM State Signer failed: #{message}"
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

def parse_hex(name, raw, bytes)
  fail_with("#{name} must contain #{bytes * 2} hexadecimal digits") unless
    raw&.match?(/\A[0-9a-fA-F]{#{bytes * 2}}\z/)
  [raw].pack("H*")
end

def read_name(path)
  bytes = File.binread(path)
  if bytes.bytesize == 36 && bytes.byteslice(0, 2).unpack1("n") == 34
    bytes = bytes.byteslice(2, 34)
  end
  fail_with("Name must contain 34 bytes or one canonical TPM2B_NAME") unless bytes.bytesize == 34
  fail_with("Name must use TPM_ALG_SHA256") unless bytes.byteslice(0, 2) == "\x00\x0b".b
  bytes
end

def read_public_key(path)
  key = OpenSSL::PKey.read(File.binread(path))
  fail_with("public key must be an EC key") unless key.is_a?(OpenSSL::PKey::EC)
  fail_with("public key input contains private material") if key.private?
  fail_with("public key must use NIST P-256") unless key.group.curve_name == "prime256v1"
  compressed = key.public_key.to_octet_string(:compressed)
  fail_with("compressed public key must contain 33 bytes") unless
    compressed.bytesize == 33 && [2, 3].include?(compressed.getbyte(0))
  compressed
rescue OpenSSL::PKey::PKeyError, OpenSSL::PKey::ECError => error
  fail_with("unable to parse public key: #{error.message}")
end

options = {}
OptionParser.new do |parser|
  parser.banner = "usage: scripts/inspect-tpm-state-signer.rb [options]"
  parser.on("--handle VALUE", "persistent TPM handle") { |value| options[:handle] = value }
  parser.on(
    "--command NAME",
    "sign-digest-v185 or sign-v184"
  ) { |value| options[:command] = value }
  parser.on(
    "--policy-generation VALUE",
    "nonzero durable policy generation"
  ) { |value| options[:policy_generation] = value }
  parser.on(
    "--authorization MODE",
    "empty-password or pcr-policy"
  ) { |value| options[:authorization] = value }
  parser.on(
    "--pcr-selection HEX",
    "three-byte SHA-256 PCR bitmap"
  ) { |value| options[:pcr_selection] = value }
  parser.on(
    "--pcr-digest HEX",
    "expected SHA-256 PCR composite digest"
  ) { |value| options[:pcr_digest] = value }
  parser.on("--name PATH", "TPM2_ReadPublic Name output") { |value| options[:name] = value }
  parser.on(
    "--public-key PATH",
    "PEM public key from TPM2_ReadPublic"
  ) { |value| options[:public_key] = value }
  parser.on_tail("-h", "--help", "show this help") do
    puts parser
    exit
  end
end.parse!

%i[handle command policy_generation name public_key].each do |name|
  fail_with("--#{name.to_s.tr("_", "-")} is required") unless options[name]
end
fail_with("unexpected positional arguments") unless ARGV.empty?

handle = parse_integer("handle", options[:handle], HANDLE_RANGE)
generation = parse_integer("policy generation", options[:policy_generation], 1..U64_MAX)
command_key = options[:command]
command = COMMANDS[command_key]
fail_with("command must be sign-digest-v185 or sign-v184") unless command
authorization = options.fetch(:authorization, "empty-password")
fail_with("authorization must be empty-password or pcr-policy") unless
  AUTHORIZATIONS.include?(authorization)
name = read_name(input_path("Name", options[:name]))
public_key = read_public_key(input_path("public key", options[:public_key]))
signer_id = Digest::SHA256.digest(SIGNER_DOMAIN + [2].pack("v") + public_key)

puts "profile=NativeTpmSignerProfile::Crb"
puts format("persistent_handle=0x%08x", handle)
puts "digest_sign_command=#{command}"
puts "policy_generation=#{generation}"
puts "expected_name=#{name.unpack1("H*")}"
puts "expected_public_key=#{public_key.unpack1("H*")}"
puts "state_signer_id=#{signer_id.unpack1("H*")}"

if authorization == "empty-password"
  fail_with("--pcr-selection requires --authorization pcr-policy") if options[:pcr_selection]
  fail_with("--pcr-digest requires --authorization pcr-policy") if options[:pcr_digest]
  puts "authorization=EmptyPassword"
else
  selection = parse_hex("PCR selection", options[:pcr_selection], 3)
  fail_with("PCR selection must contain at least one PCR") if selection == ("\0".b * 3)
  pcr_digest = parse_hex("PCR digest", options[:pcr_digest], 32)
  marshalled_selection =
    [1].pack("N") + [TPM_ALG_SHA256].pack("n") + [selection.bytesize].pack("C") + selection
  pcr_policy = Digest::SHA256.digest(
    ("\0".b * 32) + [TPM_CC_POLICY_PCR].pack("N") + marshalled_selection + pcr_digest
  )
  auth_policy = Digest::SHA256.digest(
    pcr_policy +
      [TPM_CC_POLICY_COMMAND_CODE].pack("N") +
      [COMMAND_CODES.fetch(command_key)].pack("N")
  )
  puts "authorization=PcrPolicy"
  puts "pcr_bank=Sha256"
  puts "pcr_selection=#{selection.unpack1("H*")}"
  puts "expected_pcr_digest=#{pcr_digest.unpack1("H*")}"
  puts "expected_auth_policy=#{auth_policy.unpack1("H*")}"
end
