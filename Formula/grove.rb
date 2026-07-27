class Grove < Formula
  desc "Multi-project git worktree workspace manager"
  homepage "https://github.com/yauyy/grove"
  version "0.8.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-aarch64-apple-darwin.tar.gz"
      sha256 "dcbae4d1c94ce9071f472c05b94a8a2c3b2064b770ecbe5415e693489707e55a"
    else
      url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-apple-darwin.tar.gz"
      sha256 "629c4da60269616c4ea201a395d792ed38c7b7e8bdbda3735f7b80057ed26e48"
    end
  end

  on_linux do
    url "https://github.com/yauyy/grove/releases/download/v#{version}/grove-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "d776df1535eb01a9e8801692020ac2bb9413f04b7321d88effdd943f02faef4c"
  end

  def install
    bin.install "grove"
  end

  test do
    assert_match "Multi-project git worktree workspace manager", shell_output("#{bin}/grove --help")
  end
end
