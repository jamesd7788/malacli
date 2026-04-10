class Malacli < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/malacli"
  version "0.4.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-apple-darwin.tar.gz"
      sha256 "f0745c3dd7d7627e6e6ea4d1da9847474936b75ebcc0ce333800bdd6d31c43ef"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-apple-darwin.tar.gz"
      sha256 "9a7a09fef4d910ba1b0bb097d5dc97b4bb10b2223bd21e408614c1d889340042"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "d13065375d9b5f530058db9793adf1525e6f120c6d3ff9096f6d9ae2ac0ead27"
    end

    on_intel do
      url "https://github.com/jamesd7788/malacli/releases/download/v#{version}/malacli-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6917130f335f9d5329ce963f13aad2972eba84a80bb43f215b3db1f18c527e36"
    end
  end

  def install
    bin.install "malacli"
  end

  test do
    assert_match "malacli", shell_output("#{bin}/malacli --version")
  end
end
