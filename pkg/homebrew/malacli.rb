class Malacli < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/malacli"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-apple-darwin.tar.gz"
      sha256 "4bba11afb056d8371d1d1a465fe095a7c5a6a620509819236c51ee26656cd5cc"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-apple-darwin.tar.gz"
      sha256 "6f291527ef2ad46a4a18b3afa886f1e7eff4bf09d022a83e0f4544fb94589e96"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "1c33f08248ba70541d5ce701176465db901a52929184e3a9b5a595c4bcf83555"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "024cfdd74ec2af622db0cdd2032067c7e57648868a0127987dcc67a0458d6f89"
    end
  end

  def install
    bin.install "malacli"
  end

  test do
    assert_match "malacli", shell_output("#{bin}/malacli --version")
  end
end
