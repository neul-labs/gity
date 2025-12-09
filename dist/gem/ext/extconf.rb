# frozen_string_literal: true

# This is a dummy extconf.rb that triggers binary download during gem install
require "mkmf"
require_relative "../lib/gity"

# Download the binary during installation
Gity.ensure_binary

# Create a dummy Makefile
File.write("Makefile", "install:\n\t@echo 'Binary already installed'\n")
