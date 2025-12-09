class Gity < Formula
  desc "Make large Git repositories feel instant"
  homepage "https://github.com/neul-labs/gity"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/neul-labs/gity/releases/download/v#{version}/gity-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/gity/releases/download/v#{version}/gity-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_X64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/neul-labs/gity/releases/download/v#{version}/gity-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/gity/releases/download/v#{version}/gity-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X64"
    end
  end

  def install
    bin.install "gity"
  end

  def caveats
    <<~EOS
      To accelerate a Git repository:
        gity register /path/to/repo

      The daemon will start automatically when needed.
      For manual control:
        gity daemon start
        gity daemon stop
    EOS
  end

  test do
    assert_match "gity #{version}", shell_output("#{bin}/gity --version")
  end
end
