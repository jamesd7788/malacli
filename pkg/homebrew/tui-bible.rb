class TuiBible < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/tui-bible"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "tui-bible"
  end

  test do
    assert_match "tui-bible", shell_output("#{bin}/tui-bible --version")
  end
end
