class Grove < Formula
  desc "Multi-project git worktree workspace manager"
  homepage "https://github.com/yauyy/grove"
  version "0.7.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-aarch64-apple-darwin.tar.gz"
      sha256 "77512bdd2e2486cc490e4647abf3c509e5667f00fbdb618d8e14d811d4bbe04a"
    else
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-apple-darwin.tar.gz"
      sha256 "b72f89fd2f0346464571676a95dd8320a8e4916950e390e6c30a062254a440f8"
    end
  end

  on_linux do
    url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "b8f4579eae19699960a019f2697786da6d015257ecfecee2c11d636c7724c65d"
  end

  def install
    bin.install "grove"
  end

  test do
    assert_match "Multi-project git worktree workspace manager", shell_output("#{bin}/grove --help")
  end
end
