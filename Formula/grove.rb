class Grove < Formula
  desc "Multi-project git worktree workspace manager"
  homepage "https://github.com/yauyy/grove"
  version "0.4.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-aarch64-apple-darwin.tar.gz"
      sha256 "0a14b2ad52875482ec5eca80072c23cb202f2743b38854a780f5feadf57cb051"
    else
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-apple-darwin.tar.gz"
      sha256 "5e2c1c492059c54050565b815ca9f3815d176390e8dae5ed4a1b9f8896f35f60"
    end
  end

  on_linux do
    url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "3744c4299ac6c5a067a8b12aad13b36ca6698297c26a25c02876f0fea1dad07f"
  end

  def install
    bin.install "grove"
  end

  test do
    assert_match "Multi-project git worktree workspace manager", shell_output("#{bin}/grove --help")
  end
end
