class TuiBible < Formula
  desc "A fast, keyboard-first terminal Bible reader"
  homepage "https://github.com/jamesd7788/tui-bible"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-apple-darwin.tar.gz"
      sha256 "00c6ba7ddb5d228a015b2922987b6c6a36ceeb5329414439f51ab51ef685c5aa"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-apple-darwin.tar.gz"
      sha256 "a66bf32518de177040dec2a216b5ec68669a8cb354d63d420f66a07a2dd827e3"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f86bc59548fd8938abf2f4ae3cb7f83c5faacc4264cf7843e11ea1dbf91a076e"
    end

    on_intel do
      url "https://github.com/jamesd7788/tui-bible/releases/download/v#{version}/tui-bible-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "c82530d3392cd4bd0c4de1daf8c2515844647c1c43f4c6e4a739a13a490bd6a2"
    end
  end

  def install
    bin.install "tui-bible"
  end

  test do
    assert_match "tui-bible", shell_output("#{bin}/tui-bible --version")
  end
end
