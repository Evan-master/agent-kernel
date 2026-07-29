#!/usr/bin/env ruby
# frozen_string_literal: true

require "socket"

EXPECTED = "AGENT-V30-UDP".b
PORT = 40_130

socket = UDPSocket.new(Socket::AF_INET)
socket.bind("127.0.0.1", PORT)
$stdout.sync = true
puts "ready"

loop do
  payload, sender = socket.recvfrom(2048)
  next unless payload == EXPECTED

  socket.send(payload, 0, sender.fetch(3), sender.fetch(1))
end
