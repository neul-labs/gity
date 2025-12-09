# frozen_string_literal: true

require "os"
require "open-uri"
require "fileutils"
require "rubygems/package"
require "zlib"

module Gity
  VERSION = "0.1.0"
  REPO = "neul-labs/gity"

  class << self
    def platform_target
      case [OS.host_os, OS.host_cpu]
      when ["darwin", "x86_64"]
        "x86_64-apple-darwin"
      when ["darwin", "arm64"]
        "aarch64-apple-darwin"
      when ["linux", "x86_64"]
        "x86_64-unknown-linux-gnu"
      when ["linux", "aarch64"]
        "aarch64-unknown-linux-gnu"
      when ["mingw32", "x86_64"], ["mswin64", "x86_64"]
        "x86_64-pc-windows-msvc"
      else
        raise "Unsupported platform: #{OS.host_os}-#{OS.host_cpu}"
      end
    end

    def binary_dir
      File.join(File.dirname(__FILE__), "..", "bin")
    end

    def binary_path
      binary_name = OS.windows? ? "gity.exe" : "gity"
      File.join(binary_dir, binary_name)
    end

    def ensure_binary
      return binary_path if File.exist?(binary_path)

      target = platform_target
      ext = OS.windows? ? "zip" : "tar.gz"
      url = "https://github.com/#{REPO}/releases/download/v#{VERSION}/gity-#{VERSION}-#{target}.#{ext}"

      puts "Downloading gity #{VERSION} for #{target}..."

      FileUtils.mkdir_p(binary_dir)

      Dir.mktmpdir do |tmpdir|
        archive_path = File.join(tmpdir, "gity.#{ext}")

        URI.open(url) do |remote|
          File.open(archive_path, "wb") do |local|
            local.write(remote.read)
          end
        end

        if ext == "tar.gz"
          extract_tar_gz(archive_path, binary_dir)
        else
          extract_zip(archive_path, binary_dir)
        end
      end

      FileUtils.chmod(0o755, binary_path) unless OS.windows?

      puts "gity installed successfully!"
      binary_path
    end

    private

    def extract_tar_gz(archive_path, dest_dir)
      Gem::Package::TarReader.new(Zlib::GzipReader.open(archive_path)) do |tar|
        tar.each do |entry|
          dest_path = File.join(dest_dir, entry.full_name)
          if entry.file?
            File.open(dest_path, "wb") do |f|
              f.write(entry.read)
            end
          end
        end
      end
    end

    def extract_zip(archive_path, dest_dir)
      system("unzip", "-o", archive_path, "-d", dest_dir)
    end
  end
end
