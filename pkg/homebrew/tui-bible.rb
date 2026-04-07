class TuiBible < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/tui-bible"
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-apple-darwin.tar.gz"
      sha256 "acc3504aa5ff159b54da7fcc8b513b1d879cd9e45149c69bbe90a3fe5235a3db"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-apple-darwin.tar.gz"
      sha256 "b00f9e56e159521a4248e47d209720580d2d8a7498d09181321c533a33685861"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "52afc5d5b32b1f690eb63406c96c498144c6ca58cd5f23507484bc48844257b7"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "445f2eb8bf1eb4b9bbd2bc5bd8a3f098128cdfd78baf219a635f13113e312b73"
    end
  end

  def install
    bin.install "tui-bible"
  end

  test do
    assert_match "tui-bible", shell_output("#{bin}/tui-bible --version")
  end
end
