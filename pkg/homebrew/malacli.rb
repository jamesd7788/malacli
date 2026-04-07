class Malacli < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/malacli"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "malacli"
  end

  test do
    assert_match "malacli", shell_output("#{bin}/malacli --version")
  end
end
